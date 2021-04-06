use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, CosmosMsg, Empty, HandleResponse, HumanAddr,
    InitResponse, QuerierResult, QueryRequest, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cosmwasm_vm::{
    testing::{handle, init, mock_env, mock_instance, query, MockApi, MockStorage},
    Extern, GasInfo, Instance, Querier,
};
use cw20::BalanceResponse;
use flux_aggregator::msg::{HandleMsg, InitMsg, QueryMsg};

static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/flux_aggregator.wasm");

#[test]
fn test_init() {
    let link_addr = HumanAddr::from("link");

    let validator_addr = HumanAddr::from("validator");
    let msg = InitMsg {
        link: link_addr,
        payment_amount: Uint128(128),
        timeout: 1800,
        validator: validator_addr,
        min_submission_value: Uint128(1),
        max_submission_value: Uint128(10000000),
        decimals: 18,
        description: "LINK/USD".to_string(),
    };

    let mut deps = mock_instance(WASM, &[]);
    let env = mock_env("aggregator", &[]);
    let _: InitResponse = init(&mut deps, env.clone(), msg).unwrap();
}

#[test]
fn test_withdraw_funds_success() {
    let link_addr = HumanAddr::from("link");

    let validator_addr = HumanAddr::from("validator");
    let msg = InitMsg {
        link: link_addr,
        payment_amount: Uint128(128),
        timeout: 1800,
        validator: validator_addr,
        min_submission_value: Uint128(1),
        max_submission_value: Uint128(10000000),
        decimals: 18,
        description: "LINK/USD".to_string(),
    };

    // stub gives us defaults. Consume it and override...
    let custom = mock_dependencies_with_custom_querier();
    let mut deps = Instance::from_code(WASM, custom, 10000000).unwrap();
    let env = mock_env("aggregator", &[]);
    let _: InitResponse = init(&mut deps, env.clone(), msg).unwrap();
    let _: HandleResponse =
        handle(&mut deps, env.clone(), HandleMsg::UpdateAvailableFunds {}).unwrap();

    let recipient_addr = HumanAddr::from("recipient");
    let withdraw = HandleMsg::WithdrawFunds {
        recipient: recipient_addr,
        amount: Uint128(1000),
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
        let _: HandleResponse =
            handle(&mut deps, env, from_binary::<HandleMsg>(msg).unwrap()).unwrap();

        let funds_query = QueryMsg::GetAvailableFunds {};
        let res: StdResult<Uint128> = from_binary(&query(&mut deps, funds_query).unwrap()).unwrap();
        assert_eq!(res.unwrap(), Uint128(2000));
    }
    {
        assert!(false);
    }
}

fn mock_dependencies_with_custom_querier() -> Extern<MockStorage, MockApi, MyMockQuerier> {
    Extern {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: MyMockQuerier { balance: 2000 },
    }
}

struct MyMockQuerier {
    pub balance: u128,
}

impl Querier for MyMockQuerier {
    fn query_raw(
        &self,
        request: &[u8],
        _gas_limit: u64,
    ) -> cosmwasm_vm::FfiResult<cosmwasm_std::SystemResult<StdResult<Binary>>> {
        let request: QueryRequest<Empty> = from_slice(request).unwrap();
        match &request {
            QueryRequest::Wasm(msg) => match msg {
                WasmQuery::Smart { contract_addr, .. } => {
                    let link = HumanAddr::from("link");
                    match &contract_addr {
                        link => (
                            Ok(QuerierResult::Ok(to_binary(&BalanceResponse {
                                balance: Uint128(self.balance),
                            }))),
                            GasInfo::with_externally_used(20),
                        ),
                        _ => unimplemented!(),
                    }
                }
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }
}
