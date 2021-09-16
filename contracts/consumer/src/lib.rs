#[allow(clippy::needless_question_mark)]
pub use fadroma::terra::*;

pub use aggregator_proxy::msg::QueryMsg as ProxyQuery;
use chainlink_aggregator::{LatestAnswerResponse, QueryMsg::*, RoundDataResponse};

#[macro_use]
extern crate fadroma;

contract!(
    [State]{
        proxy_contract: String
    }

    [Instantiate] (deps, _env, _info, msg: {
        proxy: String
    }) {
        let state = State { proxy_contract: proxy.clone() };
        save_state!(state);
        Response::new().add_events(vec![
            Event::new("consumer instantiated")
            .add_attribute("proxy_contract", format!("{:?}", proxy))
        ])
    }

    [Query] (deps, state, _env, msg) -> Res {
        LatestRoundData () {
            let latest_round: RoundDataResponse = deps.querier.query_wasm_smart(
                state.proxy_contract,
                &ProxyQuery::AggregatorQuery(GetLatestRoundData{}),
            )?;

            Ok(Res::RoundDataResponse {
                round_id: latest_round.round_id,
                answer: latest_round.answer,
                started_at: latest_round.started_at,
                updated_at: latest_round.updated_at,
                answered_in_round: latest_round.answered_in_round
            })
        }

        RoundData (round_id: u32) {
            let round_data: RoundDataResponse = deps.querier.query_wasm_smart(
                state.proxy_contract,
                &ProxyQuery::AggregatorQuery(GetRoundData{ round_id })
            )?;

            Ok(Res::RoundDataResponse {
                round_id: round_data.round_id,
                answer: round_data.answer,
                started_at: round_data.started_at,
                updated_at: round_data.updated_at,
                answered_in_round: round_data.answered_in_round
            })
        }
        LatestAnswer () {
            let latest_answer: LatestAnswerResponse = deps.querier.query_wasm_smart(
                state.proxy_contract,
                &ProxyQuery::AggregatorQuery(GetLatestAnswer{})
            )?;

            Ok(Res::Answer {
                value: latest_answer
            })
        } 
        CurrentAggregator () {
            let current_aggregator: Addr = deps.querier.query_wasm_smart(
                state.proxy_contract,
                &ProxyQuery::GetAggregator{}
            )?;

            Ok(Res::Aggregator {
                address: current_aggregator
            })
        }
    }
    [Res] {
        RoundDataResponse {
            round_id: u32,           // uint80
            answer: Option<Uint128>, // int256
            started_at: Option<u64>, // int256
            updated_at: Option<u64>, // uint256
            answered_in_round: u32
        }

        Answer {value: LatestAnswerResponse}
        Aggregator {address: Addr}
    }

    [Execute] (deps, _env, _info, state, msg) -> Result {
        SwitchProxy (address: String) {
            state.proxy_contract = address.clone();
            Ok(Response::new().add_attribute("new proxy", address))
        }
    }
);
