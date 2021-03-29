use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage,
};
use cw20::TokenInfoResponse;
use cw20_base::{
    allowances::{
        handle_decrease_allowance, handle_increase_allowance, handle_transfer_from, query_allowance,
    },
    contract::{create_accounts, handle_send, handle_transfer, query_balance},
};

use crate::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::{token_info, token_info_read, TokenInfo},
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // check valid token info
    msg.validate()?;
    // create initial accounts
    let total_supply = create_accounts(deps, &msg.initial_balances)?;

    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
    };
    token_info(&mut deps.storage).save(&data)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Transfer { recipient, amount } => handle_transfer(deps, env, recipient, amount),
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => handle_transfer_from(deps, env, owner, recipient, amount),
        HandleMsg::Send {
            contract,
            amount,
            msg,
        } => handle_send(deps, env, contract, amount, msg),
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_increase_allowance(deps, env, spender, amount, expires),
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_decrease_allowance(deps, env, spender, amount, expires),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)),
        QueryMsg::Allowance { owner, spender } => to_binary(&query_allowance(deps, owner, spender)),
    }
}

pub fn query_token_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TokenInfoResponse> {
    let info = token_info_read(&deps.storage).load()?;

    Ok(info.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, HumanAddr, Uint128};
    use cw20::Cw20CoinHuman;

    #[test]
    fn test_query_token_info() {
        let mut deps = mock_dependencies(10, &coins(2, "test_token"));
        let address = HumanAddr::from("address1");
        let amount = Uint128::from(1_000_000_u128);

        let init_msg = InitMsg {
            name: "Test token".to_string(),
            symbol: "TEST".to_string(),
            decimals: 15,
            initial_balances: vec![Cw20CoinHuman {
                address: address,
                amount,
            }],
        };

        let env = mock_env(&HumanAddr("creator".to_string()), &[]);
        let _ = init(&mut deps, env, init_msg).unwrap();

        let query_res = query_token_info(&deps).unwrap();

        assert_eq!(
            query_res,
            TokenInfoResponse {
                name: "Test token".to_string(),
                symbol: "TEST".to_string(),
                decimals: 15,
                total_supply: Uint128::from(1_000_000_u128)
            }
        );
    }
}
