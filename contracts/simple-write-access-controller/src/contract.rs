use std::thread::AccessError;

use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, attr
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
    _info: MessageInfo,
    _msg: InstantiateMsg,
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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAccess { user } => Ok(try_add_access(deps, env, info, user)?),
        ExecuteMsg::RemoveAccess { user } => Ok(try_remove_access(deps, env, info, user)?),
        ExecuteMsg::EnableAccessCheck {} => Ok(try_enable_access_check(deps, env, info)?),
        ExecuteMsg::DisableAccessCheck {} => Ok(try_disable_access_check(deps,env, info)?),
    }
}

pub fn try_add_access(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    user: Addr,
) -> StdResult<Response> {
    let may_have_access = ACCESS_LIST.may_load(deps.storage, &user)?;
    if may_have_access.is_none() {
        ACCESS_LIST.save(deps.storage, &user, &true)?;
    };
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "added access"),
            attr("user", user)
        ],
        data: None,
    })
}

pub fn try_remove_access(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
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
        attributes: vec![
            attr("action", "removed access"),
            attr("user", user)
        ],
        data: None,
    })
}

pub fn try_enable_access_check(deps: DepsMut, _env: Env, _info: MessageInfo) -> StdResult<Response> {
    let check = config_read(deps.storage).load()?.check_enabled;
    if !check {
        config(deps.storage).update(|_state| -> StdResult<_> {
            Ok(State {
                check_enabled: true,
            })
        })?;
    }
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "enable access check"),
        ],
        data: None,
    })
}

pub fn try_disable_access_check(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> StdResult<Response> {
    let check = config_read(deps.storage).load()?.check_enabled;
    if check {
        config(deps.storage).update(|_state| -> StdResult<_> {
            Ok(State {
                check_enabled: false,
            })
        })?;
    }
    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "disable access check")
        ],
        data: None,
    })
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::HasAccess { user } => to_binary(&has_access(deps, env, user)?),
        QueryMsg::GetCheckEnabled {} => to_binary(&query_check_enabled(deps, env)?),
    }
}

pub fn has_access(deps: Deps, _env: Env, user: Addr) -> StdResult<bool> {
    let access = config_read(deps.storage).load()?.check_enabled;
    let user = ACCESS_LIST.load(deps.storage, &user)?;

    Ok(user || !access)
}

pub fn query_check_enabled(deps: Deps, _env: Env) -> StdResult<bool> {
    let check_enabled = config_read(deps.storage).load()?.check_enabled;

    Ok(check_enabled)
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