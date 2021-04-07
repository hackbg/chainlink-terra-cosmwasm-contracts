use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use chainlink_contract_utils::modifier::Immutable;
use cosmwasm_std::{CanonicalAddr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static OWNER_KEY: &[u8] = b"owner";
pub static CONFIG_KEY: &[u8] = b"config";
pub static PREFIX_ORACLES: &[u8] = b"oracles";
pub static ORACLE_ADDRESSES_KEY: &[u8] = b"oracle_addr";
pub static RECORDED_FUNDS_KEY: &[u8] = b"recorded_funds";
pub static PREFIX_REQUESTERS: &[u8] = b"requesters";
pub static PREFIX_ROUND: &[u8] = b"round";
pub static LATEST_ROUND_ID_KEY: &[u8] = b"latest_round_id";
pub static REPORTING_ROUND_ID_KEY: &[u8] = b"reporting_round_id";

pub fn owner<S: Storage>(storage: &mut S) -> Singleton<S, CanonicalAddr> {
    singleton(storage, OWNER_KEY)
}

pub fn owner_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, CanonicalAddr> {
    singleton_read(storage, OWNER_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub link: CanonicalAddr,
    pub validator: CanonicalAddr,

    pub payment_amount: Uint128,
    pub max_submission_count: u32,
    pub min_submission_count: u32,
    pub restart_delay: u32,
    pub timeout: u32,
    pub decimals: u8,
    pub description: String,

    min_submission_value: Immutable<Uint128>,
    max_submission_value: Immutable<Uint128>,
}

impl State {
    #[allow(clippy::clippy::too_many_arguments)]
    pub fn new(
        link: CanonicalAddr,
        validator: CanonicalAddr,
        payment_amount: Uint128,
        max_submission_count: u32,
        min_submission_count: u32,
        restart_delay: u32,
        timeout: u32,
        decimals: u8,
        description: String,
        min_submission_value: Uint128, // int256
        max_submission_value: Uint128, // int256
    ) -> Self {
        Self {
            link,
            validator,
            payment_amount,
            max_submission_count,
            min_submission_count,
            restart_delay,
            timeout,
            decimals,
            description,
            min_submission_value: Immutable::new(min_submission_value),
            max_submission_value: Immutable::new(max_submission_value),
        }
    }
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleStatus {
    pub withdrawable: Uint128,
    pub starting_round: u32,
    pub ending_round: u32,
    pub last_reported_round: u32,
    pub last_started_round: u32,
    pub latest_submission: Uint128, // int256
    pub index: u16,
    pub admin: CanonicalAddr,
    pub pending_admin: Option<CanonicalAddr>,
}

pub fn oracles<S: Storage>(storage: &mut S) -> Bucket<S, OracleStatus> {
    bucket(&PREFIX_ORACLES, storage)
}

pub fn oracles_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, OracleStatus> {
    bucket_read(&PREFIX_ORACLES, storage)
}

pub fn oracle_addresses<S: Storage>(storage: &mut S) -> Singleton<S, Vec<CanonicalAddr>> {
    singleton(storage, ORACLE_ADDRESSES_KEY)
}

pub fn oracle_addresses_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, Vec<CanonicalAddr>> {
    singleton_read(storage, ORACLE_ADDRESSES_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Round {
    pub answer: Option<Uint128>, // int256,
    pub started_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub answered_in_round: u32,
}

pub fn rounds<S: Storage>(storage: &mut S) -> Bucket<S, Round> {
    bucket(&PREFIX_ROUND, storage)
}

pub fn rounds_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, Round> {
    bucket_read(&PREFIX_ROUND, storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Requester {
    pub authorized: bool,
    pub delay: u32,
    pub last_started_round: u32,
}

pub fn requesters<S: Storage>(storage: &mut S) -> Bucket<S, Requester> {
    bucket(&PREFIX_REQUESTERS, storage)
}

pub fn requesters_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, Requester> {
    bucket_read(&PREFIX_REQUESTERS, storage)
}

pub fn reporting_round_id<S: Storage>(storage: &mut S) -> Singleton<S, u32> {
    singleton(storage, REPORTING_ROUND_ID_KEY)
}

pub fn reporting_round_id_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, u32> {
    singleton_read(storage, REPORTING_ROUND_ID_KEY)
}

pub fn latest_round_id<S: Storage>(storage: &mut S) -> Singleton<S, u32> {
    singleton(storage, LATEST_ROUND_ID_KEY)
}

pub fn latest_round_id_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, u32> {
    singleton_read(storage, LATEST_ROUND_ID_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Funds {
    pub available: Uint128,
    pub allocated: Uint128,
}

impl Default for Funds {
    fn default() -> Self {
        Self {
            available: Uint128::zero(),
            allocated: Uint128::zero(),
        }
    }
}

pub fn recorded_funds<S: Storage>(storage: &mut S) -> Singleton<S, Funds> {
    singleton(storage, RECORDED_FUNDS_KEY)
}

pub fn recorded_funds_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, Funds> {
    singleton_read(storage, RECORDED_FUNDS_KEY)
}
