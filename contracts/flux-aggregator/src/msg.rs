use cosmwasm_std::{HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// LINK token address
    pub link: HumanAddr,
    pub payment_amount: Uint128,
    pub timeout: u32,
    /// Address to external data validation
    pub validator: HumanAddr,
    pub min_submission_value: Uint128, // int256
    pub max_submission_value: Uint128, // int256
    pub decimals: u8,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Invoked by oracles when they have witnessed a need to update
    Submit {
        /// ID of the round this submission pertains to
        round_id: u32, // uint256
        /// The updated data that the oracle is submitting
        submission: Uint128, // int256
    },
    /// Invoked by the owner to remove and add new oracles as well as
    /// update the round related parameters that pertain to total oracle count
    ChangeOracles {
        /// Oracles to be removed
        removed: Vec<HumanAddr>,
        /// Oracles to be added
        added: Vec<HumanAddr>,
        /// Admins to be added. Only this address is allowed to access the respective oracle's funds
        added_admins: Vec<HumanAddr>,
        /// The new minimum submission count for each round
        min_submissions: u32,
        /// The new maximum submission count for each round
        max_submissions: u32,
        /// The number of rounds an Oracle has to wait before they can initiate a round
        restart_delay: u32,
    },
    WithdrawPayment {
        oracle: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128, // uint256
    },
    WithdrawFunds {
        recipient: HumanAddr,
        amount: Uint128, // uint256
    },
    TransferAdmin {
        oracle: HumanAddr,
        new_admin: HumanAddr,
    },
    AcceptAdmin {
        oracle: HumanAddr,
    },
    /// Allows non-oracles to request a new round.
    /// Response contains the new `round_id` ([`u32`]).
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
    /// Updates the address which does external data validation
    SetValidator {
        /// Address of the new validation contract
        validator: HumanAddr,
    },
    /// Handler for LINK token Send message
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the amount of payment yet to be withdrawn by oracles.
    /// Response: [`Uint128`].
    GetAllocatedFunds {},
    /// Get the amount of future funding available to oracles.
    /// Response: [`Uint128`].
    GetAvailableFunds {},
    /// Query the available amount of LINK for an oracle to withdraw.
    /// Response: [`Uint128`].
    GetWithdrawablePayment { oracle: HumanAddr },
    /// Response: [`u8`].
    GetOracleCount {},
    /// Response: [`Vec<HumanAddr>`].
    GetOracles {},
    /// Response: [`HumanAddr`].
    GetAdmin {
        /// The address of the oracle whose admin is being queried
        oracle: HumanAddr,
    },
    /// Response: [`RoundDataResponse`].
    GetRoundData {
        /// The round ID to retrieve the round data for
        round_id: u32,
    },
    /// Response: [`RoundDataResponse`].
    GetLatestRoundData {},
    /// Response: [`OracleRoundStateResponse`].
    GetOracleRoundState {
        oracle: HumanAddr,
        queried_round_id: u32,
        timestamp: u64,
    },
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
pub struct OracleRoundStateResponse {
    pub elegible_to_submit: bool,           // bool
    pub round_id: u32,                      // uint32
    pub latest_submission: Option<Uint128>, // int256
    pub started_at: u64,                    // uint64
    pub timeout: u32,                       // uint64
    pub available_funds: Uint128,           // uint128
    pub oracle_count: u8,                   // uint8
    pub payment_amount: Uint128,            // uint128
}
