use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, HumanAddr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";
pub static AUTH_NODE_KEY: &[u8] = b"authorized_nodes";

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
