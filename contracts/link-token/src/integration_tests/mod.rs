#![cfg(test)]

mod receiver_mock;

use crate::{
    contract::{execute, instantiate, query, DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOTAL_SUPPLY},
    integration_tests::receiver_mock::{contract_receiver_mock, MockInstantiateMsg, PingMsg},
    msg::{HandleMsg, InstantiateMsg, QueryMsg},
};
use cosmwasm_std::{
    attr,
    testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, Empty, OverflowError, OverflowOperation, StdError, Uint128,
};
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};
use cw20_base::ContractError;
use cw_multi_test::{App, Contract, ContractWrapper, SimpleBank};

fn mock_app() -> App {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = SimpleBank {};

    App::new(api, env.block, bank, || Box::new(MockStorage::new()))
}

pub fn contract_link_token() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

#[test]
fn test_successful_init() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let sender = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, sender.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let expected_state = TokenInfoResponse {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply: Uint128(TOTAL_SUPPLY),
    };

    let state: TokenInfoResponse = router
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::TokenInfo {})
        .unwrap();

    assert_eq!(state, expected_state);
}

#[test]
fn test_transfer_success() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();
    let recipient_addr = MOCK_CONTRACT_ADDR;

    let msg = HandleMsg::Transfer {
        recipient: recipient_addr.to_owned(),
        amount: Uint128(128),
    };

    router
        .execute_contract(owner, contract.clone(), &msg, &[])
        .unwrap();

    let query = QueryMsg::Balance {
        address: recipient_addr.into(),
    };
    let balance: BalanceResponse = router.wrap().query_wasm_smart(contract, &query).unwrap();

    assert_eq!(balance.balance, Uint128(128));
}

#[test]
fn test_transfer_underflow() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let balance = Uint128(TOTAL_SUPPLY);
    let amount = balance + Uint128(1);

    let msg = HandleMsg::Transfer {
        recipient: MOCK_CONTRACT_ADDR.into(),
        amount,
    };

    let res = router.execute_contract(owner, contract, &msg, &[]);

    assert_eq!(
        res.unwrap_err(),
        ContractError::Std(StdError::Overflow {
            source: OverflowError {
                operation: OverflowOperation::Sub,
                operand1: balance.into(),
                operand2: amount.into()
            }
        })
        .to_string()
    );
}

#[test]
fn test_queries() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let balance_res: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    assert_eq!(balance_res.balance, Uint128(TOTAL_SUPPLY));

    let expected_info = TokenInfoResponse {
        name: TOKEN_NAME.to_string(),
        symbol: TOKEN_SYMBOL.to_string(),
        decimals: DECIMALS,
        total_supply: Uint128(TOTAL_SUPPLY),
    };
    let token_info: TokenInfoResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::TokenInfo {})
        .unwrap();

    assert_eq!(token_info, expected_info);

    let spender_addr = "spender";
    let allowance_query = QueryMsg::Allowance {
        owner: owner.into(),
        spender: spender_addr.to_owned(),
    };
    let res: AllowanceResponse = router
        .wrap()
        .query_wasm_smart(contract, &allowance_query)
        .unwrap();

    assert_eq!(res.allowance, Uint128::zero());
}

#[test]
fn test_modify_allowance() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let spender_addr = "spender";

    let msg = HandleMsg::IncreaseAllowance {
        spender: spender_addr.to_owned(),
        amount: Uint128(100),
        expires: None,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let allowance_query = QueryMsg::Allowance {
        owner: owner.to_string(),
        spender: spender_addr.to_owned(),
    };
    let allowance: AllowanceResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &allowance_query)
        .unwrap();

    let msg = HandleMsg::DecreaseAllowance {
        spender: spender_addr.to_owned(),
        amount: Uint128(50),
        expires: None,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    assert_eq!(allowance.allowance, Uint128(100));

    let allowance_query = QueryMsg::Allowance {
        owner: owner.to_string(),
        spender: spender_addr.to_owned(),
    };
    let allowance: AllowanceResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &allowance_query)
        .unwrap();

    assert_eq!(allowance.allowance, Uint128(50));
}

#[test]
fn test_transfer_from() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let spender = Addr::unchecked("spender");
    let recipient = Addr::unchecked("recipient");
    let amount = Uint128(100);

    let msg = HandleMsg::IncreaseAllowance {
        spender: spender.to_string(),
        amount,
        expires: None,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let allowance_query = QueryMsg::Allowance {
        owner: owner.to_string(),
        spender: spender.to_string(),
    };
    let allowance: AllowanceResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &allowance_query)
        .unwrap();

    assert_eq!(allowance.allowance, amount);

    let msg = HandleMsg::TransferFrom {
        owner: owner.to_string(),
        recipient: recipient.to_string(),
        amount,
    };
    router
        .execute_contract(spender.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let balance_res: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::Balance {
                address: recipient.to_string(),
            },
        )
        .unwrap();
    assert_eq!(balance_res.balance, amount);

    // Allowance should be expended
    let allowance_query = QueryMsg::Allowance {
        owner: owner.to_string(),
        spender: spender.to_string(),
    };
    let allowance: AllowanceResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &allowance_query)
        .unwrap();

    assert_eq!(allowance.allowance, Uint128::zero());
}

#[test]
fn test_transfer_from_without_allowance() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let recipient = "recipient";

    let allowance_query = QueryMsg::Allowance {
        owner: owner.to_string(),
        spender: recipient.to_owned(),
    };
    let allowance: AllowanceResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &allowance_query)
        .unwrap();
    assert_eq!(allowance.allowance, Uint128::zero());

    let msg = HandleMsg::TransferFrom {
        owner: owner.to_string(),
        recipient: recipient.to_owned(),
        amount: Uint128(100),
    };
    let res = router.execute_contract(owner, contract, &msg, &[]);

    assert_eq!(res.unwrap_err(), ContractError::NoAllowance {}.to_string());
}

#[test]
fn test_change_allowance_self() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let msg = HandleMsg::IncreaseAllowance {
        spender: owner.to_string(),
        amount: Uint128(1000),
        expires: None,
    };
    let res = router.execute_contract(owner, contract, &msg, &[]);
    assert_eq!(
        res.unwrap_err(),
        ContractError::CannotSetOwnAccount {}.to_string()
    );
}

#[test]
fn test_send() {
    let mut router = mock_app();
    let id = router.store_code(contract_link_token());
    let owner = Addr::unchecked("owner");
    let contract = router
        .instantiate_contract(id, owner.clone(), &InstantiateMsg {}, &[], "LINK")
        .unwrap();

    let id = router.store_code(contract_receiver_mock());
    let owner_receiver = Addr::unchecked("owner_receiver");
    let receiver = router
        .instantiate_contract(
            id,
            owner_receiver.clone(),
            &MockInstantiateMsg {},
            &[],
            "LINK",
        )
        .unwrap();

    let amount = Uint128(10000);
    let payload = to_binary(&PingMsg {
        payload: "test_data".to_string(),
    })
    .unwrap();

    let send_msg = HandleMsg::Send {
        contract: receiver.to_string(),
        amount,
        msg: payload,
    };

    let res = router
        .execute_contract(owner.clone(), contract.clone(), &send_msg, &[])
        .unwrap();
    assert_eq!(res.attributes.last().unwrap(), &attr("action", "pong"));

    let sender_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    let expected_balance = Uint128(TOTAL_SUPPLY).checked_sub(amount).unwrap();
    assert_eq!(sender_balance.balance, expected_balance);

    let receiver_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::Balance {
                address: receiver.to_string(),
            },
        )
        .unwrap();
    assert_eq!(receiver_balance.balance, amount);
}
