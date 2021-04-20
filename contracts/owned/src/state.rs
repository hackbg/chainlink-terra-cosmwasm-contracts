use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

pub static OWNER_KEY: &[u8] = b"owner";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: CanonicalAddr,
    pub pending_owner: Option<CanonicalAddr>,
}

pub fn owner<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, OWNER_KEY)
}

pub fn owner_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, OWNER_KEY)
}
