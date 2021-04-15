use cosmwasm_std::{coins, testing::mock_env, InitResponse};

use cosmwasm_vm::testing::{init, mock_instance};
use owned::msg::InitMsg;

static WASM: &[u8] = include_bytes!("../../../artifacts/owned.wasm");

#[test]
fn proper_initialization() {
    let mut deps = mock_instance(WASM, &[]);
    assert_eq!(deps.required_features.len(), 0);

    let msg = InitMsg {};

    let env = mock_env("creator", &coins(1000, "earth"));

    // we can just call .unwrap() to assert this was a success
    let _: InitResponse = init(&mut deps, env, msg).unwrap();
}
