use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdResult, Storage, Uint128, WasmMsg,
};

use crate::error::*;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

use flags::msg::HandleMsg as FlagsMsg;
use owned::contract::{get_owner, init as owned_init};

static THRESHOLD_MULTIPLIER: u128 = 100000;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let flags_addr = deps.api.canonical_address(&msg.flags)?;

    owned_init(deps, env, owned::msg::InitMsg {})?;

    let state = State {
        flags: flags_addr,
        flagging_threshold: msg.flagging_threshold,
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
        HandleMsg::SetFlagsAddress { flags } => handle_set_flags_address(deps, env, flags),
        HandleMsg::SetFlaggingThreshold { threshold } => {
            handle_set_flagging_threshold(deps, env, threshold)
        }
        HandleMsg::Validate {
            previous_round_id,
            previous_answer,
            round_id,
            answer,
        } => handle_validate(
            deps,
            env,
            previous_round_id,
            previous_answer,
            round_id,
            answer,
        ),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsValid {
            previous_answer,
            answer,
        } => to_binary(&is_valid(&deps, previous_answer, answer)),
        QueryMsg::GetFlaggingThreshold {} => to_binary(&query_flagging_threshold(&deps)),
    }
}

pub fn handle_validate<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _previous_round_id: Uint128,
    previous_answer: Uint128,
    _round_id: Uint128,
    answer: Uint128,
) -> StdResult<HandleResponse> {
    if !(is_valid(deps, previous_answer, answer)?) {
        let flags = config_read(&deps.storage).load()?.flags;
        let flags_addr = deps.api.human_address(&flags)?;
        let raise_flag_msg = WasmMsg::Execute {
            contract_addr: flags_addr,
            msg: to_binary(&FlagsMsg::RaiseFlag {
                subject: env.contract.address,
            })?,
            send: vec![],
        };
        Ok(HandleResponse {
            messages: vec![raise_flag_msg.into()],
            log: vec![log("action", "validate"), log("is valid", false)],
            data: Some(to_binary(&false)?),
        })
    } else {
        Ok(HandleResponse {
            messages: vec![],
            log: vec![log("action", "validate"), log("is valid", true)],
            data: Some(to_binary(&true)?),
        })
    }
}

pub fn handle_set_flags_address<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    flags: HumanAddr,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;
    let previous = config_read(&deps.storage).load()?.flags;
    let new_addr = deps.api.canonical_address(&flags)?;
    if previous != new_addr {
        let new_flags = deps.api.canonical_address(&flags)?;
        config(&mut deps.storage).update(|mut state| {
            state.flags = new_flags;
            Ok(state)
        })?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "flags address updated"),
            log("previous", deps.api.human_address(&previous)?),
            log("current", deps.api.human_address(&new_addr)?),
        ],
        data: None,
    })
}

pub fn handle_set_flagging_threshold<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    threshold: u32,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;
    let previous_ft = config_read(&deps.storage).load()?.flagging_threshold;

    if previous_ft != threshold {
        config(&mut deps.storage).update(|mut state| {
            state.flagging_threshold = threshold;
            Ok(state)
        })?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "flagging threshold updated"),
            log("previous", previous_ft),
            log("current", threshold),
        ],
        data: None,
    })
}

fn is_valid<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    previous_answer: Uint128,
    answer: Uint128,
) -> StdResult<bool> {
    if previous_answer == Uint128::zero() {
        Ok(true)
    } else {
        let flagging_threshold = config_read(&deps.storage).load()?.flagging_threshold;
        let change = (previous_answer - answer)?;
        let ratio_numerator = change.u128() * THRESHOLD_MULTIPLIER;
        let ratio = ratio_numerator / previous_answer.u128();
        Ok(ratio <= flagging_threshold as u128)
    }
}

pub fn query_flagging_threshold<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<u32> {
    let flagging_threshold = config_read(&deps.storage).load()?.flagging_threshold;
    Ok(flagging_threshold)
}

fn validate_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    let owner = get_owner(deps)?;
    if env.message.sender != owner {
        return ContractErr::NotOwner.std_err();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 100000,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn setting_flags_address() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 100000,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = HandleMsg::SetFlagsAddress {
            flags: HumanAddr::from("new_flags"),
        };
        let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        let flag_addr = deps
            .api
            .human_address(&config_read(&deps.storage).load().unwrap().flags)
            .unwrap();
        assert_eq!(HumanAddr::from("new_flags"), flag_addr);
    }

    #[test]
    fn setting_threshold() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 100000,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = HandleMsg::SetFlaggingThreshold { threshold: 1000 };
        let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        let threshold = &config_read(&deps.storage)
            .load()
            .unwrap()
            .flagging_threshold;
        assert_eq!(1000, *threshold);
    }

    #[test]
    fn is_valid_gives_right_response() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 80000,
        };

        let env = mock_env("creator", &coins(1000, "earth"));
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // this input should return false
        let msg = QueryMsg::IsValid {
            previous_answer: Uint128::from(100 as u64),
            answer: Uint128::from(5 as u64),
        };
        let res = query(&deps, msg).unwrap();
        let is_valid: StdResult<bool> = from_binary(&res).unwrap();
        assert_eq!(false, is_valid.unwrap());

        // this input should return true
        let msg = QueryMsg::IsValid {
            previous_answer: Uint128::from(3 as u64),
            answer: Uint128::from(1 as u64),
        };
        let res = query(&deps, msg).unwrap();
        let is_valid: StdResult<bool> = from_binary(&res).unwrap();
        assert_eq!(true, is_valid.unwrap());

        // should return true if previous_answer is 0
        let msg = QueryMsg::IsValid {
            previous_answer: Uint128::zero(),
            answer: Uint128::from(5 as u64),
        };
        let res = query(&deps, msg).unwrap();
        let is_valid: StdResult<bool> = from_binary(&res).unwrap();
        assert_eq!(true, is_valid.unwrap());
    }

    #[test]
    fn validate() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 80000,
        };

        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = HandleMsg::Validate {
            previous_round_id: Uint128::from(2 as u64),
            previous_answer: Uint128::from(3 as u64),
            answer: Uint128::from(1 as u64),
            round_id: Uint128::from(3 as u64),
        };

        // the case if validate is true
        let res = handle(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(
            vec![log("action", "validate"), log("is valid", true)],
            res.log
        );

        let msg = HandleMsg::Validate {
            previous_round_id: Uint128::from(2 as u64),
            previous_answer: Uint128::from(100 as u64),
            answer: Uint128::from(5 as u64),
            round_id: Uint128::from(3 as u64),
        };
        let res = handle(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(
            vec![log("action", "validate"), log("is valid", false)],
            res.log
        );
    }

    #[test]
    fn query_flagging_threshold() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 80000,
        };

        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = QueryMsg::GetFlaggingThreshold {};
        let res = query(&deps, msg).unwrap();
        let flagging_threshold: StdResult<u32> = from_binary(&res).unwrap();
        assert_eq!(80000 as u32, flagging_threshold.unwrap());
    }
}
