use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub rac_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RaiseFlag { subject: Addr },
    RaiseFlags { subjects: Vec<Addr> },
    LowerFlags { subjects: Vec<Addr> },
    SetRaisingAccessController { rac_address: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetFlag { subject: Addr },
    GetFlags { subjects: Vec<Addr> },
    GetRac {},
}
