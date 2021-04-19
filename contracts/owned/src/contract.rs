use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, LogAttribute, Querier, StdError, StdResult, Storage,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{owner, owner_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        owner: deps.api.canonical_address(&env.message.sender)?,
        pending_owner: None,
    };

    owner(&mut deps.storage).save(&state)?;

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
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)),
    }
}

pub fn handle_transfer_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: CanonicalAddr,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let owner = owner_read(&deps.storage).load()?.owner;
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
    owner(&mut deps.storage).update(|mut state| {
        state.pending_owner = Some(to.clone());

        Ok(state)
    })?;

    Ok(vec![
        log("action", "ownership transferred"),
        log("pending_owner", deps.api.human_address(&to).unwrap()),
    ])
}

pub fn handle_accept_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let pending_owner = owner_read(&deps.storage).load()?.pending_owner;

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

    owner(&mut deps.storage).update(|mut state| {
        state.owner = sender.clone();
        state.pending_owner = None;

        Ok(state)
    })?;

    Ok(vec![
        log("action", "ownership accepted"),
        log("owner", deps.api.human_address(&sender).unwrap()),
    ])
}

pub fn get_owner<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<HumanAddr> {
    let owner = owner_read(&deps.storage).load()?.owner;

    deps.api.human_address(&owner)
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

        let sender = env.clone().message.sender;
        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let owner: StdResult<HumanAddr> = from_binary(&res).unwrap();
        assert_eq!(true, HumanAddr::eq(&sender, &owner.unwrap()));
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

        let _res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let new_pending_owner = owner_read(&deps.storage)
            .load()
            .unwrap()
            .pending_owner
            .unwrap();
        assert_eq!(true, CanonicalAddr::eq(&mock_addr, &new_pending_owner));
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

        let _res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let new_pending_owner = owner_read(&deps.storage).load().unwrap().pending_owner;
        assert_eq!(
            true,
            CanonicalAddr::eq(&mock_addr, &new_pending_owner.clone().unwrap())
        );

        let env = mock_env(MOCK_CONTRACT_ADDR, &coins(1000, "earth"));
        let msg = HandleMsg::AcceptOwnership {};
        let res = handle(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let _res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        assert_eq!(
            true,
            CanonicalAddr::eq(&mock_addr, &new_pending_owner.clone().unwrap())
        );
        let new_pending_owner = owner_read(&deps.storage).load().unwrap().pending_owner;
        assert_eq!(true, new_pending_owner.is_none());
    }
}
