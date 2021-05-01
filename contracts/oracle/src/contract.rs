use std::collections::hash_set::Union;

use cw0::{Duration, Expiration};

use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128,
};

use crate::state::{authorized_nodes, config, config_read, State};
use crate::{
    error::*,
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::authorized_nodes_read,
};

// that should be 5 min?
static EXPIRY_TIME: Duration = Duration::Time(60 * 5);
static MINIMUM_CONSUMER_GAS_LIMIT: u128 = 400000;
static ONE_FOR_CONSISTENT_GAS_COST: u128 = 1;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        link_token: deps.api.canonical_address(&msg.link_token)?,
        withdrawable_tokens: ONE_FOR_CONSISTENT_GAS_COST,
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
        HandleMsg::OracleRequest {
            sender,
            payment,
            spec_id,
            callback_address,
            callback_function_id,
            nonce,
            data_version,
            data,
        } => handle_oracle_request(
            deps,
            env,
            sender,
            payment,
            spec_id,
            callback_address,
            callback_function_id,
            nonce,
            data_version,
            data,
        ),
        HandleMsg::FulfillOracleRequest {
            request_id,
            payment,
            callback_address,
            callback_function_id,
            expiration,
            data,
        } => handle_fulfill_oracle_request(
            deps,
            env,
            request_id,
            payment,
            callback_address,
            callback_function_id,
            expiration,
            data,
        ),
        HandleMsg::SetFulfillmentPermission { node, allowed } => {
            handle_set_fulfillment_permissions(deps, env, node, allowed)
        }
        HandleMsg::Withdraw { recipient, amount } => handle_withdraw(deps, env, recipient, amount),
        HandleMsg::CancelOracleRequest {
            request_id,
            payment,
            callback_func,
            expiration,
        } => {
            handle_cancel_oracle_request(deps, env, request_id, payment, callback_func, expiration)
        }
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAuthorizationStatus { node } => {
            to_binary(&get_authorization_status(deps, node))
        }
        QueryMsg::Withdrawable {} => todo!(),
        QueryMsg::GetChainlinkToken {} => to_binary(&get_chainlink_token(deps)),
        QueryMsg::GetExpiryTime {} => todo!(),
    }
}

pub fn handle_set_fulfillment_permissions<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    node: HumanAddr,
    allowed: bool,
) -> StdResult<HandleResponse> {
    // TODO onlyOwner
    let node_addr = deps.api.canonical_address(&node)?;
    let key = node_addr.as_slice();
    authorized_nodes(&mut deps.storage).save(key, &allowed)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}

pub fn handle_oracle_request<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    payment: Uint128,
    spec_id: Binary,
    callback_address: HumanAddr,
    callback_function_id: Binary,
    nonce: Uint128,
    data_version: Uint128,
    data: Binary,
) -> StdResult<HandleResponse> {
    unimplemented!()
}

pub fn handle_fulfill_oracle_request<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    request_id: Binary,
    payment: Uint128,
    callback_address: HumanAddr,
    callback_function_id: Binary,
    expiration: Uint128,
    data: Binary,
) -> StdResult<HandleResponse> {
    unimplemented!()
}

pub fn handle_withdraw<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    unimplemented!()
}

pub fn handle_cancel_oracle_request<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    request_id: Binary,
    payment: Uint128,
    callback_func: Binary,
    expiration: Uint128,
) -> StdResult<HandleResponse> {
    unimplemented!()
}

pub fn get_authorization_status<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    node: HumanAddr,
) -> StdResult<bool> {
    let auth_status =
        authorized_nodes_read(&deps.storage).load(deps.api.canonical_address(&node)?.as_slice())?;
    Ok(auth_status)
}

pub fn get_chainlink_token<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HumanAddr> {
    let link_token_addr = deps
        .api
        .human_address(&config_read(&deps.storage).load()?.link_token)?;
    Ok(link_token_addr)
}

fn check_callback_address<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
    to: HumanAddr,
) -> StdResult<()> {
    let link_token_addr = deps
        .api
        .human_address(&config_read(&deps.storage).load()?.link_token)?;
    if !link_token_addr.eq(&to) {
        return ContractErr::BadCallback.std_err();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coins, from_binary, StdError};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        HumanAddr,
    };

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let link_token_addr = HumanAddr::from("link");
        let msg = InitMsg {
            link_token: link_token_addr,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetChainlinkToken {}).unwrap();
        let value: StdResult<u32> = from_binary(&res).unwrap();
        assert_eq!(17, value.unwrap());
    }
}
