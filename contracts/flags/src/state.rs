use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket,
    ReadonlySingleton, Singleton,
};
pub static CONFIG_KEY: &[u8] = b"config";
pub static FLAG_KEY: &[u8] = b"flags";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    raising_access_controller: HumanAddr,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn flags<S: Storage>(storage: &mut S) -> Bucket<S, bool> {
    bucket(&FLAG_KEY, storage)
}

pub fn flags_read<S: Storage>(storage: &S) -> ReadonlyBucket<S, bool> {
    bucket_read(&FLAG_KEY, storage)
}
