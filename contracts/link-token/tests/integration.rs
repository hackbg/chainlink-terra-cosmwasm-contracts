use cosmwasm_std::{
    from_binary, testing::MOCK_CONTRACT_ADDR, to_binary, ContractResult, Empty, Env, MessageInfo,
    Response, Uint128,
};
use cosmwasm_vm::{
    testing::{
        execute, instantiate, mock_env, mock_info, mock_instance, query, MockApi, MockQuerier,
        MockStorage,
    },
    Instance,
};
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};
use cw20_base::ContractError;
use link_token::{
    contract::{DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOTAL_SUPPLY},
    msg::{HandleMsg, InstantiateMsg, QueryMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/link_token.wasm");

fn default_init() -> (
    Instance<MockApi, MockStorage, MockQuerier<Empty>>,
    Env,
    MessageInfo,
) {
    let mut deps = mock_instance(WASM, &[]);
    assert_eq!(deps.required_features.len(), 0);

    let env = mock_env();
    let info = mock_info("creator", &[]);
    let _: Response = instantiate(&mut deps, env.clone(), info.clone(), InstantiateMsg {}).unwrap();

    (deps, env, info)
}

fn query_balance(
    deps: &mut Instance<MockApi, MockStorage, MockQuerier<Empty>>,
    env: Env,
    address: String,
) -> BalanceResponse {
    let balance_query = QueryMsg::Balance { address };
    let res = query(deps, env, balance_query).unwrap();
    let balance: BalanceResponse = from_binary(&res).unwrap();

    balance
}

#[test]
fn test_successful_init() {
    let (mut deps, env, _info) = default_init();

    let expected_state = TokenInfoResponse {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply: Uint128(TOTAL_SUPPLY),
    };

    let res = query(&mut deps, env, QueryMsg::TokenInfo {}).unwrap();
    let state: TokenInfoResponse = from_binary(&res).unwrap();

    assert_eq!(state, expected_state);
}

#[test]
fn test_transfer_success() {
    let (mut deps, env, info) = default_init();

    let recipient_addr = MOCK_CONTRACT_ADDR;

    let msg = HandleMsg::Transfer {
        recipient: recipient_addr.to_owned(),
        amount: Uint128(128),
    };

    let _: ContractResult<Response> = execute(&mut deps, env.clone(), info, msg);

    let balance_res = query_balance(&mut deps, env, recipient_addr.to_owned());
    assert_eq!(balance_res.balance, Uint128(128));
}

#[test]
fn test_transfer_underflow() {
    let (mut deps, env, info) = default_init();

    let balance = Uint128(TOTAL_SUPPLY);
    let amount = balance + Uint128(1);

    let msg = HandleMsg::Transfer {
        recipient: info.clone().sender.into(),
        amount,
    };

    let res: ContractResult<Response> = execute(&mut deps, env, info, msg);
    assert!(res.is_err());
    // assert_eq!(
    //     res.unwrap_err(),
    //     StdError::Underflow {
    //         minuend: balance.to_string(),
    //         subtrahend: amount.to_string(),
    //         backtrace: None
    //     }
    // );
}

#[test]
fn test_queries() {
    let (mut deps, env, info) = default_init();

    let owner_addr = info.clone().sender.to_string();

    let balance_res = query_balance(&mut deps, env.clone(), owner_addr.clone());
    assert_eq!(balance_res.balance, Uint128(TOTAL_SUPPLY));

    let expected_info = TokenInfoResponse {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply: Uint128(TOTAL_SUPPLY),
    };
    let token_info_query = QueryMsg::TokenInfo {};
    let res = query(&mut deps, env.clone(), token_info_query).unwrap();
    let token_info: TokenInfoResponse = from_binary(&res).unwrap();

    assert_eq!(token_info, expected_info);

    let spender_addr = "spender";
    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr,
        spender: spender_addr.to_owned(),
    };
    let res = query(&mut deps, env, allowance_query).unwrap();
    let allowance: AllowanceResponse = from_binary(&res).unwrap();

    assert_eq!(allowance.allowance, Uint128::zero());
}

#[test]
fn test_modify_allowance() {
    let (mut deps, env, info) = default_init();

    let owner_addr = info.clone().sender;
    let spender_addr = "spender";

    let msg = HandleMsg::IncreaseAllowance {
        spender: spender_addr.to_owned(),
        amount: Uint128(100),
        expires: None,
    };
    let _: ContractResult<Response> = execute(&mut deps, env.clone(), info.clone(), msg);

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.to_string(),
        spender: spender_addr.to_owned(),
    };
    let res = query(&mut deps, env.clone(), allowance_query).unwrap();
    let allowance: AllowanceResponse = from_binary(&res).unwrap();

    let msg = HandleMsg::DecreaseAllowance {
        spender: spender_addr.to_owned(),
        amount: Uint128(50),
        expires: None,
    };
    let _: ContractResult<Response> = execute(&mut deps, env.clone(), info, msg);

    assert_eq!(allowance.allowance, Uint128(100));

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.into(),
        spender: spender_addr.to_owned(),
    };
    let res = query(&mut deps, env, allowance_query).unwrap();
    let allowance: AllowanceResponse = from_binary(&res).unwrap();

    assert_eq!(allowance.allowance, Uint128(50));
}

