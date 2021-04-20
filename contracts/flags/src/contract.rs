use cosmwasm_std::{
    to_binary, log, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{flags, flags_read};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
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
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFlag { subject } => todo!(),
        QueryMsg::GetFlags { subjects } => todo!(),
    }
}

pub fn handle_raise_flag<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    subject: CanonicalAddr,
) -> StdResult<HandleResponse> {
    let key = subject.as_slice();
    if flags_read(&mut deps.storage).may_load(subject.as_slice())?.is_none() {
        flags(&mut deps.storage).save(key, &true)?;
        Ok(HandleResponse {
            messages: vec![],
            log: vec![
                log("action", "raised flag"),
                log("subject", subject)
            ],
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
    env: Env,
    subjects: Vec<CanonicalAddr>,
) -> StdResult<HandleResponse> {
    subjects.iter().for_each(|addr| {
        flags(&mut deps.storage)
            .save(addr.as_slice(), &true)
            .unwrap();
    });
    Ok(HandleResponse {
        messages: vec![],
        log: vec![], // TODO: add logs
        data: None,
    })
}

fn validate_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    // let owner = get_owner(deps)?;
    // if env.message.sender != owner {
    //     return ContractErr::NotOwner.std_err();
    // }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let env = mock_env("human", &[]);
        let addr = deps.api.canonical_address(&env.message.sender).unwrap();
        let msg = HandleMsg::RaiseFlag {
            subject: addr.clone(),
        };

        let _res = handle(&mut deps, env, msg);

        let flag = flags_read(&deps.storage).load(addr.as_slice()).unwrap();
        assert_eq!(true, flag);
    }
}
