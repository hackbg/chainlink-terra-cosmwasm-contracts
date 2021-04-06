use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    QueryRequest, StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};
use cw20::BalanceResponse;
use link_token::msg::{HandleMsg as LinkMsg, QueryMsg as LinkQuery};

use crate::{error::*, msg::*, state::*};

static RESERVE_ROUNDS: u128 = 2;
static MAX_ORACLE_COUNT: u128 = 77;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.min_submission_value > msg.max_submission_value {
        return ContractErr::MinGreaterThanMax.std_err();
    }
    oracle_addresses(&mut deps.storage).save(&vec![])?;
    recorded_funds(&mut deps.storage).save(&Funds::default())?;

    let sender = deps.api.canonical_address(&env.message.sender)?;
    let link = deps.api.canonical_address(&msg.link)?;
    let validator = deps.api.canonical_address(&msg.validator)?;
    owner(&mut deps.storage).save(&sender)?;

    config(&mut deps.storage).save(&State::new(
        link,
        validator,
        msg.payment_amount,
        0,
        0,
        0,
        msg.timeout,
        msg.decimals,
        msg.description.clone(),
        msg.min_submission_value,
        msg.max_submission_value,
    ))?;

    rounds(&mut deps.storage).save(
        &0_u32.to_be_bytes(), // TODO: this should be improved
        &Round {
            answer: None,
            started_at: None,
            updated_at: None,
            answered_in_round: env.block.time as u32 - msg.timeout,
        },
    )?;
    latest_round_id(&mut deps.storage).save(&0)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Submit {
            round_id,
            submission,
        } => handle_submit(deps, env, round_id, submission),
        HandleMsg::ChangeOracles {
            removed: _,
            added: _,
            added_admins: _,
            min_submissions: _,
            max_submissions: _,
            restart_delay: _,
        } => todo!(),
        HandleMsg::WithdrawPayment {
            oracle,
            recipient,
            amount,
        } => handle_withdraw_payment(deps, env, oracle, recipient, amount),
        HandleMsg::WithdrawFunds { recipient, amount } => {
            handle_withdraw_funds(deps, env, recipient, amount)
        }
        HandleMsg::TransferAdmin { oracle, new_admin } => {
            handle_transfer_admin(deps, env, oracle, new_admin)
        }
        HandleMsg::AcceptAdmin { oracle } => handle_accept_admin(deps, env, oracle),
        HandleMsg::RequestNewRound {} => todo!(),
        HandleMsg::SetRequesterPermissions {
            requester: _,
            authorized: _,
            delay: _,
        } => todo!(),
        HandleMsg::UpdateFutureRounds {
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        } => handle_update_future_rounds(
            deps,
            env,
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        ),
        HandleMsg::UpdateAvailableFunds {} => handle_update_available_funds(deps, env),
        HandleMsg::SetValidator { validator: _ } => todo!(),
        HandleMsg::Receive(_) => todo!(),
    }
}

pub fn handle_submit<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _round_id: u32,
    _submission: Uint128,
) -> StdResult<HandleResponse> {
    todo!()
}

pub fn handle_change_oracles<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    removed: Vec<HumanAddr>,
    added: Vec<HumanAddr>,
    added_admins: Vec<HumanAddr>,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    for oracle in removed {
        todo!()
    }
    if added.len() != added_admins.len() {
        return ContractErr::OracleAdminCountMismatch.std_err();
    }

    todo!()
}

pub fn handle_transfer_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    oracle: HumanAddr,
    new_admin: HumanAddr,
) -> StdResult<HandleResponse> {
    let oracle_addr = deps.api.canonical_address(&oracle)?;
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let new_admin_addr = deps.api.canonical_address(&new_admin)?;

    oracles(&mut deps.storage).update(oracle_addr.as_slice(), |status| {
        let mut status = status.unwrap(); // TODO: improve handling
        if status.admin != sender_addr {
            return ContractErr::NotAdmin.std_err();
        }
        status.pending_admin = Some(new_admin_addr);

        Ok(status)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("oracle_admin_update_requested", oracle),
            log("sender", env.message.sender),
            log("new_admin", new_admin),
        ],
        data: None,
    })
}