#[test]
fn test_transfer_from() {
    let (mut deps, env, info) = default_init();

    let owner_addr = info.clone().sender.to_string();
    let spender_addr = "spender";
    let recipient_addr = "recipient";
    let amount = Uint128(100);

    let msg = HandleMsg::IncreaseAllowance {
        spender: spender_addr.into(),
        amount,
        expires: None,
    };
    let _: ContractResult<Response> = execute(&mut deps, env.clone(), info, msg);

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.clone(),
        spender: spender_addr.to_owned(),
    };
    let res = query(&mut deps, env.clone(), allowance_query).unwrap();
    let allowance: AllowanceResponse = from_binary(&res).unwrap();

    assert_eq!(allowance.allowance, amount);

    let info = mock_info(spender_addr, &[]);
    let msg = HandleMsg::TransferFrom {
        owner: owner_addr.clone(),
        recipient: recipient_addr.to_owned(),
        amount,
    };
    let res: ContractResult<Response> = execute(&mut deps, env.clone(), info, msg);
    assert!(res.is_ok());

    let balance_res = query_balance(&mut deps, env.clone(), recipient_addr.to_owned());
    assert_eq!(balance_res.balance, amount);

    // Allowance should be expended
    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr,
        spender: spender_addr.to_owned(),
    };
    let res = query(&mut deps, env, allowance_query).unwrap();
    let allowance: AllowanceResponse = from_binary(&res).unwrap();

    assert_eq!(allowance.allowance, Uint128::zero());
}

#[test]
fn test_transfer_from_without_allowance() {
    let (mut deps, env, info) = default_init();

    let owner_addr = info.clone().sender.to_string();
    let recipient_addr = "recipient";

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.clone(),
        spender: recipient_addr.to_owned(),
    };
    let res = query(&mut deps, env.clone(), allowance_query).unwrap();
    let allowance: AllowanceResponse = from_binary(&res).unwrap();

    assert_eq!(allowance.allowance, Uint128::zero());

    let msg = HandleMsg::TransferFrom {
        owner: owner_addr,
        recipient: recipient_addr.to_owned(),
        amount: Uint128(100),
    };

    let res: ContractResult<Response> = execute(&mut deps, env, info, msg);

    assert_eq!(res.unwrap_err(), ContractError::NoAllowance {}.to_string());
}

#[test]
fn test_change_allowance_self() {
    let (mut deps, env, info) = default_init();

    let owner_addr = info.sender.clone();

    let msg = HandleMsg::IncreaseAllowance {
        spender: owner_addr.into(),
        amount: Uint128(1000),
        expires: None,
    };
    let res: ContractResult<Response> = execute(&mut deps, env.clone(), info, msg);

    assert_eq!(
        res.unwrap_err(),
        // StdError::generic_err("Cannot set allowance to own account")
        ContractError::CannotSetOwnAccount {}.to_string()
    );
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
struct TestMsg {
    payload: String,
}

#[test]
fn test_send() {
    let (mut deps, env, info) = default_init();

    let contract_addr = MOCK_CONTRACT_ADDR;
    let amount = Uint128(10000);
    let payload = to_binary(&TestMsg {
        payload: "test_data".to_string(),
    })
    .unwrap();

    let send_msg = HandleMsg::Send {
        contract: contract_addr.to_owned(),
        amount,
        msg: payload,
    };

    let res: ContractResult<Response> = execute(&mut deps, env.clone(), info.clone(), send_msg);

    let sender_balance = query_balance(&mut deps, env.clone(), info.sender.into());
    let expected_balance = Uint128(TOTAL_SUPPLY).checked_sub(amount).unwrap();
    assert_eq!(sender_balance.balance, expected_balance);

    let receiver_balance = query_balance(&mut deps, env, contract_addr.into());
    assert_eq!(receiver_balance.balance, amount);

    let handle_response = res.unwrap();
    assert!(!handle_response.messages.is_empty());
}
