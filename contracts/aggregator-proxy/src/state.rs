use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdResult, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static CURRENT_PHASE_KEY: &[u8] = b"current_phase";
pub static PROPOSED_AGGREGATOR_KEY: &[u8] = b"proposed_aggregator";
pub static PREFIX_PHASE_AGGREGATORS: &[u8] = b"phase_aggregators";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Phase {
    pub id: u16,
    pub aggregator_addr: CanonicalAddr,
}

pub fn current_phase<S: Storage>(storage: &mut S) -> Singleton<S, Phase> {
    singleton(storage, CURRENT_PHASE_KEY)
}

pub fn current_phase_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Phase> {
    singleton_read(storage, CURRENT_PHASE_KEY)
}

pub fn proposed_aggregator<S: Storage>(storage: &mut S) -> Singleton<S, CanonicalAddr> {
    singleton(storage, PROPOSED_AGGREGATOR_KEY)
}

pub fn proposed_aggregator_read<S: ReadonlyStorage>(
    storage: &S,
) -> ReadonlySingleton<S, CanonicalAddr> {
    singleton_read(storage, PROPOSED_AGGREGATOR_KEY)
}

pub fn phase_aggregators<S: Storage>(storage: &mut S) -> Bucket<S, CanonicalAddr> {
    bucket(PREFIX_PHASE_AGGREGATORS, storage)
}

pub fn phase_aggregators_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, CanonicalAddr> {
    bucket_read(PREFIX_PHASE_AGGREGATORS, storage)
}

pub fn set_phase_aggregator<S: Storage>(
    storage: &mut S,
    id: u16,
    aggregator: &CanonicalAddr,
) -> StdResult<()> {
    phase_aggregators(storage).save(&id.to_be_bytes(), aggregator)
}

pub fn get_phase_aggregator<S: ReadonlyStorage>(storage: &S, id: u16) -> StdResult<CanonicalAddr> {
    phase_aggregators_read(storage).load(&id.to_be_bytes())
}
