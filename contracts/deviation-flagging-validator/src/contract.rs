use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;

use flags::msg::ExecuteMsg as FlagsMsg;
use owned::contract::{
    execute_accept_ownership, execute_transfer_ownership, get_owner, instantiate as owned_init,
};

static THRESHOLD_MULTIPLIER: u128 = 100000;

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let flags = deps.api.addr_validate(&msg.flags)?;
    let state = State {
        flags,
        flagging_threshold: msg.flagging_threshold,
    };

    CONFIG.save(deps.storage, &state)?;
    owned_init(deps, env, info, owned::msg::InstantiateMsg {})?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetFlagsAddress { flags } => execute_set_flags_address(deps, env, info, flags),
        ExecuteMsg::SetFlaggingThreshold { threshold } => {
            execute_set_flagging_threshold(deps, env, info, threshold)
        }
        ExecuteMsg::Validate {
            previous_round_id,
            previous_answer,
            round_id,
            answer,
        } => execute_validate(
            deps,
            env,
            info,
            previous_round_id,
            previous_answer,
            round_id,
            answer,
        ),
        ExecuteMsg::TransferOwnership { to } => {
            execute_transfer_ownership(deps, env, info, to).map_err(ContractError::from)
        }
        ExecuteMsg::AcceptOwnership {} => {
            execute_accept_ownership(deps, env, info).map_err(ContractError::from)
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsValid {
            previous_answer,
            answer,
        } => to_binary(&is_valid(deps, previous_answer, answer)?),
        QueryMsg::GetFlaggingThreshold {} => to_binary(&query_flagging_threshold(deps)?),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)?),
    }
}

pub fn execute_validate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _previous_round_id: u32,
    previous_answer: Uint128,
    _round_id: u32,
    answer: Uint128,
) -> Result<Response, ContractError> {
    if !(is_valid(deps.as_ref(), previous_answer, answer)?) {
        let flags = CONFIG.load(deps.storage)?.flags;
        let raise_flag_msg = WasmMsg::Execute {
            contract_addr: String::from(flags),
            msg: to_binary(&FlagsMsg::RaiseFlag {
                subject: info.sender.to_string(),
            })?,
            funds: vec![],
        };
        Ok(Response::new()
            .add_message(raise_flag_msg)
            .add_attribute("action", "validate")
            .add_attribute("is_valid", false.to_string())
            .set_data(to_binary(&false)?))
    } else {
        Ok(Response::new()
            .add_attribute("action", "validate")
            .add_attribute("is_valid", true.to_string())
            .set_data(to_binary(&true)?))
    }
}

pub fn execute_set_flags_address(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    flags: Addr,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &env, info)?;
    let previous = CONFIG.load(deps.storage)?.flags;
    if previous != flags {
        CONFIG.update(deps.storage, |mut state| -> StdResult<_> {
            state.flags = flags.clone();
            Ok(state)
        })?;
    }

    Ok(Response::new()
        .add_attribute("action", "flags_address_updated")
        .add_attribute("previous", previous))
}

pub fn execute_set_flagging_threshold(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    threshold: u32,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &env, info)?;
    let previous_ft = CONFIG.load(deps.storage)?.flagging_threshold;

    if previous_ft != threshold {
        CONFIG.update(deps.storage, |mut state| -> StdResult<_> {
            state.flagging_threshold = threshold;
            Ok(state)
        })?;
    }

    Ok(Response::new()
        .add_attribute("action", "flagging_threshold_updated")
        .add_attribute("previous", previous_ft.to_string())
        .add_attribute("current", threshold.to_string()))
}

fn is_valid(deps: Deps, previous_answer: Uint128, answer: Uint128) -> StdResult<bool> {
    if previous_answer == Uint128::zero() {
        Ok(true)
    } else {
        let flagging_threshold = CONFIG.load(deps.storage)?.flagging_threshold;
        let change = if previous_answer.u128() > answer.u128() {
            previous_answer.u128() - answer.u128()
        } else {
            answer.u128() - previous_answer.u128()
        };
        //Uint128::from(previous_answer.u128() - answer.u128());
        let ratio_numerator = change * THRESHOLD_MULTIPLIER;
        let ratio = ratio_numerator / previous_answer.u128();
        Ok(ratio <= flagging_threshold as u128)
    }
}

pub fn query_flagging_threshold(deps: Deps) -> StdResult<FlaggingThresholdResponse> {
    let flagging_threshold = CONFIG.load(deps.storage)?.flagging_threshold;
    Ok(FlaggingThresholdResponse {
        threshold: flagging_threshold,
    })
}

fn validate_ownership(deps: Deps, _env: &Env, info: MessageInfo) -> Result<(), ContractError> {
    let owner = get_owner(deps)?;
    if info.sender != owner {
        return Err(ContractError::NotOwner {});
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coins, Api};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            flags: "flags".to_string(),
            flagging_threshold: 100000,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn setting_flags_address() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            flags: "flags".to_string(),
            flagging_threshold: 100000,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let new_flags = deps.api.addr_validate("new_flags").unwrap();
        let res = execute_set_flags_address(deps.as_mut(), mock_env(), info, new_flags.clone());
        assert_eq!(0, res.unwrap().messages.len());

        let flag_addr = CONFIG.load(&deps.storage).unwrap().flags;
        assert_eq!(new_flags, flag_addr);
    }

    #[test]
    fn setting_threshold() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            flags: "flags".to_string(),
            flagging_threshold: 100000,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let _threshold =
            execute_set_flagging_threshold(deps.as_mut(), mock_env(), info, 1000).unwrap();

        let threshold = CONFIG.load(&deps.storage).unwrap().flagging_threshold;
        assert_eq!(1000, threshold);
    }

    #[test]
    fn is_valid_gives_right_response() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            flags: "flags".to_string(),
            flagging_threshold: 80000,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let previous_answer = Uint128::from(100 as u64);
        let answer = Uint128::from(5 as u64);
        let check_valid = is_valid(deps.as_ref(), previous_answer, answer).unwrap();
        assert_eq!(false, check_valid);

        // this input should return true
        let previous_answer = Uint128::from(3 as u64);
        let answer = Uint128::from(1 as u64);
        let check_valid = is_valid(deps.as_ref(), previous_answer, answer).unwrap();
        assert_eq!(true, check_valid);

        // should return true if previous_answer is 0
        let previous_answer = Uint128::zero();
        let answer = Uint128::from(5 as u64);
        let check_valid = is_valid(deps.as_ref(), previous_answer, answer).unwrap();
        assert_eq!(true, check_valid);
    }

    #[test]
    fn validate() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            flags: "flags".to_string(),
            flagging_threshold: 80000,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::Validate {
            previous_round_id: 2,
            previous_answer: Uint128::from(3 as u64),
            answer: Uint128::from(1 as u64),
            round_id: 3,
        };

        // the case if validate is true
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(
            vec![
                attr("action", "validate"),
                attr("is_valid", true.to_string())
            ],
            res.attributes
        );

        let msg = ExecuteMsg::Validate {
            previous_round_id: 2,
            previous_answer: Uint128::from(100 as u64),
            answer: Uint128::from(5 as u64),
            round_id: 3,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            vec![
                attr("action", "validate"),
                attr("is_valid", false.to_string())
            ],
            res.attributes
        );
    }

    #[test]
    fn test_query_flagging_threshold() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            flags: "flags".to_string(),
            flagging_threshold: 80000,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let flagging_threshold: u32 = query_flagging_threshold(deps.as_ref()).unwrap().threshold;
        assert_eq!(80000 as u32, flagging_threshold);
    }
}
