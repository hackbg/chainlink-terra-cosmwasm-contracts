use cosmwasm_std::{
    coins, from_binary, testing::MOCK_CONTRACT_ADDR, to_binary, Empty, Env, HandleResponse,
    HumanAddr, InitResponse, StdError, StdResult, Uint128,
};
use cosmwasm_storage::to_length_prefixed;
use cosmwasm_vm::{
    from_slice,
    testing::{handle, init, mock_env, mock_instance, query, MockApi, MockQuerier, MockStorage},
    Instance, Storage,
};
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};
use link_token::{
    contract::{DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOTAL_SUPPLY},
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::TOKEN_INFO_KEY,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static WASM: &[u8] = include_bytes!("../../../artifacts/link_token.wasm");

fn default_init() -> (Instance<MockStorage, MockApi, MockQuerier<Empty>>, Env) {
    let mut deps = mock_instance(WASM, &[]);
    assert_eq!(deps.required_features.len(), 0);

    let env = mock_env("creator", &coins(1000, "earth"));
    let _: InitResponse = init(&mut deps, env.clone(), InitMsg {}).unwrap();

    (deps, env)
}

fn query_balance(
    deps: &mut Instance<MockStorage, MockApi, MockQuerier<Empty>>,
    address: HumanAddr,
) -> BalanceResponse {
    let balance_query = QueryMsg::Balance { address };
    let res = query(deps, balance_query).unwrap();
    let balance: StdResult<BalanceResponse> = from_binary(&res).unwrap();

    balance.unwrap()
}

#[test]
fn test_successful_init() {
    let (mut deps, _env) = default_init();

    let expected_state = TokenInfoResponse {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply: Uint128(TOTAL_SUPPLY),
    };

    let state: TokenInfoResponse = deps
        .with_storage(|store| {
            let data = store
                .get(&to_length_prefixed(TOKEN_INFO_KEY))
                .0
                .expect("error reading db")
                .expect("no data stored");
            from_slice(&data)
        })
        .unwrap();
    assert_eq!(state, expected_state);
}

#[test]
fn test_transfer_success() {
    let (mut deps, env) = default_init();

    let recipient_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);

    let msg = HandleMsg::Transfer {
        recipient: recipient_addr.clone(),
        amount: Uint128(128),
    };

    let _: Result<HandleResponse, StdError> = handle(&mut deps, env, msg);

    let balance_res = query_balance(&mut deps, recipient_addr);
    assert_eq!(balance_res.balance, Uint128(128));
}

#[test]
fn test_transfer_underflow() {
    let (mut deps, env) = default_init();

    let balance = Uint128(TOTAL_SUPPLY);
    let amount = balance + Uint128(1);

    let msg = HandleMsg::Transfer {
        recipient: env.message.sender.clone(),
        amount,
    };

    let res: Result<HandleResponse, StdError> = handle(&mut deps, env, msg);
    assert_eq!(
        res.err().unwrap(),
        StdError::Underflow {
            minuend: balance.to_string(),
            subtrahend: amount.to_string(),
            backtrace: None
        }
    );
}

#[test]
fn test_queries() {
    let (mut deps, env) = default_init();

    let owner_addr = env.message.sender;

    let balance_res = query_balance(&mut deps, owner_addr.clone());
    assert_eq!(balance_res.balance, Uint128(TOTAL_SUPPLY));

    let expected_info = TokenInfoResponse {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply: Uint128(TOTAL_SUPPLY),
    };
    let token_info_query = QueryMsg::TokenInfo {};
    let res = query(&mut deps, token_info_query).unwrap();
    let token_info: StdResult<TokenInfoResponse> = from_binary(&res).unwrap();

    assert_eq!(token_info.unwrap(), expected_info);

    let spender_addr = HumanAddr::from("spender");
    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr,
        spender: spender_addr,
    };
    let res = query(&mut deps, allowance_query).unwrap();
    let allowance: StdResult<AllowanceResponse> = from_binary(&res).unwrap();

    assert_eq!(allowance.unwrap().allowance, Uint128::zero());
}

