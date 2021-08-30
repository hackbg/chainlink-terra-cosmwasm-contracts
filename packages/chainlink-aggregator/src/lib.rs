use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query data for a specific round
    /// Response: [`RoundDataResponse`].
    GetRoundData {
        /// The round ID to retrieve the round data for
        round_id: u32,
    },
    /// Query data for the latest round
    /// Response: [`RoundDataResponse`].
    GetLatestRoundData {},

    GetDecimals {},

    GetDescription {},

    GetVersion {},

    GetLatestAnswer {},
}

impl QueryMsg {
    pub fn wrap(self) -> AggregatorQuery {
        AggregatorQuery::new(self)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AggregatorQuery {
    aggregator_query: QueryMsg,
}

impl AggregatorQuery {
    pub fn new(msg: QueryMsg) -> Self {
        Self {
            aggregator_query: msg,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundDataResponse {
    pub round_id: u32,           // uint80
    pub answer: Option<Uint128>, // int256
    pub started_at: Option<u64>, // int256
    pub updated_at: Option<u64>, // uint256
    pub answered_in_round: u32,  // uint80
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LatestAnswerResponse(pub Option<Uint128>);