pub fn handle_accept_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    oracle: HumanAddr,
) -> StdResult<HandleResponse> {
    let oracle_addr = deps.api.canonical_address(&oracle)?;
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;

    oracles(&mut deps.storage).update(oracle_addr.as_slice(), |status| {
        let mut status = status.unwrap(); // TODO: improve handling
        if let Some(pending_admin) = status.pending_admin {
            if pending_admin != sender_addr {
                return ContractErr::NotPendingAdmin.std_err();
            }
            status.admin = sender_addr;
            status.pending_admin = None;

            Ok(status)
        } else {
            ContractErr::PendingAdminMissing.std_err()
        }
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("oracle_admin_updated", oracle),
            log("new_admin", env.message.sender),
        ],
        data: None,
    })
}

pub fn handle_withdraw_payment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    oracle: HumanAddr,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let oracle = deps.api.canonical_address(&oracle)?;
    let oracle_status = oracles_read(&deps.storage).load(oracle.as_slice())?;

    if oracle_status.admin != sender {
        return ContractErr::NotAdmin.std_err();
    }
    if oracle_status.withdrawable < amount {
        return ContractErr::InsufficientWithdrawableFunds.std_err();
    }

    oracles(&mut deps.storage).update(oracle.as_slice(), |status| {
        let mut status = status.unwrap(); // TODO: might have to add Default
        let new_withdrawable = (status.withdrawable - amount)?;
        status.withdrawable = new_withdrawable;
        Ok(status)
    })?;
    recorded_funds(&mut deps.storage).update(|mut funds| {
        funds.allocated = (funds.allocated - amount)?;
        Ok(funds)
    })?;

    let link = config_read(&deps.storage).load()?.link;
    let link_addr = deps.api.human_address(&link)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link_addr,
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![transfer_msg.into()],
        log: vec![],
        data: None,
    })
}

pub fn handle_withdraw_funds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    let funds = recorded_funds_read(&deps.storage).load()?;
    let payment_amount = config_read(&deps.storage).load()?.payment_amount;
    let oracle_count = get_oracle_count(&deps)?;
    let available = (funds.available - required_reserve(payment_amount, oracle_count))?;

    if available < amount {
        return ContractErr::InsufficientReserveFunds.std_err();
    }

    let link = config_read(&deps.storage).load()?.link;
    let link_addr = deps.api.human_address(&link)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link_addr,
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };
    let update_funds_msg = WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&HandleMsg::UpdateAvailableFunds {})?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![transfer_msg.into(), update_funds_msg.into()],
        log: vec![],
        data: None,
    })
}

pub fn handle_update_available_funds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let link_addr = config_read(&deps.storage).load()?.link;
    let query = QueryRequest::<()>::Wasm(WasmQuery::Smart {
        contract_addr: deps.api.human_address(&link_addr)?,
        msg: to_binary(&LinkQuery::Balance {
            address: env.contract.address,
        })?,
    });
    let prev_available: BalanceResponse = deps.querier.custom_query(&query)?;

    let funds = recorded_funds_read(&deps.storage).load()?;
    let now_available = (prev_available.balance - funds.allocated)?;

    // Does this offer us benefits?
    if funds.available == now_available {
        return Ok(HandleResponse::default());
    }

    recorded_funds(&mut deps.storage).update(|mut funds| {
        funds.available = now_available;
        Ok(funds)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "update_available_funds"),
            log("amount", now_available),
        ],
        data: None,
    })
}

