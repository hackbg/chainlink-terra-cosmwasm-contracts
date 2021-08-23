use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The address of the flags contract
    pub flags: String,
    /// The threshold that will trigger a flag to be raised
    /// Setting the value of 100,000 is equivalent to tolerating a 100% change
    /// compared to the previous price
    pub flagging_threshold: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Initiate contract ownership transfer to another address.
    /// Can be used only by owner
    TransferOwnership {
        /// Address to transfer ownership to
        to: String,
    },
    /// Finish contract ownership transfer. Can be used only by pending owner
    AcceptOwnership {},
    /// Updates the flagging threshold
    /// Can be used only by owner
    SetFlaggingThreshold { threshold: u32 },
    /// Updates the flagging contract address for raising flags
    /// Can be used only by owner
    SetFlagsAddress { flags: Addr },
    /// Checks whether the parameters count as valid by comparing the difference
    /// change to the flagging threshold
    Validate {
        /// ID of the previous round
        previous_round_id: u32,
        /// Previous answer, used as the median of difference with the current
        /// answer to determine if the deviation threshold has been exceeded
        previous_answer: Uint128,
        /// ID of the current round
        round_id: u32,
        /// Current answer which is compared for a ration of change to make sure
        /// it has not exceeded the flagging threshold
        answer: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Check whether the parameters count is valid by comparing the difference
    /// change to the flagging threshold
    /// Response: [`bool`]
    IsValid {
        /// Previous answer, used as the median of difference with the current
        /// answer to determine if the deviation threshold has been exceeded
        previous_answer: Uint128,
        /// Current answer which is compared for a ration of change to make sure
        /// it has not exceeded the flagging threshold
        answer: Uint128,
    },
    /// Query the flagging threshold
    /// Response: [`u32`]
    GetFlaggingThreshold {},
    /// Returns contract owner's address
    /// Response [`Addr`]
    GetOwner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FlaggingThresholdResponse {
    pub threshold: u32,
}
