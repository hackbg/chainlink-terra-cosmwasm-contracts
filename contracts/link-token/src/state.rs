use cw20::TokenInfoResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ReadonlyStorage, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
}

impl From<TokenInfo> for TokenInfoResponse {
    fn from(info: TokenInfo) -> Self {
        Self {
            name: info.name,
            symbol: info.symbol,
            decimals: info.decimals,
            total_supply: info.total_supply,
        }
    }
}

pub static TOKEN_INFO_KEY: &[u8] = b"token_info";

pub fn token_info<S: Storage>(storage: &mut S) -> Singleton<S, TokenInfo> {
    singleton(storage, TOKEN_INFO_KEY)
}

pub fn token_info_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, TokenInfo> {
    singleton_read(storage, TOKEN_INFO_KEY)
}
