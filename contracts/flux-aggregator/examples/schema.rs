use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use flux_aggregator::msg::*;
use flux_aggregator::state::*;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(RoundDataResponse), &out_dir);
    export_schema(&schema_for!(OracleRoundStateResponse), &out_dir);
    export_schema(&schema_for!(State), &out_dir);
    export_schema(&schema_for!(OracleStatus), &out_dir);
    export_schema(&schema_for!(Round), &out_dir);
    export_schema(&schema_for!(RoundDetails), &out_dir);
    export_schema(&schema_for!(Requester), &out_dir);
    export_schema(&schema_for!(Funds), &out_dir);
}
