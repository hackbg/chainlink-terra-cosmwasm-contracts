use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    LogAttribute, Querier, StdError, StdResult, Storage, Uint128,
};

use crate::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::{
        config, config_read, oracle_addresses_read, recorded_funds_read, rounds, Round, State,
    },
};

static RESERVE_ROUNDS: u128 = 2;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let link = deps.api.canonical_address(&msg.link)?;
    let validator = deps.api.canonical_address(&msg.validator)?;
    let logs = update_future_rounds(deps, msg.payment_amount, 0, 0, 0, msg.timeout)?;

    config(&mut deps.storage).update(|state| {
        Ok(State::new(
            sender,
            link,
            validator,
            state.payment_amount,
            state.max_submission_count,
            state.min_submission_count,
            state.restart_delay,
            state.timeout,
            msg.decimals,
            msg.description.clone(),
            msg.min_submission_value,
            msg.max_submission_value,
        ))
    })?;

    rounds(&mut deps.storage).save(
        &0_u64.to_be_bytes(),
        &Round {
            answer: None,
            started_at: None,
            updated_at: None,
            answered_in_round: env.block.time - msg.timeout as u64,
        },
    )?;

    Ok(InitResponse {
        messages: vec![],
        log: logs,
    })
}

fn update_future_rounds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    payment_amount: Uint128,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
    timeout: u32,
) -> StdResult<Vec<LogAttribute>> {
    let oracle_count = oracle_addresses_read(&deps.storage).load()?.len();

    if min_submissions > max_submissions {
        return Err(StdError::generic_err("Min cannot be greater than max"));
    }

    if oracle_count < max_submissions as usize {
        return Err(StdError::generic_err("Max cannot exceed total"));
    }

    if oracle_count != 0 && oracle_count <= restart_delay as usize {
        return Err(StdError::generic_err("Delay cannot exceed total"));
    }

    let funds = recorded_funds_read(&deps.storage).load()?;
    if funds.available < required_reserve(payment_amount, oracle_count as u128) {
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

    Ok(vec![
        log("action", "round_details_updated"),
        log("payment_amount", payment_amount),
        log("min_submissions", min_submissions),
        log("max_submissions", max_submissions),
        log("restart_delay", restart_delay),
        log("timeout", timeout),
    ])
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
        HandleMsg::WithdrawFunds {
            recipient: _,
            amount: _,
        } => todo!(),
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
        HandleMsg::UpdateAvailableFunds {} => todo!(),
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
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _oracle: HumanAddr,
    _recipient: HumanAddr,
    _amount: Uint128,
) -> StdResult<HandleResponse> {
    todo!()
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
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let owner = config_read(&deps.storage).load()?.owner;
    if sender != *owner {
        return Err(StdError::generic_err("Only callable by owner"));
    }

    let logs = update_future_rounds(
        deps,
        payment_amount,
        min_submissions,
        max_submissions,
        restart_delay,
        timeout,
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: logs,
        data: None,
    })
}

fn required_reserve(payment: Uint128, oracle_count: u128) -> Uint128 {
    Uint128(payment.u128() * oracle_count * RESERVE_ROUNDS)
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
