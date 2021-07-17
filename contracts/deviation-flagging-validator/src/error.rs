use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    /// Only callable by owner
    #[error("Only callable by owner")]
    NotOwner,
    #[error("{0}")]
    OwnedError(#[from] owned::error::ContractError),
}
