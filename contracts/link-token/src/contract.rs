use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw20::{Cw20Coin, TokenInfoResponse};
use cw20_base::{
    allowances::{
        execute_decrease_allowance, execute_increase_allowance, execute_transfer_from,
        query_allowance,
    },
    contract::{create_accounts, execute_send, execute_transfer, query_balance},
    ContractError,
};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{TokenInfo, TOKEN_INFO},
};

pub const TOKEN_NAME: &str = "Chainlink";
pub const TOKEN_SYMBOL: &str = "LINK";
pub const DECIMALS: u8 = 18;
pub const TOTAL_SUPPLY: u128 = 1_000_000_000;

pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let main_balance = Cw20Coin {
        address: info.sender.into(),
        amount: Uint128::from(TOTAL_SUPPLY),
    };
    let total_supply = create_accounts(&mut deps, &[main_balance])?;

    // store token info
    let data = TokenInfo {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply,
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
    }
}

pub fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;

    Ok(info.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Uint128};

    #[test]
    fn test_query_token_info() {
        let mut deps = mock_dependencies(&coins(2, "test_token"));

        let env = mock_env();
        let info = mock_info(&"creator", &[]);
        let _ = instantiate(deps.as_mut(), env, info, InstantiateMsg {}).unwrap();

        let query_res = query_token_info(deps.as_ref()).unwrap();

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
