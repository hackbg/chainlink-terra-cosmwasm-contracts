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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, HumanAddr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(1000, "earth"));

        let sender = deps.api.canonical_address(&env.message.sender).unwrap();
        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(true, CanonicalAddr::eq(&sender, &value.owner));
    }

    #[test]
    fn transfer_ownership() {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("creator", &coins(1000, "earth"));

        let msg = InitMsg {};

        let res: InitResponse = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let mock_addr = deps
            .api
            .canonical_address(&HumanAddr::from(MOCK_CONTRACT_ADDR))
            .unwrap();

        let msg = HandleMsg::TransferOwnership {
            to: mock_addr.clone(),
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            true,
            CanonicalAddr::eq(&mock_addr, &value.pending_owner.unwrap())
        );
    }

    #[test]
    fn accept_ownership() {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("creator", &coins(1000, "earth"));

        let msg = InitMsg {};

        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let mock_addr = deps
            .api
            .canonical_address(&HumanAddr::from(MOCK_CONTRACT_ADDR))
            .unwrap();

        let msg = HandleMsg::TransferOwnership {
            to: mock_addr.clone(),
        };

        let res = handle(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            true,
            CanonicalAddr::eq(&mock_addr, &value.pending_owner.unwrap())
        );

        let env = mock_env(MOCK_CONTRACT_ADDR, &coins(1000, "earth"));
        let msg = HandleMsg::AcceptOwnership {};
        let res = handle(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(true, CanonicalAddr::eq(&mock_addr, &value.owner));
        assert_eq!(true, value.pending_owner.is_none());
    }
}
