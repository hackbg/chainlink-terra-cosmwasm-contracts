use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response, StdResult,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{owner, owner_read, State};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender,
        pending_owner: None,
    };

    owner(deps.storage).save(&state)?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::TransferOwnership { to } => handle_transfer_ownership(deps, env, info, to),
        ExecuteMsg::AcceptOwnership {} => handle_accept_ownership(deps, env, info),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)?),
    }
}

pub fn handle_transfer_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: Addr,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    let owner = owner_read(deps.storage).load()?.owner;
    if sender != owner {
        return Err(ContractError::OnlyOwner {});
    }

    let attributes = transfer_ownership(deps, env, to)?;

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes,
        data: None,
    })
}

fn transfer_ownership(deps: DepsMut, _env: Env, to: Addr) -> Result<Vec<Attribute>, ContractError> {
    owner(deps.storage).update(|mut state| -> StdResult<_> {
        state.pending_owner = Some(to.clone());

        Ok(state)
    })?;

    Ok(vec![
        attr("action", "ownership transferred"),
        attr("pending_owner", to),
    ])
}

pub fn handle_accept_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = info.clone().sender;
    let pending_owner = owner_read(deps.storage).load()?.pending_owner;

    if sender != pending_owner.unwrap() {
        return Err(ContractError::MustBeProposed {});
    }

    let logs = accept_ownership(deps, env, info)?;

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: logs,
        data: None,
    })
}

fn accept_ownership(deps: DepsMut, _env: Env, info: MessageInfo) -> StdResult<Vec<Attribute>> {
    let sender = info.sender;

    owner(deps.storage).update(|mut state| -> StdResult<_> {
        state.owner = sender.clone();
        state.pending_owner = None;

        Ok(state)
    })?;

    Ok(vec![
        attr("action", "ownership accepted"),
        attr("owner", sender),
    ])
}

pub fn get_owner(deps: Deps) -> StdResult<Addr> {
    let owner = owner_read(deps.storage).load()?.owner;

    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        let sender = info.clone().sender;
        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = get_owner(deps.as_ref()).unwrap();
        assert_eq!(String::from(sender), String::from(res));
    }

    #[test]
    fn transfer_ownership() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let mock_addr = String::from(MOCK_CONTRACT_ADDR);

        let res = handle_transfer_ownership(
            deps.as_mut(),
            mock_env(),
            info,
            Addr::unchecked(mock_addr.clone()),
        )
        .unwrap();
        assert_eq!(0, res.messages.len());

        let res = owner_read(&deps.storage)
            .load()
            .unwrap()
            .pending_owner
            .unwrap();

        assert_eq!(mock_addr, String::from(res));
    }

    #[test]
    fn accept_ownership() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let mock_addr = String::from(MOCK_CONTRACT_ADDR);

        let res = handle_transfer_ownership(
            deps.as_mut(),
            mock_env(),
            info,
            Addr::unchecked(mock_addr.clone()),
        )
        .unwrap();
        assert_eq!(0, res.messages.len());

        let res = owner_read(deps.as_ref().storage)
            .load()
            .unwrap()
            .pending_owner
            .unwrap();

        assert_eq!(mock_addr, String::from(res));
        let info = mock_info(MOCK_CONTRACT_ADDR, &coins(1000, "earth"));
        let msg = ExecuteMsg::AcceptOwnership {};
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = owner_read(deps.as_ref().storage).load().unwrap().owner;
        assert_eq!(mock_addr, String::from(res));
        let new_pending_owner = owner_read(&deps.storage).load().unwrap().pending_owner;
        assert_eq!(true, new_pending_owner.is_none());
    }
}
