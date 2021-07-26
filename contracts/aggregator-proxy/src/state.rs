use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map, U16Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Phase {
    pub id: u16,
    pub aggregator_addr: Addr,
}

pub const CURRENT_PHASE: Item<Phase> = Item::new("current_phase");
pub const PROPOSED_AGGREGATOR: Item<Addr> = Item::new("proposed_aggregator");
pub const PHASE_AGGREGATORS: Map<U16Key, Addr> = Map::new("phase_aggreagtors");
