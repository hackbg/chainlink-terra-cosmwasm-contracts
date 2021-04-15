use cosmwasm_std::{
    from_slice, to_binary, Binary, Empty, QuerierResult, QueryRequest, StdResult, Uint128,
    WasmQuery,
};
use cosmwasm_vm::{
    testing::{MockApi, MockQuerier, MockStorage},
    Extern, GasInfo, Querier,
};
use cw20::BalanceResponse;

pub struct CustomQuerier {
    balance: u128,
    querier: MockQuerier,
}

impl CustomQuerier {
    pub fn new(balance: u128) -> Self {
        Self {
            balance,
            querier: MockQuerier::new(&[]),
        }
    }

    pub fn decrease_balance(&mut self, balance: u128) {
        self.balance -= balance;
    }

    pub fn increase_balance(&mut self, balance: u128) {
        self.balance += balance;
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
                WasmQuery::Smart { contract_addr, .. } => match contract_addr.as_str() {
                    "link" => (
                        Ok(QuerierResult::Ok(to_binary(&BalanceResponse {
                            balance: Uint128(self.balance),
                        }))),
                        GasInfo::with_externally_used(20),
                    ),
                    _ => self.querier.query_raw(request, gas_limit),
                },
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }
}

pub fn mock_dependencies_with_custom_querier(
    balance: u128,
) -> Extern<MockStorage, MockApi, CustomQuerier> {
    Extern {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: CustomQuerier::new(balance),
    }
}

#[macro_export]
macro_rules! personas {
    [$($name:ident), *] => {
        vec![$(HumanAddr::from(stringify!($name))), *]
    };
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
