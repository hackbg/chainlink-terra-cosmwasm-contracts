#![cfg(test)]

use chainlink_aggregator::{QueryMsg::*, RoundDataResponse};
use cosmwasm_std::{
    attr, from_binary,
    testing::{mock_env, MockApi, MockStorage},
    Addr, Attribute, Binary, Empty, Uint128,
};
use cw20::BalanceResponse;
use cw_multi_test::{App, BankKeeper, Contract, ContractWrapper, Executor};

use crate::{
    contract::{execute, instantiate, query},
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
};

macro_rules! personas {
    [$($name:ident), *] => {
        vec![$(stringify!($name).to_owned()), *]
    };
}

static MIN_ANS: u32 = 1;
static MAX_ANS: u32 = 1;
static RESTART_DELAY: u32 = 0;
static PAYMENT_AMOUNT: Uint128 = Uint128::new(3);
static DEPOSIT: Uint128 = Uint128::new(100);
static ANSWER: Uint128 = Uint128::new(100);

fn mock_app() -> App {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();

    App::new(api, env.block, bank, storage)
}

pub fn contract_flux_aggregator() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub fn contract_link_token() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        link_token::contract::execute,
        link_token::contract::instantiate,
        link_token::contract::query,
    );
    Box::new(contract)
}

pub fn contract_df_validator() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        deviation_flagging_validator::contract::execute,
        deviation_flagging_validator::contract::instantiate,
        deviation_flagging_validator::contract::query,
    );
    Box::new(contract)
}

fn default_init() -> (App, Addr, Addr, Addr) {
    let mut router = mock_app();
    let owner = Addr::unchecked("owner");

    let id = router.store_code(contract_link_token());
    let link_addr = router
        .instantiate_contract(
            id,
            owner.clone(),
            &link_token::msg::InstantiateMsg {},
            &[],
            "LINK",
            None,
        )
        .unwrap();

    let id = router.store_code(contract_df_validator());
    let validator_addr = router
        .instantiate_contract(
            id,
            owner.clone(),
            &deviation_flagging_validator::msg::InstantiateMsg {
                flags: "flags".to_owned(),
                flagging_threshold: 100000,
            },
            &[],
            "Deviation Flagging Validator",
            None,
        )
        .unwrap();

    let id = router.store_code(contract_flux_aggregator());
    let contract = router
        .instantiate_contract(
            id,
            owner.clone(),
            &InstantiateMsg {
                link: link_addr.to_string(),
                payment_amount: PAYMENT_AMOUNT,
                timeout: 1800,
                validator: validator_addr.to_string(),
                min_submission_value: Uint128::new(1),
                max_submission_value: Uint128::new(10000000),
                decimals: 18,
                description: "LINK/USD".to_string(),
            },
            &[],
            "Flux aggregator",
            None,
        )
        .unwrap();

    // Supply contract with funds
    router
        .execute_contract(
            owner.clone(),
            link_addr.clone(),
            &link_token::msg::ExecuteMsg::Send {
                contract: contract.to_string(),
                amount: DEPOSIT,
                msg: Binary::from(b""),
            },
            &[],
        )
        .unwrap();

    (router, owner, link_addr, contract)
}

#[test]
fn successful_init() {
    // Successfull init should not panic
    let _ = default_init();
}

#[test]
fn submit_funds() {
    let oracles = personas![Neil, Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let allocated: Uint128 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAllocatedFunds {})
        .unwrap();
    assert_eq!(allocated, Uint128::zero());

    let msg = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Neil"), contract.clone(), &msg, &[])
        .unwrap();

    let allocated: Uint128 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAllocatedFunds {})
        .unwrap();
    assert_eq!(allocated, PAYMENT_AMOUNT);

    let expected = DEPOSIT.checked_sub(PAYMENT_AMOUNT).unwrap();
    let available: Uint128 = router
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::GetAvailableFunds {})
        .unwrap();
    assert_eq!(available, expected);
}

#[test]
fn submit_withdrawable_payment() {
    let oracles = personas![Neil, Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let allocated: Uint128 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAllocatedFunds {})
        .unwrap();
    assert_eq!(allocated, Uint128::zero());

    let submission = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Neil"), contract.clone(), &submission, &[])
        .unwrap();

    let withdrawable: Uint128 = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::GetWithdrawablePayment {
                oracle: "Neil".into(),
            },
        )
        .unwrap();
    assert_eq!(withdrawable, PAYMENT_AMOUNT);

    let withdrawable: Uint128 = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::GetWithdrawablePayment {
                oracle: "Ned".into(),
            },
        )
        .unwrap();
    assert_eq!(withdrawable, Uint128::zero());

    let withdrawable: Uint128 = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::GetWithdrawablePayment {
                oracle: "Nelly".into(),
            },
        )
        .unwrap();
    assert_eq!(withdrawable, Uint128::zero());
}

