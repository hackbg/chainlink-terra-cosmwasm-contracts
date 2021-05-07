use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// The address of the link token
    pub link_token: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Creates the chainlink request, stores the hash of params as the on-chain
    /// commitment for the request.
    OracleRequest {
        /// The sender of the request
        sender: HumanAddr,
        /// The amount of payment given (specified in wei)
        payment: Uint128,
        /// The Job Specification ID
        spec_id: Binary,
        /// The callback address for the response
        callback_address: HumanAddr,
        /// The callback function ID for the response
        callback_function_id: Binary,
        /// The nonce sent by the request
        nonce: Uint128,
        /// The specified data version
        data_version: Uint128,
        /// the CBOR payload of the request
        data: Binary,
    },
    /// Called by the Chainlink node to fulfill requests
    /// Given parameters must hash back to the commitment stored from `oracleRequest`
    /// Will call the callback address` callback function without bubbling up error
    /// checking, so that the node can get paid
    FulfillOracleRequest {
        /// The fulfillment request ID that must match the requester's
        request_id: Binary,
        /// The payment amount that that will be released for the oracle(specified in wei)
        payment: Uint128,
        /// The callback address for fulfillment
        callback_address: HumanAddr,
        /// The callback function ID for fulfillment
        callback_function_id: Binary,
        /// the expiration that the node should respond by before the requester can cancel
        expiration: Uint128,
        /// The data to return to the consuming contract
        data: Binary,
    },
    /// Sets the fulfillment permission for a given node. 'true' to allow, 'false' to disallow
    SetFulfillmentPermission {
        /// The address of the Chainlink node
        node: HumanAddr,
        /// Value to determine if the node can fulfill requests
        allowed: bool,
    },
    /// Allows the node operator to withdraw earned LINK to a given address
    Withdraw {
        /// The address to send the LINK token to
        recipient: HumanAddr,
        /// The amount to send(specified in wei)
        amount: Uint128,
    },
    /// Allows requesters to cancel requests sent to this oracle contract.
    /// Will transfer the LINK sent for the request back to the requester's address
    CancelOracleRequest {
        /// The request ID
        request_id: Binary,
        /// The amount of payment given(specified in wei)
        payment: Uint128,
        nonce: Uint128,
        /// The requester's specified callback address
        callback_func: Binary,
        /// The time of expiration for the request
        expiration: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Check if a node is authorized for fulfilling requests
    GetAuthorizationStatus {
        /// The address of the Chainlink node
        node: HumanAddr,
    },
    /// Displays the amount of LINK that is available for the node operator to withdraw
    Withdrawable {},
    /// Returns address of the LINK token
    GetChainlinkToken {},
}
