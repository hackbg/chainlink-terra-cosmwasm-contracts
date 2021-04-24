use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO raising_access_controller
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub rac_address: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    RaiseFlag { subject: HumanAddr },
    RaiseFlags { subjects: Vec<HumanAddr> },
    LowerFlags { subjects: Vec<HumanAddr> },
    SetRaisingAccessController { rac_address: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetFlag { subject: HumanAddr },
    GetFlags { subjects: Vec<HumanAddr> },
    GetRac {},
}
