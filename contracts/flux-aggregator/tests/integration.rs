mod helpers;

use cosmwasm_std::{
    from_binary, CosmosMsg, Env, HandleResponse, HumanAddr, InitResponse, StdResult, Uint128,
    WasmMsg,
};
use cosmwasm_vm::{
    testing::{handle, init, mock_env, query, MockApi, MockStorage},
    Instance,
};

use flux_aggregator::msg::{HandleMsg, InitMsg, QueryMsg};
use helpers::{mock_dependencies_with_custom_querier, CustomQuerier};

static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/flux_aggregator.wasm");

static MIN_ANS: u32 = 1;
static MAX_ANS: u32 = 1;
static RESTART_DELAY: u32 = 0;
static PAYMENT_AMOUNT: Uint128 = Uint128(3);
static DEPOSIT: Uint128 = Uint128(100);
static ANSWER: Uint128 = Uint128(100);

#[test]
fn test_init() {
    // Successfull init should not panic
    let _ = default_init();
}

#[test]
fn test_submit() {
    let oracles = personas![Neil, Ned, Nelly];
    let (mut deps, _) = init_with_oracles(oracles);

    let allocated = q!(deps, GetAllocatedFunds => Uint128);
    assert_eq!(allocated, Uint128::zero());

    let env = mock_env("Neil", &[]);

    let msg = HandleMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), msg).unwrap();

    let allocated = q!(deps, GetAllocatedFunds => Uint128);
    assert_eq!(allocated, PAYMENT_AMOUNT);

    let expected = (DEPOSIT - PAYMENT_AMOUNT).unwrap();
    let available = q!(deps, GetAvailableFunds => Uint128);
    assert_eq!(expected, available);
}

#[test]
fn test_withdraw_funds_success() {
    let (mut deps, env) = default_init();

    let amount = Uint128(85);
    let recipient_addr = HumanAddr::from("recipient");
    let withdraw = HandleMsg::WithdrawFunds {
        recipient: recipient_addr,
        amount,
    };

    let res: HandleResponse = handle(&mut deps, env.clone(), withdraw).unwrap();
    assert!(res.messages.len() == 2);
    let cosmos_msg = res.messages.get(1).unwrap();
    if let CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: _,
        msg,
        send: _,
    }) = cosmos_msg
    {
        let msg = from_binary::<HandleMsg>(msg).unwrap();
        assert!(msg == HandleMsg::UpdateAvailableFunds {});

        deps.with_querier(|querier| Ok(querier.decrease_balance(amount.u128())))
            .unwrap();
        let _: HandleResponse = handle(&mut deps, env, msg).unwrap();

        let funds_query = QueryMsg::GetAvailableFunds {};
        let res: StdResult<Uint128> = from_binary(&query(&mut deps, funds_query).unwrap()).unwrap();
        assert_eq!(res.unwrap(), Uint128(15));
    }
}

#[test]
fn test_add_oracles() {
    let (mut deps, env) = default_init();

    let old_count = q!(deps, GetOracleCount => u8);
    assert_eq!(old_count, 0);

    let oracle_to_add = HumanAddr::from("oracle");

    let msg = HandleMsg::ChangeOracles {
        removed: vec![],
        added: vec![oracle_to_add.clone()],
        added_admins: vec![oracle_to_add],
        min_submissions: MIN_ANS,
        max_submissions: MAX_ANS,
        restart_delay: RESTART_DELAY,
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), msg).unwrap();

    let new_count = q!(deps, GetOracleCount => u8);
    assert_eq!(new_count, 1);
}

#[test]
fn test_remove_oracles() {
    let (mut deps, env) = default_init();

    let oracle = HumanAddr::from("oracle");
    let oracle_to_remove = HumanAddr::from("oracle_to_remove");
    let oracles = vec![oracle.clone(), oracle_to_remove.clone()];
    let msg = HandleMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles.clone(),
        min_submissions: oracles.len() as u32,
        max_submissions: oracles.len() as u32,
        restart_delay: RESTART_DELAY,
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), msg).unwrap();

    let old_count = q!(deps, GetOracleCount => u8);
    assert_eq!(old_count, 2);

    let msg = HandleMsg::ChangeOracles {
        removed: vec![oracle_to_remove],
        added: vec![],
        added_admins: vec![],
        min_submissions: MIN_ANS,
        max_submissions: MAX_ANS,
        restart_delay: RESTART_DELAY,
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), msg).unwrap();

    let new_count = q!(deps, GetOracleCount => u8);
    assert_eq!(new_count, 1);

    let remaining_oracles = q!(deps, GetOracles => Vec<HumanAddr>);
    assert_eq!(vec![oracle], remaining_oracles);
}

fn default_init() -> (Instance<MockStorage, MockApi, CustomQuerier>, Env) {
    let link_addr = HumanAddr::from("link");
    let validator_addr = HumanAddr::from("validator");
    let msg = InitMsg {
        link: link_addr,
        payment_amount: PAYMENT_AMOUNT,
        timeout: 1800,
        validator: validator_addr,
        min_submission_value: Uint128(1),
        max_submission_value: Uint128(10000000),
        decimals: 18,
        description: "LINK/USD".to_string(),
    };

    // stub gives us defaults. Consume it and override...
    let custom = mock_dependencies_with_custom_querier(DEPOSIT.u128());
    let mut deps = Instance::from_code(WASM, custom, 10000000).unwrap();
    let env = mock_env("aggregator", &[]);
    let _: InitResponse = init(&mut deps, env.clone(), msg).unwrap();

    let _: HandleResponse =
        handle(&mut deps, env.clone(), HandleMsg::UpdateAvailableFunds {}).unwrap();

    (deps, env)
}

fn init_with_oracles(
    oracles: Vec<HumanAddr>,
) -> (Instance<MockStorage, MockApi, CustomQuerier>, Env) {
    let (mut deps, env) = default_init();

    let min_max = oracles.len() as u32;

    let msg = HandleMsg::ChangeOracles {
        removed: vec![],
        added: oracles.clone(),
        added_admins: oracles,
        min_submissions: min_max,
        max_submissions: min_max,
        restart_delay: RESTART_DELAY,
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), msg).unwrap();

    (deps, env)
}
