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
    /// Transfer LINK from oracle to another address. Callable only by oracle's admin
    WithdrawPayment {
        /// Oracle whose LINK is to be transferred
        oracle: HumanAddr,
        /// Transfer recipient
        recipient: HumanAddr,
        /// Amount of LINK to be send
        amount: Uint128, // uint256
    },
    /// Transfers the contract owner's LINK to another address
    WithdrawFunds {
        /// Recipient address
        recipient: HumanAddr,
        /// LINK to be sent
        amount: Uint128, // uint256
    },
    /// Transfer admin address for an oracle
    TransferAdmin {
        /// The oracle adddress whose admin is being transferred
        oracle: HumanAddr,
        /// New admin address
        new_admin: HumanAddr,
    },
    /// Accept the pending admin transfer for an oracle
    AcceptAdmin {
        /// Address of the oracle whose admin is being transfered
        oracle: HumanAddr,
    },
    /// Allows non-oracles to request a new round.
    /// Response contains the new `round_id` ([`u32`]).
    RequestNewRound {},
    /// Allows/disallows non-oracles to start new rounds. Callable only by contract owner
    SetRequesterPermissions {
        requester: HumanAddr,
        authorized: bool,
        delay: u32,
    },
    /// Update the round and payment related parameters for subsequent rounds
    UpdateFutureRounds {
        payment_amount: Uint128,
        min_submissions: u32,
        max_submissions: u32,
        restart_delay: u32,
        timeout: u32,
    },
    /// Recalculate available LINK for payouts
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
    /// Returns the settings of the flux aggregator
    /// Response: [`ConfigResponse`]
    GetAggregatorConfig {},
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
pub struct ConfigResponse {
    pub link: HumanAddr,
    pub validator: HumanAddr,
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
