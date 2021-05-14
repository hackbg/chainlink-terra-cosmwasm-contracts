use cw20::TokenInfoResponse;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;

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

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");
