use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage,
    Uint128,
};
use cw20::{Cw20CoinHuman, TokenInfoResponse};
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

pub const TOKEN_NAME: &str = "Chainlink";
pub const TOKEN_SYMBOL: &str = "LINK";
pub const DECIMALS: u8 = 18;
pub const TOTAL_SUPPLY: u128 = 1_000_000_000;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    let main_balance = Cw20CoinHuman {
        address: env.message.sender,
        amount: Uint128::from(TOTAL_SUPPLY),
    };
    let total_supply = create_accounts(deps, &[main_balance])?;

    // store token info
    let data = TokenInfo {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
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

    #[test]
    fn test_query_token_info() {
        let mut deps = mock_dependencies(10, &coins(2, "test_token"));

        let env = mock_env(&HumanAddr("creator".to_string()), &[]);
        let _ = init(&mut deps, env, InitMsg {}).unwrap();

        let query_res = query_token_info(&deps).unwrap();

        assert_eq!(
            query_res,
            TokenInfoResponse {
                name: "Chainlink".to_string(),
                symbol: "LINK".to_string(),
                decimals: 18,
                total_supply: Uint128::from(1_000_000_000_u128)
            }
        );
    }
}
