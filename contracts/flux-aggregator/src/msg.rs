use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// LINK token address
    pub link: String,
    /// Amount of LINK paid to each oracle per submission
    pub payment_amount: Uint128,
    /// The number of seconds after the previous round that are
    /// allowed to lapse before allowing an oracle to skip an unfinished round
    pub timeout: u32,
    /// Address to external data validation
    pub validator: String,
    /// An immutable check for a lower bound of what
    /// submission values are accepted from an oracle
    pub min_submission_value: Uint128, // int256
    /// An immutable check for an upper bound of what
    /// submission values are accepted from an oracle
    pub max_submission_value: Uint128, // int256
    /// The number of decimals to offset the answer by
    pub decimals: u8,
    /// A short description of what is being reported
    pub description: String,
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
        removed: Vec<String>,
        /// Oracles to be added
        added: Vec<String>,
        /// Admins to be added. Only this address is allowed to access the respective oracle's funds
        added_admins: Vec<String>,
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
        oracle: String,
        /// Transfer recipient
        recipient: String,
        /// Amount of LINK to be send
        amount: Uint128, // uint256
    },
    /// Transfers the contract owner's LINK to another address
    WithdrawFunds {
        /// Recipient address
        recipient: String,
        /// LINK to be sent
        amount: Uint128, // uint256
    },
    /// Transfer admin address for an oracle
    TransferAdmin {
        /// The oracle adddress whose admin is being transferred
        oracle: String,
        /// New admin address
        new_admin: String,
    },
    /// Accept the pending admin transfer for an oracle
    AcceptAdmin {
        /// Address of the oracle whose admin is being transfered
        oracle: String,
    },
    /// Allows non-oracles to request a new round.
    /// Response contains the new `round_id` ([`u32`]).
    RequestNewRound {},
    /// Allows/disallows non-oracles to start new rounds. Callable only by contract owner
    SetRequesterPermissions {
        /// Address to set permission for
        requester: String,
        /// Is requester authorized
        authorized: bool,
        /// The number of rounds the requester must wait before starting another round
        delay: u32,
    },
    /// Update the round and payment related parameters for subsequent rounds
    UpdateFutureRounds {
        /// Payment amount for subsequent rounds
        payment_amount: Uint128,
        /// The new minimum submission count for each round
        min_submissions: u32,
        /// The new maximum submission count for each round
        max_submissions: u32,
        /// The number of rounds an Oracle has to wait before they can initiate a round
        restart_delay: u32,
        /// The new timeout to be used for future rounds
        timeout: u32,
    },
    /// Recalculate available LINK for payouts
    UpdateAvailableFunds {},
    /// Updates the address which does external data validation
    SetValidator {
        /// Address of the new validation contract
        validator: String,
    },
    /// Handler for LINK token Send message
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns contract owner's address
    /// Response [`Addr`]
    GetOwner {},
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
    GetWithdrawablePayment {
        /// Address of the Oracle which is query for
        oracle: String,
    },
    /// Query the number of oracles
    /// Response: [`u8`].
    GetOracleCount {},
    /// Query for the addresses of the oracles on the contract
    /// Response: [`Vec<Addr>`].
    GetOracles {},
    /// Get the admin address of a specific Oracle
    /// Response: [`Addr`].
    GetAdmin {
        /// The address of the oracle whose admin is being queried
        oracle: String,
    },
    /// Get status of specific oracle
    /// Response: [`OracleStatus`].
    GetOracleStatus {
        /// Oracle address to look up for
        oracle: String,
    },
    AggregatorQuery(chainlink_aggregator::QueryMsg),
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
