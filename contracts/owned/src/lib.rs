pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

// TODO! convert owned to package

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
