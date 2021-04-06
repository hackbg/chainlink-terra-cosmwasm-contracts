use cosmwasm_std::{coins, HumanAddr, InitResponse, Uint128};
use cosmwasm_vm::testing::{init, mock_env, mock_instance};
use flux_aggregator::msg::InitMsg;

static WASM: &[u8] = include_bytes!("../../../artifacts/link_token.wasm");

#[test]
fn test_init() {
    let mut deps = mock_instance(WASM, &[]);
    assert_eq!(deps.required_features.len(), 0);

    let link_addr = HumanAddr::from("link");
    let validator_addr = HumanAddr::from("validator");

    let msg = InitMsg {
        link: link_addr,
        payment_amount: Uint128(128),
        timeout: 1800,
        validator: validator_addr,
        min_submission_value: 1.to_string(),
        max_submission_value: 10000000.to_string(),
        decimals: 18,
        description: "LINK/USD".to_string(),
    };

    let env = mock_env("creator", &coins(1000, "earth"));
    let _: InitResponse = init(&mut deps, env.clone(), msg).unwrap();
}
