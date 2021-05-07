use cw0::Expiration;

use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::{
    authorized_nodes, commitments, commitments_read, config, config_read, Commitment, State,
};
use crate::{
    error::*,
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::authorized_nodes_read,
};

use link_token::msg::HandleMsg as LinkMsg;
use owned::contract::{get_owner, init as owned_init};

// TODO static EXPIRY_TIME
static MINIMUM_CONSUMER_GAS_LIMIT: u128 = 400000;
static ONE_FOR_CONSISTENT_GAS_COST: u128 = 1;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    owned_init(deps, env, owned::msg::InitMsg {})?;
    let state = State {
        link_token: deps.api.canonical_address(&msg.link_token)?,
        withdrawable_tokens: Uint128::from(ONE_FOR_CONSISTENT_GAS_COST),
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
            nonce,
            callback_func,
            expiration,
        } => handle_cancel_oracle_request(
            deps,
            env,
            request_id,
            nonce,
            payment,
            callback_func,
            expiration,
        ),
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
    }
}

pub fn handle_set_fulfillment_permissions<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
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
    _data_version: Uint128,
    data: Binary,
) -> StdResult<HandleResponse> {
    validate_unique_commitment_id(deps, &env, nonce)?;
    // that's 5 minutes from now
    let expiration = Expiration::AtTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 300,
    );
    let commitment = Commitment {
        caller_account: sender,
        spec_id,
        callback_address,
        callback_function_id,
        data,
        payment,
        expiration,
    };
    commitments(&mut deps.storage).save(&nonce.0.to_be_bytes(), &commitment)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
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
    validate_ownership(deps, &env)?;
    has_available_funds(deps, &env, amount)?;
    let withdrawable_tokens = config_read(&deps.storage).load()?.withdrawable_tokens;

    config(&mut deps.storage).update(|state| {
        Ok(State {
            withdrawable_tokens: Uint128::from(
                withdrawable_tokens.u128() - ONE_FOR_CONSISTENT_GAS_COST,
            ),
            ..state
        })
    })?;

    let link = config_read(&deps.storage).load()?.link_token;
    let link_addr = deps.api.human_address(&link)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link_addr,
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![transfer_msg.into()],
        log: vec![],
        data: None,
    })
}

pub fn handle_cancel_oracle_request<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _request_id: Binary,
    nonce: Uint128,
    payment: Uint128,
    _callback_func: Binary,
    _expiration: Uint128,
) -> StdResult<HandleResponse> {
    let commitment = commitments_read(&deps.storage).may_load(&nonce.0.to_be_bytes())?;

    if commitment.is_some() && !commitment.unwrap().expiration.is_expired(&env.block) {
        commitments(&mut deps.storage).remove(&nonce.0.to_be_bytes());
    }

    let link = config_read(&deps.storage).load()?.link_token;
    let link_addr = deps.api.human_address(&link)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link_addr,
        msg: to_binary(&LinkMsg::Transfer {
            recipient: env.message.sender,
            amount: payment,
        })?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![transfer_msg.into()],
        log: vec![],
        data: None,
    })
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

fn validate_unique_commitment_id<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: &Env,
    nonce: Uint128,
) -> StdResult<()> {
    if commitments_read(&deps.storage)
        .may_load(&nonce.0.to_be_bytes())?
        .is_some()
    {
        return ContractErr::NotUniqueId.std_err();
    }
    Ok(())
}

fn has_available_funds<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: &Env,
    amount: Uint128,
) -> StdResult<()> {
    let withdrawable_tokens = config_read(&deps.storage).load()?.withdrawable_tokens;
    if withdrawable_tokens.u128() > amount.u128() + ONE_FOR_CONSISTENT_GAS_COST {
        return ContractErr::NotEnoughFunds.std_err();
    }
    Ok(())
}

fn only_authorized_nodes<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let owner = get_owner(deps)?;
    let node = authorized_nodes_read(&deps.storage).may_load(sender_addr.as_slice())?;
    if node.is_none() || env.message.sender != owner {
        return ContractErr::NotAuthorizedNode.std_err();
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
