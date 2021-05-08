use std::thread::AccessError;

use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{config, config_read, State};
use crate::{error::ContractError, state::ACCESS_LIST};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        check_enabled: true,
    };
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAccess { user } => todo!(),
        ExecuteMsg::RemoveAccess { user } => todo!(),
        ExecuteMsg::EnableAccessCheck {} => todo!(),
        ExecuteMsg::DisableAccessCheck {} => todo!(),
    }
}

pub fn try_add_access(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user: Addr,
) -> StdResult<Response> {
    let may_have_access = ACCESS_LIST.may_load(deps.storage, &user)?;
    if may_have_access.is_none() {
        ACCESS_LIST.save(deps.storage, &user, &true)?;
    };
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn try_remove_access(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user: Addr,
) -> StdResult<Response> {
    let may_have_access = ACCESS_LIST.may_load(deps.storage, &user)?;
    if may_have_access.is_some() {
        ACCESS_LIST.update(deps.storage, &user, |_user: Option<bool>| -> StdResult<_> {
            Ok(false)
        })?;
    };
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn try_enable_access_check(deps: DepsMut, _env: Env, info: MessageInfo) -> StdResult<Response> {
    let check = config_read(deps.storage).load()?.check_enabled;
    if !check {
        config(deps.storage).update(|state| -> StdResult<_> {
            Ok(State {
                check_enabled: true,
            })
        })?;
    }
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn try_disable_access_check(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> StdResult<Response> {
    let check = config_read(deps.storage).load()?.check_enabled;
    if check {
        config(deps.storage).update(|state| -> StdResult<_> {
            Ok(State {
                check_enabled: false,
            })
        })?;
    }
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::HasAccess { user } => to_binary(&has_access(deps, env)?),
    }
}

pub fn has_access(deps: Deps, env: Env) -> StdResult<bool> {
    let access = config_read(deps.storage).load()?.check_enabled;

    Ok(access)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
