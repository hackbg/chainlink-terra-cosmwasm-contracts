use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, U32Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
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
pub struct OracleStatus {
    pub withdrawable: Uint128,
    pub starting_round: u32,
    pub ending_round: u32,
    pub last_reported_round: Option<u32>,
    pub last_started_round: Option<u32>,
    pub latest_submission: Option<Uint128>, // int256
    pub index: u16,
    pub admin: Addr,
    pub pending_admin: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Round {
    pub answer: Option<Uint128>, // int256,
    pub started_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub answered_in_round: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundDetails {
    pub submissions: Vec<Uint128>, // int256[]
    pub max_submissions: u32,
    pub min_submissions: u32,
    pub timeout: u32,
    pub payment_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Requester {
    pub authorized: bool,
    pub delay: u32,
    pub last_started_round: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Funds {
    pub available: Uint128,
    pub allocated: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("aggregator_config");
pub const ORACLES: Map<&Addr, OracleStatus> = Map::new("oracles");
pub const ORACLE_ADDRESSES: Item<Vec<Addr>> = Item::new("oracle_addresses");
pub const DETAILS: Map<U32Key, RoundDetails> = Map::new("details");
pub const ROUNDS: Map<U32Key, Round> = Map::new("rounds");
pub const REQUESTERS: Map<&Addr, Requester> = Map::new("requesters");
pub const REPORTING_ROUND_ID: Item<u32> = Item::new("reporting_round_id");
pub const LATEST_ROUND_ID: Item<u32> = Item::new("latest_round_id");
pub const RECORDED_FUNDS: Item<Funds> = Item::new("recorded_funds");
