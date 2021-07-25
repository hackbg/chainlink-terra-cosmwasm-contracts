use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

pub static OWNER_KEY: &[u8] = b"owner";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub pending_owner: Option<Addr>,
}

pub fn owner(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, OWNER_KEY)
}

pub fn owner_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, OWNER_KEY)
}
