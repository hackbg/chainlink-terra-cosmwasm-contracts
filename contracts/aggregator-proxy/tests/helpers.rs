use aggregator_proxy::msg::QueryMsg;
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, Empty, QuerierResult, QueryRequest, StdResult,
    Uint128, WasmQuery,
};
use cosmwasm_vm::{
    testing::{MockApi, MockQuerier, MockStorage},
    Extern, GasInfo, Querier,
};
use flux_aggregator::msg::QueryMsg as AggregatorQuery;

pub struct CustomQuerier {
    querier: MockQuerier,
}

impl CustomQuerier {
    pub fn new() -> Self {
        Self {
            querier: MockQuerier::new(&[]),
        }
    }
}

impl Querier for CustomQuerier {
    fn query_raw(
        &self,
        request: &[u8],
        gas_limit: u64,
    ) -> cosmwasm_vm::FfiResult<cosmwasm_std::SystemResult<StdResult<Binary>>> {
        let parsed_request: QueryRequest<Empty> = from_slice(request).unwrap();
        match &parsed_request {
            QueryRequest::Wasm(msg) => match msg {
                WasmQuery::Smart { contract_addr, msg } => match from_binary(&msg).unwrap() {
                    AggregatorQuery::GetAggregatorConfig {} => todo!(),
                    AggregatorQuery::GetRoundData { round_id } => todo!(),
                    AggregatorQuery::GetLatestRoundData {} => todo!(),
                    _ => self.querier.query_raw(request, gas_limit),
                },
                // match contract_addr.as_str() {
                //     "link" => (
                //         Ok(QuerierResult::Ok(to_binary(&"ph"))),
                //         GasInfo::with_externally_used(20),
                //     ),
                //     _ => self.querier.query_raw(request, gas_limit),
                // },
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }
}

pub fn mock_dependencies_with_custom_querier() -> Extern<MockStorage, MockApi, CustomQuerier> {
    Extern {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: CustomQuerier::new(),
    }
}

#[macro_export]
macro_rules! q {
    ($deps:ident, $query_type:ident $($prop:ident : $val:expr), * => $ret:ty) => {{
        let msg = (QueryMsg::$query_type{
            $(
                $prop: $val,
            )*
        });
        let res = query(&mut $deps, msg).unwrap();
        let parsed: StdResult<$ret> = from_binary(&res).unwrap();
        parsed.unwrap()
    }};
}
