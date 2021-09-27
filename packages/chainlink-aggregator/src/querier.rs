use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};

use crate::{LatestAnswerResponse, QueryMsg, RoundDataResponse};

pub trait FeedQuerier {
    fn get_latest_answer(&self, feed_address: Addr) -> StdResult<LatestAnswerResponse>;

    fn get_round_data(&self, feed_address: Addr, round_id: u32) -> StdResult<RoundDataResponse>;

    fn get_latest_round_data(&self, feed_address: Addr) -> StdResult<RoundDataResponse>;

    fn get_description(&self, feed_address: Addr) -> StdResult<String>;

    fn get_decimals(&self, feed_address: Addr) -> StdResult<u8>;

    fn get_version(&self, feed_address: Addr) -> StdResult<Uint128>;
}

impl<'a> FeedQuerier for QuerierWrapper<'a> {
    fn get_latest_answer(&self, feed_address: Addr) -> StdResult<LatestAnswerResponse> {
        self.query_wasm_smart(feed_address, &QueryMsg::GetLatestAnswer {}.wrap())
    }

    fn get_round_data(&self, feed_address: Addr, round_id: u32) -> StdResult<RoundDataResponse> {
        self.query_wasm_smart(feed_address, &QueryMsg::GetRoundData { round_id }.wrap())
    }

    fn get_latest_round_data(&self, feed_address: Addr) -> StdResult<RoundDataResponse> {
        self.query_wasm_smart(feed_address, &QueryMsg::GetLatestRoundData {}.wrap())
    }

    fn get_description(&self, feed_address: Addr) -> StdResult<String> {
        self.query_wasm_smart(feed_address, &QueryMsg::GetDescription {}.wrap())
    }

    fn get_decimals(&self, feed_address: Addr) -> StdResult<u8> {
        self.query_wasm_smart(feed_address, &QueryMsg::GetDecimals {}.wrap())
    }

    fn get_version(&self, feed_address: Addr) -> StdResult<Uint128> {
        self.query_wasm_smart(feed_address, &QueryMsg::GetVersion {}.wrap())
    }
}
