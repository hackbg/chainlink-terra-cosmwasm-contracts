use chainlink_client::{
    msg::InitMsg,
    state::{State, CONFIG_KEY},
};
use cosmwasm_std::{coins, InitResponse};
use cosmwasm_storage::to_length_prefixed;
use cosmwasm_vm::{
    from_slice,
    testing::{init, mock_env, mock_instance},
    Api, Storage,
};

static WASM: &[u8] = include_bytes!("../../../artifacts/chainlink_client.wasm");

#[test]
fn test_init() {
    let mut deps = mock_instance(WASM, &[]);
    assert_eq!(deps.required_features.len(), 0);

    let env = mock_env("creator", &coins(1000, "earth"));

    let expected_state = State {
        count: 4,
        owner: deps.api.canonical_address(&env.message.sender).0.unwrap(),
    };

    let init_msg = InitMsg { count: 4 };
    let _: InitResponse = init(&mut deps, env, init_msg).unwrap();

    let state: State = deps
        .with_storage(|storage| {
            println!("{:?}", *storage);
            let data = storage
                .get(&to_length_prefixed(CONFIG_KEY))
                .0
                .expect("error reading db")
                .expect("no data stored");
            from_slice(&data)
        })
        .unwrap();
    assert_eq!(state, expected_state);
}
