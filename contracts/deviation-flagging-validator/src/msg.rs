use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub flags: HumanAddr,
    pub flagging_threshold: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    SetFlaggingThreshold {
        threshold: u32,
    },
    SetFlagsAddress {
        flags: HumanAddr,
    },
    Validate {
        previous_round_id: Uint128,
        previous_answer: Uint128,
        round_id: Uint128,
        answer: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    IsValid {
        previous_answer: Uint128,
        answer: Uint128,
    },
}
