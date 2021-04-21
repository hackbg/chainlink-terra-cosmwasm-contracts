use crate::{error::*, msg::*, state::*};
use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};
use owned::contract::{get_owner, init as owned_init};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    owned_init(deps, env.clone(), owned::msg::InitMsg {})?;
    config(&mut deps.storage).save(&State {
        raising_access_controller: msg.rac_address,
    })?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::RaiseFlag { subject } => handle_raise_flag(deps, env, subject),
        HandleMsg::RaiseFlags { subjects } => handle_raise_flags(deps, env, subjects),
        HandleMsg::LowerFlags { subjects } => handle_lower_flags(deps, env, subjects),
        HandleMsg::SetRaisingAccessController { rac_address } => {
            handle_set_raising_access_controller(deps, env, rac_address)
        }
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFlag { subject } => to_binary(&get_flag(deps, subject)),
        QueryMsg::GetFlags { subjects } => to_binary(&get_flags(deps, subjects)),
    }
}

pub fn handle_raise_flag<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    subject: HumanAddr,
) -> StdResult<HandleResponse> {
    let key = deps.api.canonical_address(&subject)?;
    if flags_read(&mut deps.storage)
        .may_load(key.as_slice())?
        .is_none()
    {
        flags(&mut deps.storage).save(key.as_slice(), &true)?;
        Ok(HandleResponse {
            messages: vec![],
            log: vec![log("action", "raised flag"), log("subject", subject)],
            data: None,
        })
    } else {
        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: None,
        })
    }
}

pub fn handle_raise_flags<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    subjects: Vec<HumanAddr>,
) -> StdResult<HandleResponse> {
    subjects.iter().for_each(|addr| {
        let key = deps.api.canonical_address(&addr).unwrap();
        flags(&mut deps.storage)
            .save(key.as_slice(), &true)
            .unwrap();
    });
    Ok(HandleResponse {
        messages: vec![],
        log: vec![], // TODO: add logs
        data: None,
    })
}

pub fn handle_lower_flags<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    subjects: Vec<HumanAddr>,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;
    subjects.iter().for_each(|subject| {
        let key = deps.api.canonical_address(&subject).unwrap();
        if flags_read(&deps.storage)
            .may_load(key.as_slice())
            .unwrap()
            .is_some()
        {
            flags(&mut deps.storage).save(key.as_slice(), &false).unwrap();
        }
    });
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}

pub fn handle_set_raising_access_controller<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    rac_address: HumanAddr,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;
    config(&mut deps.storage).update(|_state| {
        Ok(State {
            raising_access_controller: rac_address,
        })
    })?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}

pub fn get_flag<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    subject: HumanAddr,
) -> StdResult<bool> {
    let key = deps.api.canonical_address(&subject).unwrap();
    flags_read(&deps.storage).load(key.as_slice())
}

pub fn get_flags<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    subjects: Vec<HumanAddr>,
) -> StdResult<Vec<bool>> {
    let flags = subjects
        .iter()
        .map(|subject| {
            flags_read(&deps.storage)
                .load(deps.api.canonical_address(subject).unwrap().as_slice())
                .unwrap()
        })
        .collect();
    Ok(flags)
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
    use cosmwasm_std::{coins, from_binary, HumanAddr, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            rac_address: HumanAddr::from("rac"),
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let env = mock_env("human", &[]);
        let addr = deps.api.canonical_address(&env.message.sender).unwrap();
        let msg = HandleMsg::RaiseFlag {
            subject: deps.api.human_address(&addr.clone()).unwrap(),
        };

        let _res = handle(&mut deps, env, msg);

        let flag = flags_read(&deps.storage).load(addr.as_slice()).unwrap();
        assert_eq!(true, flag);
    }
}
