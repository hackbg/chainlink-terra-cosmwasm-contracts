use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Map, Item};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub check_enabled: bool,
}

pub const CONFIG: Item<State> = Item::new("config");

pub const ACCESS_LIST: Map<&Addr, bool> = Map::new("access_list");
