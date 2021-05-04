use cw0::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, HumanAddr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";
pub static AUTH_NODE_KEY: &[u8] = b"authorized_nodes";
pub static COMMITMENTS: &[u8] = b"commitments";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub link_token: CanonicalAddr,
    pub withdrawable_tokens: u128,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn authorized_nodes<S: Storage>(storage: &mut S) -> Bucket<S, bool> {
    bucket(&AUTH_NODE_KEY, storage)
}

pub fn authorized_nodes_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, bool> {
    bucket_read(&AUTH_NODE_KEY, storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Commitment {
    pub caller_account: HumanAddr,
    pub spec_id: Binary,
    pub callback_address: HumanAddr,
    pub callback_function_id: Binary,
    pub data: Binary,
    pub payment: Uint128,
    pub expiration: Expiration,
}

pub fn commitments<S: Storage>(storage: &mut S) -> Bucket<S, Commitment> {
    bucket(&COMMITMENTS, storage)
}

pub fn commitments_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, Commitment> {
    bucket_read(&COMMITMENTS, storage)
}
