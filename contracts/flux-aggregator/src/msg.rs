use cosmwasm_std::{HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub link: HumanAddr,
    pub payment_amount: Uint128,
    pub timeout: u32,
    pub validator: HumanAddr,
    pub min_submission_value: String, // BigInt
    pub max_submission_value: String, // BigInt
    pub decimals: u8,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Submit {
        round_id: Uint128,
        submission: i128,
    },
    ChangeOracles {
        removed: Vec<HumanAddr>,
        added: Vec<HumanAddr>,
        added_admins: Vec<HumanAddr>,
        min_submissions: u32,
        max_submissions: u32,
        restart_delay: u32,
    },
    WithdrawPayment {
        oracle: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
    },
    WithdrawFunds {
        recipient: HumanAddr,
        amount: Uint128,
    },
    TransferAdmin {
        oracle: HumanAddr,
        new_admin: HumanAddr,
    },
    AcceptAdmin {
        oracle: HumanAddr,
    },
    RequestNewRound {},
    SetRequesterPermissions {
        requester: HumanAddr,
        authorized: bool,
        delay: u32,
    },
    UpdateFutureRounds {
        payment_amount: Uint128,
        min_submissions: u32,
        max_submissions: u32,
        restart_delay: u32,
        timeout: u32,
    },
    UpdateAvailableFunds {},
    SetValidator {
        validator: HumanAddr,
    },
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAllocatedFunds {},
    GetAvailableFunds {},
    GetWithdrawablePayment {
        oracle: HumanAddr,
    },
    GetOracleCount {},
    GetOracles {},
    GetAdmin {
        oracle: HumanAddr,
    },
    GetRoundData {
        round_id: Uint128,
    },
    GetLatestRoundData {},
    GetOracleRoundState {
        oracle: HumanAddr,
        queried_round_id: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundDataResponse {
    pub round_id: Uint128,          // uint80
    pub answer: i128,               // BigInt
    pub started_at: Uint128,        // BigInt
    pub updated_at: Uint128,        // BigUInt
    pub answered_in_round: Uint128, // uint80
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleRoundStateResponse {
    pub elegible_to_submit: bool, // bool
    pub round_id: u32,            // uint32
    pub latest_submission: i128,  // BigInt
    pub started_at: u64,          // uint64
    pub timeout: u64,             // uint64
    pub available_funds: Uint128, // uint128
    pub oracle_count: u8,         // uint8
    pub payment_amount: Uint128,  // uint128
}
