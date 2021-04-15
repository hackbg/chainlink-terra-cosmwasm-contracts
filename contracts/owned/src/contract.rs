use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, InitResponse,
    LogAttribute, Querier, StdError, StdResult, Storage,
};

use crate::msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        owner: deps.api.canonical_address(&env.message.sender)?,
        pending_owner: None,
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::TransferOwnership { to } => handle_transfer_ownership(deps, env, to),
        HandleMsg::AcceptOwnership {} => handle_accept_ownership(deps, env),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {} => get_owner(deps),
    }
}

pub fn handle_transfer_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: CanonicalAddr,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let owner = config_read(&deps.storage).load()?.owner;
    if sender != owner {
        return Err(StdError::generic_err("Only callable by owner"));
    }

    let logs = transfer_ownership(deps, env, to)?;

    Ok(HandleResponse {
        messages: vec![],
        log: logs,
        data: None,
    })
}

fn transfer_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    to: CanonicalAddr,
) -> StdResult<Vec<LogAttribute>> {
    config(&mut deps.storage).update(|mut state| {
        state.pending_owner = Some(to.clone());

        Ok(state)
    })?;

    Ok(vec![
        log("action", "ownership transferred"),
        log("pending_owner", to),
    ])
}

fn handle_accept_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let pending_owner = config_read(&deps.storage).load()?.pending_owner;

    if sender != pending_owner.unwrap() {
        return Err(StdError::generic_err("Must be proposed owner"));
    }

    let logs = accept_ownership(deps, env)?;

    Ok(HandleResponse {
        messages: vec![],
        log: logs,
        data: None,
    })
}

fn accept_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<Vec<LogAttribute>> {
    let sender = deps.api.canonical_address(&env.message.sender)?;

    config(&mut deps.storage).update(|mut state| {
        state.owner = sender.clone();
        state.pending_owner = None;

        Ok(state)
    })?;

    Ok(vec![
        log("action", "ownership accepted"),
        log("owner", sender),
    ])
}

fn get_owner<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Binary> {
    let state = config_read(&deps.storage).load()?;

    let resp = ConfigResponse {
        owner: state.owner,
        pending_owner: state.pending_owner,
    };
    to_binary(&resp)
}

