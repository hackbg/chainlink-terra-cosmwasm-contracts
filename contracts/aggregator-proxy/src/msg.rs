use cosmwasm_std::{CanonicalAddr, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub aggregator: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ProposeAggregator { aggregator: HumanAddr },
    ConfirmAggregator { aggregator: HumanAddr },
    // owned
    TransferOwnership { to: CanonicalAddr },
    AcceptOwnership {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetRoundData { round_id: Uint128 },
    GetLatestRoundData {},
    GetProposedRoundData { round_id: Uint128 },
    GetProposedLatestRoundData {},
    GetProposedAggregator {},
    GetPhase {},
    GetDecimals {},
    GetDescription {},
    // owned
    GetOwner {},
}
