use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use owned::contract::execute_accept_ownership;
use owned::contract::execute_transfer_ownership;

use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;
use owned::contract::{get_owner, instantiate as owned_init};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, owned::error::ContractError> {
    // config(deps.storage).save(&State {
    //     raising_access_controller: msg.rac_address,
    // })?;
    owned_init(deps, env, info, owned::msg::InstantiateMsg {})?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RaiseFlag { subject } => execute_raise_flag(deps, env, info, subject),
        ExecuteMsg::RaiseFlags { subjects } => execute_raise_flags(deps, env, info, subjects),
        ExecuteMsg::LowerFlags { subjects } => execute_lower_flags(deps, env, info, subjects),
        ExecuteMsg::SetRaisingAccessController { rac_address } => {
            execute_set_raising_access_controller(deps, env, info, rac_address)
        }
        ExecuteMsg::TransferOwnership { to } => {
            execute_transfer_ownership(deps, env, info, to).map_err(ContractError::from)
        }
        ExecuteMsg::AcceptOwnership {} => {
            execute_accept_ownership(deps, env, info).map_err(ContractError::from)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    match msg {
        QueryMsg::GetFlag { subject } => Ok(to_binary(&get_flag(deps, subject)?)?),
        QueryMsg::GetFlags { subjects } => Ok(to_binary(&get_flags(deps, subjects)?)?),
        QueryMsg::GetRac {} => Ok(to_binary(&get_rac(deps)?)?),
        QueryMsg::GetOwner {} => Ok(to_binary(&get_owner(deps)?)?),
    }
}

pub fn execute_raise_flag(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    subject: String,
) -> Result<Response, ContractError> {
    check_access(deps.as_ref())?;
    let subject = deps.api.addr_validate(&subject)?;
    if FLAGS.may_load(deps.as_ref().storage, &subject)? == Some(true) {
        Ok(Response::new().add_attributes(vec![
            attr("action", "already raised flag"),
            attr("subject", subject),
        ]))
    } else {
        FLAGS.save(deps.storage, &subject, &true)?;
        Ok(Response::new().add_attributes(vec![
            attr("action", "raised flag"),
            attr("subject", subject),
        ]))
    }
}

pub fn execute_raise_flags(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    subjects: Vec<String>,
) -> Result<Response, ContractError> {
    check_access(deps.as_ref())?;

    let subjects = subjects
        .iter()
        .map(|subject| deps.api.addr_validate(subject))
        .collect::<Result<Vec<Addr>, _>>()?;

    let mut attributes = vec![];
    for subject in subjects {
        if FLAGS.may_load(deps.as_ref().storage, &subject)? == Some(true) {
            attributes.extend_from_slice(&[
                attr("action", "already raised flag"),
                attr("subject", subject),
            ]);
        } else {
            FLAGS.save(deps.storage, &subject, &true)?;
            attributes
                .extend_from_slice(&[attr("action", "flag raised"), attr("subject", subject)]);
        }
    }
    Ok(Response::new().add_attributes(attributes))
}

pub fn execute_lower_flags(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    subjects: Vec<String>,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &env, info)?;

    let subjects = subjects
        .iter()
        .map(|subject| deps.api.addr_validate(subject))
        .collect::<Result<Vec<Addr>, _>>()?;

    let mut attributes = vec![];
    for subject in subjects {
        if FLAGS.may_load(deps.storage, &subject)? == Some(true) {
            FLAGS.save(deps.storage, &subject, &false)?;
            attributes
                .extend_from_slice(&[attr("action", "flag lowered"), attr("address", subject)]);
        }
    }
    Ok(Response::new().add_attributes(attributes))
}

pub fn execute_set_raising_access_controller(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rac_address: String,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &env, info)?;

    let new_rac = deps.api.addr_validate(&rac_address)?;
    let prev_rac = config_read(deps.storage).load()?.raising_access_controller;
    config(deps.storage).save(&State {
        raising_access_controller: new_rac,
    })?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "raising access controller updated"),
        attr("address", rac_address),
        attr("previous", prev_rac),
    ]))
}

pub fn get_flag(deps: Deps, subject: String) -> Result<bool, ContractError> {
    check_access(deps)?;
    let subject = deps.api.addr_validate(&subject)?;
    Ok(FLAGS.load(deps.storage, &subject)?)
}

pub fn get_flags(deps: Deps, subjects: Vec<String>) -> Result<Vec<bool>, ContractError> {
    check_access(deps)?;

    let subjects = subjects
        .iter()
        .map(|subject| deps.api.addr_validate(subject))
        .collect::<Result<Vec<Addr>, _>>()?;

    let flags = subjects
        .iter()
        .filter_map(|subject| {
            let flag = FLAGS.load(deps.storage, subject).ok()?;
            Some(flag)
        })
        .collect();
    Ok(flags)
}

pub fn get_rac(deps: Deps) -> Result<Addr, ContractError> {
    let raising_access_controller = config_read(deps.storage).load()?.raising_access_controller;
    Ok(raising_access_controller)
}

fn validate_ownership(deps: Deps, _env: &Env, info: MessageInfo) -> Result<(), ContractError> {
    let owner = get_owner(deps)?;
    if info.sender != owner {
        return Err(ContractError::NotOwner {});
    }
    Ok(())
}

// TODO this needs to be an actual call to access controller
fn check_access(_deps: Deps) -> Result<(), ContractError> {
    if false {
        return Err(ContractError::NoAccess {});
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn raise_flag() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let info = mock_info("human", &[]);
        let sender = "human".to_string();

        let msg = ExecuteMsg::RaiseFlag {
            subject: sender.clone(),
        };

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());

        let flag = get_flag(deps.as_ref(), sender.clone()).unwrap();
        assert_eq!(true, flag);

        // trying to raise the flag when it's already raised
        let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
        assert_eq!(
            vec![
                attr("action", "already raised flag"),
                attr("subject", sender.clone())
            ],
            res.unwrap().attributes
        );
    }

    #[test]
    fn raise_flags() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let info = mock_info("human", &[]);
        let sender = "human".to_string();

        let _res = execute_raise_flags(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            vec![sender.clone()],
        );

        let flags = get_flags(deps.as_ref(), vec![sender.clone()]);
        assert_eq!(vec![true], flags.unwrap());

        let msg = ExecuteMsg::RaiseFlags {
            subjects: vec![sender.clone()],
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
        assert_eq!(
            vec![
                attr("action", "already raised flag"),
                attr("subject", sender.clone())
            ],
            res.unwrap().attributes
        );
    }
}