#[test]
fn submit_unfinished_round() {
    let oracles = personas![Neil, Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let round: RoundDataResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::AggregatorQuery(GetLatestRoundData {}),
        )
        .unwrap();
    assert!(round.answer.is_none());

    let submission = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();
    router
        .execute_contract(Addr::unchecked("Nelly"), contract.clone(), &submission, &[])
        .unwrap();

    let round: RoundDataResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::AggregatorQuery(GetLatestRoundData {}),
        )
        .unwrap();
    // answer should not be updated
    assert!(round.answer.is_none());
}

#[test]
fn submit_complete_round() {
    let oracles = personas![Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let round: RoundDataResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::AggregatorQuery(GetLatestRoundData {}),
        )
        .unwrap();
    assert!(round.answer.is_none());

    router
        .execute_contract(
            Addr::unchecked("Ned"),
            contract.clone(),
            &ExecuteMsg::Submit {
                round_id: 1,
                submission: Uint128::new(100),
            },
            &[],
        )
        .unwrap();
    router
        .execute_contract(
            Addr::unchecked("Nelly"),
            contract.clone(),
            &ExecuteMsg::Submit {
                round_id: 1,
                submission: Uint128::new(200),
            },
            &[],
        )
        .unwrap();

    let round: RoundDataResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::AggregatorQuery(GetLatestRoundData {}),
        )
        .unwrap();
    assert!(round.updated_at.is_some());
    assert_eq!(round.answer, Some(Uint128::new(150))); // (100 + 200) / 2
}

#[test]
fn submit_twice() {
    let oracles = personas![Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();
    let submission = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    let sender = Addr::unchecked("Nelly");
    router
        .execute_contract(sender.clone(), contract.clone(), &submission, &[])
        .unwrap();
    let res = router.execute_contract(sender, contract.clone(), &submission, &[]);
    assert_eq!(
        res.unwrap_err(),
        ContractError::ReportingPreviousRound {}.to_string()
    );
}

#[test]
fn withdraw_funds_success() {
    use link_token::msg::QueryMsg::*;

    let (mut router, owner, link_addr, contract) = default_init();

    let initial_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            link_addr.clone(),
            &Balance {
                address: contract.to_string(),
            },
        )
        .unwrap();
    assert!(initial_balance.balance == Uint128::new(100));

    let amount = Uint128::new(85);
    let expected_remaining = Uint128::new(15);

    let recipient = "recipient";
    let withdraw = ExecuteMsg::WithdrawFunds {
        recipient: recipient.into(),
        amount,
    };
    router
        .execute_contract(owner, contract.clone(), &withdraw, &[])
        .unwrap();

    let available: Uint128 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAvailableFunds {})
        .unwrap();
    assert!(available == expected_remaining);

    let new_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            link_addr,
            &Balance {
                address: contract.to_string(),
            },
        )
        .unwrap();

    assert!(new_balance.balance == expected_remaining)
}

#[test]
fn withdraw_funds_insufficient() {
    let (mut router, owner, _link_addr, contract) = default_init();

    let amount = Uint128::new(850);

    let recipient = "recipient";
    let withdraw = ExecuteMsg::WithdrawFunds {
        recipient: recipient.into(),
        amount,
    };

    let res = router.execute_contract(owner, contract.clone(), &withdraw, &[]);
    assert_eq!(
        res.unwrap_err(),
        ContractError::InsufficientReserveFunds {}.to_string()
    );
}

#[test]
fn add_oracles() {
    let (mut router, owner, _link_addr, contract) = default_init();

    let old_count: u8 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetOracleCount {})
        .unwrap();
    assert_eq!(old_count, 0);

    let oracle_to_add = "oracle";

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: vec![oracle_to_add.into()],
        added_admins: vec![oracle_to_add.into()],
        min_submissions: MIN_ANS,
        max_submissions: MAX_ANS,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner, contract.clone(), &msg, &[])
        .unwrap();

    let new_count: u8 = router
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::GetOracleCount {})
        .unwrap();
    assert_eq!(new_count, 1);
}

