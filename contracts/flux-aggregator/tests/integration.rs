mod helpers;

use cosmwasm_std::{
    from_binary, CosmosMsg, Env, HandleResponse, HumanAddr, InitResponse, StdResult, Uint128,
    WasmMsg,
};
use cosmwasm_vm::{
    testing::{handle, init, mock_env, query, MockApi, MockStorage},
    Instance,
};

use flux_aggregator::{
    error::ContractErr,
    msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg, RoundDataResponse},
};
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
fn successful_init() {
    // Successfull init should not panic
    let _ = default_init();
}

#[test]
fn submit_funds() {
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
    assert_eq!(available, expected);
}

#[test]
fn submit_under_min_reported() {
    let oracles = personas![Neil, Ned, Nelly];
    let (mut deps, _) = init_with_oracles(oracles.clone());

    let allocated = q!(deps, GetAllocatedFunds => Uint128);
    assert_eq!(allocated, Uint128::zero());

    let submission = HandleMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    let env = mock_env("Neil", &[]);
    let _: HandleResponse = handle(&mut deps, env.clone(), submission.clone()).unwrap();

    let withdrawable = q!(deps,  GetWithdrawablePayment oracle: personas!(Neil) => Uint128);
    assert_eq!(withdrawable, PAYMENT_AMOUNT);

    let withdrawable = q!(deps, GetWithdrawablePayment oracle: personas!(Ned) => Uint128);
    assert_eq!(withdrawable, Uint128::zero());

    let withdrawable = q!(deps, GetWithdrawablePayment oracle: personas!(Nelly) => Uint128);
    assert_eq!(withdrawable, Uint128::zero());

    // answer should not be updated
    let (mut deps, _) = init_with_oracles(oracles);

    let round = q!(deps, GetLatestRoundData => RoundDataResponse);
    assert!(round.answer.is_none());

    let env = mock_env("Ned", &[]);
    let _: HandleResponse = handle(&mut deps, env.clone(), submission.clone()).unwrap();

    let env = mock_env("Nelly", &[]);
    let _: HandleResponse = handle(&mut deps, env.clone(), submission).unwrap();

    let round = q!(deps, GetLatestRoundData => RoundDataResponse);
    assert!(round.answer.is_none());
}

#[test]
fn submit_complete_round() {
    let oracles = personas![Ned, Nelly];
    let (mut deps, _) = init_with_oracles(oracles.clone());

    let round = q!(deps, GetLatestRoundData => RoundDataResponse);
    assert!(round.answer.is_none());

    let env = mock_env("Ned", &[]);
    let _: HandleResponse = handle(
        &mut deps,
        env.clone(),
        HandleMsg::Submit {
            round_id: 1,
            submission: Uint128(100),
        },
    )
    .unwrap();

    let round = q!(deps, GetLatestRoundData => RoundDataResponse);
    assert!(round.answer.is_none());

    let env = mock_env("Nelly", &[]);
    let _: HandleResponse = handle(
        &mut deps,
        env.clone(),
        HandleMsg::Submit {
            round_id: 1,
            submission: Uint128(200),
        },
    )
    .unwrap();

    let round = q!(deps, GetLatestRoundData => RoundDataResponse);
    assert!(round.updated_at.is_some());
    assert_eq!(round.answer, Some(Uint128(150))); // (100 + 200) / 2
}

#[test]
fn submit_twice() {
    let oracles = personas![Ned, Nelly];
    let (mut deps, _) = init_with_oracles(oracles.clone());
    let env = mock_env("Ned", &[]);
    let submission = HandleMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };

    let _: HandleResponse = handle(&mut deps, env.clone(), &submission).unwrap();
    let res: StdResult<HandleResponse> = handle(&mut deps, env.clone(), &submission);
    assert_eq!(res.unwrap_err(), ContractErr::ReportingPreviousRound.std());
}

#[test]
fn withdraw_funds_success() {
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
fn add_oracles() {
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
fn remove_oracles() {
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

#[test]
fn set_requester_permissions() {
    let (mut deps, _) = init_with_oracles(vec![personas!(Ned)]);

    let env = mock_env("Ned", &[]);
    let submission = HandleMsg::Submit {
        round_id: 1,
        submission: ANSWER,
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), &submission).unwrap();
    // attempt without permissions
    let res: StdResult<HandleResponse> = handle(&mut deps, env, HandleMsg::RequestNewRound {});
    assert_eq!(res.unwrap_err(), ContractErr::Unauthorized.std());

    let env = mock_env("owner", &[]);
    let msg = HandleMsg::SetRequesterPermissions {
        requester: personas!(Ned),
        authorized: true,
        delay: 0,
    };
    let _: HandleResponse = handle(&mut deps, env, &msg).unwrap();
    // attempt with permissions
    let env = mock_env("Ned", &[]);
    let res: HandleResponse = handle(&mut deps, env, HandleMsg::RequestNewRound {}).unwrap();
    let round_id: u32 = from_binary(&res.data.unwrap()).unwrap();
    assert_eq!(round_id, 2);

    // set the same permission twice
    // should not panic
    let env = mock_env("owner", &[]);
    let _: HandleResponse = handle(&mut deps, env, &msg).unwrap();
}

#[test]
fn set_validator() {
    let (mut deps, env) = default_init();

    let new_validator = HumanAddr::from("new_validator");
    let msg = HandleMsg::SetValidator {
        validator: new_validator.clone(),
    };
    let res: HandleResponse = handle(&mut deps, env.clone(), &msg).unwrap();
    // successful change should return logs
    assert!(res.log.len() == 3);
    let validator = q!(deps, GetAggregatorConfig => ConfigResponse).validator;
    assert_eq!(validator, new_validator);
    // setting the same validator twice should not logs
    let res: HandleResponse = handle(&mut deps, env, &msg).unwrap();
    assert!(res.log.is_empty());
    // should only be usable by owner
    let env = mock_env("Ned", &[]);
    let res: StdResult<HandleResponse> = handle(&mut deps, env, &msg);
    assert_eq!(res.unwrap_err(), ContractErr::NotOwner.std());
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

    let custom = mock_dependencies_with_custom_querier(DEPOSIT.u128());
    let mut deps = Instance::from_code(WASM, custom, 10000000).unwrap();
    let env = mock_env("owner", &[]);
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
    let res: HandleResponse = handle(&mut deps, env.clone(), msg).unwrap();

    if let CosmosMsg::Wasm(msg) = res.messages.get(0).unwrap() {
        if let WasmMsg::Execute { msg, .. } = msg {
            let _res: HandleResponse = handle(
                &mut deps,
                env.clone(),
                &from_binary::<HandleMsg>(msg).unwrap(),
            )
            .unwrap();

            // println!("{:?}", res.log);

            return (deps, env);
        }
        panic!()
    }
    panic!()
}
