use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractErr {
    /// Only callable by owner
    #[error("Only callable by owner")]
    NotOwner,
}

impl ContractErr {
    pub fn std(&self) -> StdError {
        StdError::generic_err(format!("{}", self))
    }

    pub fn std_err<T>(&self) -> Result<T, StdError> {
        Err(self.std())
    }
}
