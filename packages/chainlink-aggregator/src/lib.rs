use cosmwasm_std::{Addr, Uint128};
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
    /// Returns the settings of the flux aggregator
    /// Response: [`ConfigResponse`]
    GetAggregatorConfig {},
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
pub struct ConfigResponse {
    pub link: Addr,
    pub validator: Addr,
    pub payment_amount: Uint128,
    pub max_submission_count: u32,
    pub min_submission_count: u32,
    pub restart_delay: u32,
    pub timeout: u32,
    pub decimals: u8,
    pub description: String,
    pub min_submission_value: Uint128,
    pub max_submission_value: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundDataResponse {
    pub round_id: u32,           // uint80
    pub answer: Option<Uint128>, // int256
    pub started_at: Option<u64>, // int256
    pub updated_at: Option<u64>, // uint256
    pub answered_in_round: u32,  // uint80
}
