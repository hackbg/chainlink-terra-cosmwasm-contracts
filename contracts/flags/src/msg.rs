use cosmwasm_std::CanonicalAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO raising_access_controller
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    RaiseFlag { subject: CanonicalAddr },
    RaiseFlags { subjects: Vec<CanonicalAddr> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetFlag { subject: CanonicalAddr },
    GetFlags { subjects: Vec<CanonicalAddr> },
}