#[test]
fn remove_oracles() {
    let (mut router, owner, _link_addr, contract) = default_init();

    let oracle = "oracle";
    let oracle_to_remove = "oracle_to_remove";
    let oracles = vec![oracle.to_string(), oracle_to_remove.to_string()];
    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let old_count: u8 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetOracleCount {})
        .unwrap();
    assert_eq!(old_count, 2);

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![oracle_to_remove.into()],
        added: vec![],
        added_admins: vec![],
        min_submissions: MIN_ANS,
        max_submissions: MAX_ANS,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner, contract.clone(), &msg, &[])
        .unwrap();

    let new_count: u8 = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetOracleCount {})
        .unwrap();
    assert_eq!(new_count, 1);

    let remaining_oracles: Vec<Addr> = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetOracles {})
        .unwrap();
    assert_eq!(vec![Addr::unchecked(oracle)], remaining_oracles);
}

#[test]
fn set_requester_permissions() {
    let oracles = personas![Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: 1,
        max_submissions: 1,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let submission = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();
    // attempt without permissions
    let res = router.execute_contract(
        Addr::unchecked("Ned"),
        contract.clone(),
        &ExecuteMsg::RequestNewRound {},
        &[],
    );
    assert_eq!(res.unwrap_err(), ContractError::Unauthorized {}.to_string());

    let msg = ExecuteMsg::SetRequesterPermissions {
        requester: "Ned".into(),
        authorized: true,
        delay: 0,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();
    // attempt with permissions
    let res = router
        .execute_contract(
            Addr::unchecked("Ned"),
            contract.clone(),
            &ExecuteMsg::RequestNewRound {},
            &[],
        )
        .unwrap();
    let round_id: u32 = from_binary(&res.data.unwrap()).unwrap();
    assert_eq!(round_id, 2);

    // set the same permission a second time
    // should not panic
    let msg = ExecuteMsg::SetRequesterPermissions {
        requester: "Ned".into(),
        authorized: true,
        delay: 0,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();
}

#[test]
fn set_validator() {
    let (mut router, owner, _link_addr, contract) = default_init();

    let new_validator = "new_validator";
    let msg = ExecuteMsg::SetValidator {
        validator: new_validator.into(),
    };
    let res = router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();
    // successful change should return more than 1 attribute (first is reserved for contract_address)
    let attributes = res.events.last().unwrap().attributes.clone();
    assert_eq!(attributes.len(), 4);
    let config: ConfigResponse = router
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAggregatorConfig {})
        .unwrap();
    assert_eq!(config.validator, new_validator);
    // setting the same validator twice should not have attributes
    let res = router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();
    let attributes = res.events.last().unwrap().attributes.clone();
    assert_eq!(attributes.len(), 1);
    // should only be usable by owner
    let res = router.execute_contract(Addr::unchecked("Ned"), contract.clone(), &msg, &[]);
    assert_eq!(res.unwrap_err(), ContractError::NotOwner {}.to_string());
}

#[test]
fn transfer_admin() {
    let oracle = "Oracle";
    let admin = "Admin";
    let pending = "Pending";
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: vec![oracle.into()],
        added_admins: vec![admin.into()],
        min_submissions: 1,
        max_submissions: 1,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::TransferAdmin {
        oracle: oracle.into(),
        new_admin: pending.into(),
    };
    // called by non admin
    let res = router.execute_contract(owner.clone(), contract.clone(), &msg, &[]);
    assert_eq!(res.unwrap_err(), ContractError::NotAdmin {}.to_string());

    let res = router
        .execute_contract(Addr::unchecked(admin), contract.clone(), &msg, &[])
        .unwrap();
    let expected_attributes = vec![
        attr("action", "transfer_admin"),
        attr("oracle", oracle.clone()),
        attr("sender", admin.clone()),
        attr("new_admin", pending.clone()),
    ];
    let attributes = res.events.last().unwrap().attributes.clone();
    assert_eq!(
        attributes.into_iter().skip(1).collect::<Vec<Attribute>>(),
        expected_attributes
    );

    let res: Addr = router
        .wrap()
        .query_wasm_smart(
            contract,
            &QueryMsg::GetAdmin {
                oracle: oracle.into(),
            },
        )
        .unwrap();
    assert_eq!(res, admin);
}

#[test]
fn accept_admin() {
    let oracle = "Oracle";
    let admin = "Admin";
    let pending = "Pending";
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: vec![oracle.into()],
        added_admins: vec![admin.into()],
        min_submissions: 1,
        max_submissions: 1,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::TransferAdmin {
        oracle: oracle.into(),
        new_admin: pending.into(),
    };
    router
        .execute_contract(Addr::unchecked(admin), contract.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::AcceptAdmin {
        oracle: oracle.into(),
    };
    // called by non pending admin
    let res = router.execute_contract(Addr::unchecked(admin), contract.clone(), &msg, &[]);
    assert_eq!(
        res.unwrap_err(),
        ContractError::NotPendingAdmin {}.to_string()
    );

    router
        .execute_contract(Addr::unchecked(pending), contract.clone(), &msg, &[])
        .unwrap();

    let res: Addr = router
        .wrap()
        .query_wasm_smart(
            contract,
            &QueryMsg::GetAdmin {
                oracle: oracle.into(),
            },
        )
        .unwrap();
    assert_eq!(res, pending);
}

#[test]
fn request_new_round() {
    let oracles = personas![Ned, Nelly];
    let (mut router, owner, _link_addr, contract) = default_init();
    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: 1,
        max_submissions: 1,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let submission = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();

    let msg = ExecuteMsg::SetRequesterPermissions {
        requester: "Ned".into(),
        authorized: true,
        delay: 0,
    };
    router
        .execute_contract(owner, contract.clone(), &msg, &[])
        .unwrap();

    let res = router
        .execute_contract(
            Addr::unchecked("Ned"),
            contract.clone(),
            &ExecuteMsg::RequestNewRound {},
            &[],
        )
        .unwrap();
    let round_id: u32 = from_binary(&res.data.unwrap()).unwrap();
    assert_eq!(round_id, 2);
}

#[test]
fn request_new_round_with_restart_delay() {
    let start_round = 1;

    let oracles = personas![Ned, Carol, Eddy];
    let (mut router, owner, _link_addr, contract) = default_init();
    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: 1,
        max_submissions: 1,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let submission = ExecuteMsg::Submit {
        round_id: start_round,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();

    let msg = ExecuteMsg::SetRequesterPermissions {
        requester: "Eddy".into(),
        authorized: true,
        delay: 1,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();
    let msg = ExecuteMsg::SetRequesterPermissions {
        requester: "Carol".into(),
        authorized: true,
        delay: 0,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    router
        .execute_contract(
            Addr::unchecked("Eddy"),
            contract.clone(),
            &ExecuteMsg::RequestNewRound {},
            &[],
        )
        .unwrap();

    let submission = ExecuteMsg::Submit {
        round_id: start_round + 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();
    // delay still remaining
    let res = router.execute_contract(
        Addr::unchecked("Eddy"),
        contract.clone(),
        &ExecuteMsg::RequestNewRound {},
        &[],
    );
    assert_eq!(
        res.unwrap_err(),
        ContractError::DelayNotRespected {}.to_string()
    );

    // advance round
    router
        .execute_contract(
            Addr::unchecked("Carol"),
            contract.clone(),
            &ExecuteMsg::RequestNewRound {},
            &[],
        )
        .unwrap();
    let submission = ExecuteMsg::Submit {
        round_id: start_round + 2,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();

    // try again
    let res = router.execute_contract(
        Addr::unchecked("Eddy"),
        contract.clone(),
        &ExecuteMsg::RequestNewRound {},
        &[],
    );
    assert!(res.is_ok());
}

#[test]
fn request_new_round_round_in_progress() {
    let oracles = personas![Ned, Carol];
    let (mut router, owner, _link_addr, contract) = default_init();

    let msg = ExecuteMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    router
        .execute_contract(owner.clone(), contract.clone(), &msg, &[])
        .unwrap();

    let submission = ExecuteMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    router
        .execute_contract(Addr::unchecked("Ned"), contract.clone(), &submission, &[])
        .unwrap();
    let res: RoundDataResponse = router
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::AggregatorQuery(GetLatestRoundData {}),
        )
        .unwrap();
    assert!(res.answer.is_none());

    let msg = ExecuteMsg::SetRequesterPermissions {
        requester: "Ned".into(),
        authorized: true,
        delay: 0,
    };
    router
        .execute_contract(owner, contract.clone(), &msg, &[])
        .unwrap();

    let res = router.execute_contract(
        Addr::unchecked("Ned"),
        contract.clone(),
        &ExecuteMsg::RequestNewRound {},
        &[],
    );
    assert_eq!(
        res.unwrap_err(),
        ContractError::NotSupersedable {}.to_string()
    );
}
