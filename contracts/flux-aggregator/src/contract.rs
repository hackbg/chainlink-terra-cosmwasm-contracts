use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    QueryRequest, StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};
use cw20::BalanceResponse;
use link_token::msg::{HandleMsg as LinkMsg, QueryMsg as LinkQuery};

use crate::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::{
        config, config_read, oracle_addresses, oracle_addresses_read, oracles, oracles_read, owner,
        owner_read, recorded_funds, recorded_funds_read, rounds, Funds, Round, State,
    },
};

static RESERVE_ROUNDS: u128 = 2;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.min_submission_value > msg.max_submission_value {
        return Err(StdError::generic_err("Min cannot be greater than max"));
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
        &0_u64.to_be_bytes(), // TODO: this should be improved
        &Round {
            answer: None,
            started_at: None,
            updated_at: None,
            answered_in_round: env.block.time - msg.timeout as u64,
        },
    )?;

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
        HandleMsg::TransferAdmin {
            oracle: _,
            new_admin: _,
        } => todo!(),
        HandleMsg::AcceptAdmin { oracle: _ } => todo!(),
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
    _round_id: Uint128,
    _submission: Uint128,
) -> StdResult<HandleResponse> {
    todo!()
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
        return Err(StdError::generic_err("Only callable by admin"));
    }
    if oracle_status.withdrawable < amount {
        return Err(StdError::generic_err("Insufficient withdrawable funds"));
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
        return Err(StdError::generic_err("Insufficient reserve funds"));
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
        return Err(StdError::generic_err("Min cannot be greater than max"));
    }
    if (oracle_count as u32) < max_submissions {
        return Err(StdError::generic_err("Max cannot exceed total"));
    }
    if oracle_count != 0 && (oracle_count as u32) <= restart_delay {
        return Err(StdError::generic_err("Delay cannot exceed total"));
    }
    let funds = recorded_funds_read(&deps.storage).load()?;
    if funds.available < required_reserve(payment_amount, oracle_count) {
        return Err(StdError::generic_err("Insufficient funds for payment"));
    }
    if oracle_count > 0 && min_submissions == 0 {
        return Err(StdError::generic_err("Min must be greater than 0"));
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
        QueryMsg::GetWithdrawablePayment { oracle: _ } => todo!(),
        QueryMsg::GetOracleCount {} => to_binary(&get_oracle_count(deps)),
        QueryMsg::GetOracles {} => to_binary(&get_oracles(deps)),
        QueryMsg::GetAdmin { oracle: _ } => todo!(),
        QueryMsg::GetRoundData { round_id: _ } => todo!(),
        QueryMsg::GetLatestRoundData {} => todo!(),
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

fn validate_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let owner = owner_read(&deps.storage).load()?;
    if sender != owner {
        return Err(StdError::generic_err("Only callable by owner"));
    }
    Ok(())
}