pub fn handle_update_future_rounds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    payment_amount: Uint128,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
    timeout: u32,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    let oracle_count = get_oracle_count(&deps)?;

    if min_submissions > max_submissions {
        return ContractErr::MinGreaterThanMax.std_err();
    }
    if (oracle_count as u32) < max_submissions {
        return ContractErr::MaxGreaterThanTotal.std_err();
    }
    if oracle_count != 0 && (oracle_count as u32) <= restart_delay {
        return ContractErr::DelayGreaterThanTotal.std_err();
    }
    let funds = recorded_funds_read(&deps.storage).load()?;
    if funds.available < required_reserve(payment_amount, oracle_count) {
        return ContractErr::InsufficientFunds.std_err();
    }
    if oracle_count > 0 && min_submissions == 0 {
        return ContractErr::MinLessThanZero.std_err();
    }

    config(&mut deps.storage).update(|mut state| {
        state.payment_amount = payment_amount;
        state.min_submission_count = min_submissions;
        state.max_submission_count = max_submissions;
        state.restart_delay = restart_delay;
        state.timeout = timeout;

        Ok(state)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "round_details_updated"),
            log("payment_amount", payment_amount),
            log("min_submissions", min_submissions),
            log("max_submissions", max_submissions),
            log("restart_delay", restart_delay),
            log("timeout", timeout),
        ],
        data: None,
    })
}

fn required_reserve(payment: Uint128, oracle_count: u8) -> Uint128 {
    Uint128(payment.u128() * oracle_count as u128 * RESERVE_ROUNDS)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAllocatedFunds {} => to_binary(&get_allocated_funds(deps)),
        QueryMsg::GetAvailableFunds {} => to_binary(&get_available_funds(deps)),
        QueryMsg::GetWithdrawablePayment { oracle } => {
            to_binary(&get_withdrawable_payment(deps, oracle))
        }
        QueryMsg::GetOracleCount {} => to_binary(&get_oracle_count(deps)),
        QueryMsg::GetOracles {} => to_binary(&get_oracles(deps)),
        QueryMsg::GetAdmin { oracle } => to_binary(&get_admin(deps, oracle)),
        QueryMsg::GetRoundData { round_id } => to_binary(&get_round_data(deps, round_id)),
        QueryMsg::GetLatestRoundData {} => to_binary(&get_latest_round_data(deps)),
        QueryMsg::GetOracleRoundState {
            oracle: _,
            queried_round_id: _,
        } => todo!(),
    }
}

pub fn get_allocated_funds<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Uint128> {
    Ok(recorded_funds_read(&deps.storage).load()?.allocated)
}

pub fn get_available_funds<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Uint128> {
    Ok(recorded_funds_read(&deps.storage).load()?.available)
}

pub fn get_withdrawable_payment<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle: HumanAddr,
) -> StdResult<Uint128> {
    let addr = deps.api.canonical_address(&oracle)?;
    let oracle = oracles_read(&deps.storage).load(addr.as_slice())?;
    Ok(oracle.withdrawable)
}

pub fn get_oracle_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<u8> {
    Ok(oracle_addresses_read(&deps.storage).load()?.len() as u8)
}

pub fn get_oracles<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Vec<HumanAddr>> {
    let addresses = oracle_addresses_read(&deps.storage).load()?;
    let human_addresses = addresses
        .iter()
        .map(|addr| deps.api.human_address(addr))
        .collect::<StdResult<Vec<HumanAddr>>>()?;
    Ok(human_addresses)
}

pub fn get_admin<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle: HumanAddr,
) -> StdResult<HumanAddr> {
    let addr = deps.api.canonical_address(&oracle)?;
    let oracle = oracles_read(&deps.storage).load(addr.as_slice())?;

    deps.api.human_address(&oracle.admin)
}

pub fn get_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    round_id: u32,
) -> StdResult<RoundDataResponse> {
    let round = rounds_read(&deps.storage).load(&round_id.to_be_bytes())?;
    if round.answered_in_round == 0 {
        return ContractErr::NoData.std_err();
    }
    Ok(RoundDataResponse {
        round_id,
        answer: round
            .answer
            .ok_or_else(|| StdError::generic_err("'answer' unset"))?,
        started_at: round
            .started_at
            .ok_or_else(|| StdError::generic_err("'started_at' unset"))?,
        updated_at: round
            .updated_at
            .ok_or_else(|| StdError::generic_err("'updated_at' unset"))?,
        answered_in_round: round.answered_in_round,
    })
}

pub fn get_latest_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<RoundDataResponse> {
    let round_id = latest_round_id_read(&deps.storage).load()?;

    get_round_data(deps, round_id)
}

fn validate_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let owner = owner_read(&deps.storage).load()?;
    if sender != owner {
        return ContractErr::NotOwner.std_err();
    }
    Ok(())
}
