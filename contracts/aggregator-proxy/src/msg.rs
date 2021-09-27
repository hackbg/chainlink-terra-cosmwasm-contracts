use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type PhaseAggregators = Vec<(u16, Addr)>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub aggregator: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ProposeAggregator { aggregator: String },
    ConfirmAggregator { aggregator: String },
    // owned
    TransferOwnership { to: Addr },
    AcceptOwnership {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetPhaseAggregators {},
    GetProposedRoundData { round_id: u32 },
    GetProposedLatestRoundData {},
    GetProposedAggregator {},
    GetAggregator {},
    GetPhaseId {},
    AggregatorQuery(chainlink_aggregator::QueryMsg),
    // owned
    GetOwner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