#[test]
fn test_modify_allowance() {
    let (mut deps, env) = default_init();

    let owner_addr = env.message.sender.clone();
    let spender_addr = HumanAddr::from("spender");

    let msg = HandleMsg::IncreaseAllowance {
        spender: spender_addr.clone(),
        amount: Uint128(100),
        expires: None,
    };
    let _: Result<HandleResponse, StdError> = handle(&mut deps, env.clone(), msg);

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.clone(),
        spender: spender_addr.clone(),
    };
    let res = query(&mut deps, allowance_query).unwrap();
    let allowance: StdResult<AllowanceResponse> = from_binary(&res).unwrap();

    let msg = HandleMsg::DecreaseAllowance {
        spender: spender_addr.clone(),
        amount: Uint128(50),
        expires: None,
    };
    let _: Result<HandleResponse, StdError> = handle(&mut deps, env, msg);

    assert_eq!(allowance.unwrap().allowance, Uint128(100));

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr,
        spender: spender_addr,
    };
    let res = query(&mut deps, allowance_query).unwrap();
    let allowance: StdResult<AllowanceResponse> = from_binary(&res).unwrap();

    assert_eq!(allowance.unwrap().allowance, Uint128(50));
}

#[test]
fn test_transfer_from() {
    let (mut deps, env) = default_init();

    let owner_addr = env.message.sender.clone();
    let spender_addr = HumanAddr::from("spender");
    let recipient_addr = HumanAddr::from("recipient");
    let amount = Uint128(100);

    let msg = HandleMsg::IncreaseAllowance {
        spender: spender_addr.clone(),
        amount,
        expires: None,
    };
    let _: Result<HandleResponse, StdError> = handle(&mut deps, env.clone(), msg);

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.clone(),
        spender: spender_addr.clone(),
    };
    let res = query(&mut deps, allowance_query).unwrap();
    let allowance: StdResult<AllowanceResponse> = from_binary(&res).unwrap();

    assert_eq!(allowance.unwrap().allowance, amount);

    let env = mock_env(spender_addr.clone(), &[]);
    let msg = HandleMsg::TransferFrom {
        owner: owner_addr.clone(),
        recipient: recipient_addr.clone(),
        amount,
    };
    let res: Result<HandleResponse, StdError> = handle(&mut deps, env, msg);
    assert!(res.is_ok());

    let balance_res = query_balance(&mut deps, recipient_addr);
    assert_eq!(balance_res.balance, amount);

    // Allowance should be expended
    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.clone(),
        spender: spender_addr.clone(),
    };
    let res = query(&mut deps, allowance_query).unwrap();
    let allowance: StdResult<AllowanceResponse> = from_binary(&res).unwrap();

    assert_eq!(allowance.unwrap().allowance, Uint128::zero());
}

#[test]
fn test_transfer_from_without_allowance() {
    let (mut deps, env) = default_init();

    let owner_addr = env.message.sender.clone();
    let recipient_addr = HumanAddr::from("recipient");

    let allowance_query = QueryMsg::Allowance {
        owner: owner_addr.clone(),
        spender: recipient_addr.clone(),
    };
    let res = query(&mut deps, allowance_query).unwrap();
    let allowance: StdResult<AllowanceResponse> = from_binary(&res).unwrap();

    assert_eq!(allowance.unwrap().allowance, Uint128::zero());

    let msg = HandleMsg::TransferFrom {
        owner: owner_addr.clone(),
        recipient: recipient_addr,
        amount: Uint128(100),
    };

    let res: Result<HandleResponse, StdError> = handle(&mut deps, env, msg);

    assert_eq!(
        res.err().unwrap(),
        StdError::generic_err("No allowance for this account")
    );
}

#[test]
fn test_change_allowance_self() {
    let (mut deps, env) = default_init();

    let owner_addr = env.message.sender.clone();

    let msg = HandleMsg::IncreaseAllowance {
        spender: owner_addr.clone(),
        amount: Uint128(1000),
        expires: None,
    };
    let res: Result<HandleResponse, StdError> = handle(&mut deps, env.clone(), msg);

    assert_eq!(
        res.err().unwrap(),
        StdError::generic_err("Cannot set allowance to own account")
    );
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
struct TestMsg {
    payload: String,
}

#[test]
fn test_send() {
    let (mut deps, env) = default_init();

    let contract_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);
    let amount = Uint128(10000);
    let payload = to_binary(&TestMsg {
        payload: "test_data".to_string(),
    })
    .unwrap();

    let send_msg = HandleMsg::Send {
        contract: contract_addr.clone(),
        amount,
        msg: Some(payload),
    };

    let res: Result<HandleResponse, StdError> = handle(&mut deps, env.clone(), send_msg);

    let sender_balance = query_balance(&mut deps, env.message.sender);
    let expected_balance = (Uint128(TOTAL_SUPPLY) - amount).unwrap();
    assert_eq!(sender_balance.balance, expected_balance);

    let receiver_balance = query_balance(&mut deps, contract_addr);
    assert_eq!(receiver_balance.balance, amount);

    let handle_response = res.unwrap();
    assert!(!handle_response.messages.is_empty());
}
