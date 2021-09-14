use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    //pub rac_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Initiate contract ownership transfer to another address.
    /// Can be used only by owner
    TransferOwnership {
        /// Address to transfer ownership to
        to: String,
    },
    /// Finish contract ownership transfer. Can be used only by pending owner
    AcceptOwnership {},
    RaiseFlag {
        subject: String,
    },
    RaiseFlags {
        subjects: Vec<String>,
    },
    LowerFlags {
        subjects: Vec<String>,
    },
    SetRaisingAccessController {
        rac_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns contract owner's address
    /// Response [`Addr`]
    GetOwner {},
    GetFlag {
        subject: String,
    },
    GetFlags {
        subjects: Vec<String>,
    },
    GetRac {},
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
