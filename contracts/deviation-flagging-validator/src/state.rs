use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

pub static FLAG_KEY: &[u8] = b"flags";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub flagging_treshold: u32,
    pub flags: CanonicalAddr
}

pub fn flags<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, FLAG_KEY)
}

pub fn flags_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, FLAG_KEY)
}
