use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Owned(#[from] owned::error::ContractError),

    /// Invalid proposed aggregator
    #[error("Invalid proposed aggregator")]
    InvalidProposedAggregator {},

    /// No proposed aggregator present
    #[error("No proposed aggregator present")]
    NoProposedAggregator {},

    /// Only callable by owner
    #[error("Only callable by owner")]
    NotOwner {},
}
