use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use chainlink_contract_utils::modifier::Immutable;
use cosmwasm_std::{CanonicalAddr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";
pub static ORACLE_ADDRESSES_KEY: &[u8] = b"oracle_addr";
pub static RECORDED_FUNDS_KEY: &[u8] = b"recorded_funds";
pub static PREFIX_ROUND: &[u8] = b"round";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Immutable<CanonicalAddr>,

    pub link: CanonicalAddr,
    pub validator: CanonicalAddr,

    pub payment_amount: Uint128,
    pub max_submission_count: u32,
    pub min_submission_count: u32,
    pub restart_delay: u32,
    pub timeout: u32,
    pub decimals: u8,
    pub description: String,

    min_submission_value: Immutable<String>,
    max_submission_value: Immutable<String>,
}

impl State {
    #[allow(clippy::clippy::too_many_arguments)]
    pub fn new(
        owner: CanonicalAddr,
        link: CanonicalAddr,
        validator: CanonicalAddr,
        payment_amount: Uint128,
        max_submission_count: u32,
        min_submission_count: u32,
        restart_delay: u32,
        timeout: u32,
        decimals: u8,
        description: String,
        min_submission_value: String,
        max_submission_value: String,
    ) -> Self {
        Self {
            owner: Immutable::new(owner),
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

pub fn oracle_addresses<S: Storage>(storage: &mut S) -> Singleton<S, Vec<CanonicalAddr>> {
    singleton(storage, ORACLE_ADDRESSES_KEY)
}

pub fn oracle_addresses_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, Vec<CanonicalAddr>> {
    singleton_read(storage, ORACLE_ADDRESSES_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Round {
    pub answer: Option<i128>, // int256,
    pub started_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub answered_in_round: u64,
}

pub fn rounds<S: Storage>(storage: &mut S) -> Bucket<S, Round> {
    bucket(&PREFIX_ROUND, storage)
}

pub fn rounds_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, Round> {
    bucket_read(&PREFIX_ROUND, storage)
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
