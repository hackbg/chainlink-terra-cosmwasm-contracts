mod helpers;

use aggregator_proxy::msg::{HandleMsg, InitMsg, PhaseAggregators, QueryMsg};
use cosmwasm_std::{from_binary, Env, HandleResponse, HumanAddr, InitResponse, StdResult};
use cosmwasm_vm::{
    testing::{handle, init, mock_env, query, MockApi, MockStorage},
    Instance,
};
use helpers::{mock_dependencies_with_custom_querier, CustomQuerier};

static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/aggregator_proxy.wasm");

static AGGREGATOR: &str = "aggregator";
static OWNER: &str = "owner";

#[test]
fn successful_init() {
    let (mut deps, _) = init_contract();
    assert_eq!(deps.required_features.len(), 0);

    let phase_id = q!(deps, GetPhaseId => u16);
    assert!(phase_id == 1);

    let aggregators = q!(deps, GetPhaseAggregators => PhaseAggregators);
    let aggregator = aggregators.iter().find(|(id, _)| *id == 1);
    assert_eq!(aggregator.unwrap().1, HumanAddr::from(AGGREGATOR));
}

#[test]
fn propose_aggregator() {
    let new_aggregator = HumanAddr::from("new aggregator");

    let (mut deps, _) = init_contract();

    let msg = HandleMsg::ProposeAggregator {
        aggregator: new_aggregator.clone(),
    };

    let env = mock_env("not owner", &[]);
    let res: StdResult<HandleResponse> = handle(&mut deps, env, &msg);
    assert!(res.is_err());

    let env = mock_env(OWNER, &[]);
    let _: HandleResponse = handle(&mut deps, env, &msg).unwrap();

    let aggregator = q!(deps, GetProposedAggregator => HumanAddr);
    assert_eq!(aggregator, new_aggregator);
}

#[test]
fn confirm_aggregator() {
    let new_aggregator = HumanAddr::from("new aggregator");

    let (mut deps, env) = init_contract();

    let msg = HandleMsg::ProposeAggregator {
        aggregator: new_aggregator.clone(),
    };
    let _: HandleResponse = handle(&mut deps, env.clone(), &msg).unwrap();
    let aggregator = q!(deps, GetProposedAggregator => HumanAddr);
    assert_eq!(aggregator, new_aggregator);

    let msg = HandleMsg::ConfirmAggregator {
        aggregator: new_aggregator.clone(),
    };

    let env = mock_env("not owner", &[]);
    let res: StdResult<HandleResponse> = handle(&mut deps, env, &msg);
    assert!(res.is_err());

    let env = mock_env(OWNER, &[]);
    let _: HandleResponse = handle(&mut deps, env, &msg).unwrap();

    let aggregator = q!(deps, GetAggregator => HumanAddr);
    assert_eq!(aggregator, new_aggregator);

    let phase_id = q!(deps, GetPhaseId => u16);
    assert_eq!(phase_id, 2);

    let aggregators = q!(deps, GetPhaseAggregators => PhaseAggregators);
    let aggregator = aggregators.iter().find(|(id, _)| *id == 2);
    assert_eq!(aggregator.unwrap().1, new_aggregator);
}

fn init_contract() -> (Instance<MockStorage, MockApi, CustomQuerier>, Env) {
    let msg = InitMsg {
        aggregator: HumanAddr::from(AGGREGATOR),
    };
    let custom = mock_dependencies_with_custom_querier();
    let mut deps = Instance::from_code(WASM, custom, 10000000).unwrap();
    let env = mock_env(OWNER, &[]);
    let _: InitResponse = init(&mut deps, env.clone(), msg).unwrap();

    (deps, env)
}
