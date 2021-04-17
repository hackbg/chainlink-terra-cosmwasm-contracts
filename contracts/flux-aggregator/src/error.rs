use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractErr {
    /// Min cannot be greater than max
    #[error("Min cannot be greater than max")]
    MinGreaterThanMax,
    /// Max cannot exceed total
    #[error("Max cannot exceed total")]
    MaxGreaterThanTotal,
    /// Min must be greater than 0
    #[error("Min must be greater than 0")]
    MinLessThanZero,
    /// No data present
    #[error("No data present")]
    NoData,
    /// Value under threshold
    #[error("Value under threshold")]
    UnderMin,
    /// Value over threshold
    #[error("Value over threshold")]
    OverMax,
    /// Only callable by owner
    #[error("Only callable by owner")]
    NotOwner,
    /// Only callable by admin
    #[error("Only callable by admin")]
    NotAdmin,
    /// Owner cannot overwrite admin
    #[error("Owner cannot overwrite admin")]
    OverwritingAdmin,
    /// Only callable by pending admin
    #[error("Only callable by pending admin")]
    NotPendingAdmin,
    /// No pending admin
    #[error("No pending admin")]
    PendingAdminMissing,
    /// Cannot set empty admin address
    #[error("Cannot set empty admin address")]
    EmptyAdminAddr,
    /// Need same oracle and admin count
    #[error("Need same oracle and admin count")]
    OracleAdminCountMismatch,
    /// Cannot add more oracles
    #[error("Cannot add more oracles")]
    MaxOraclesAllowed,
    /// Oracle already enabled
    #[error("Oracle already enabled")]
    OracleAlreadyEnabled,
    /// Oracle not enabled
    #[error("Oracle not enabled")]
    OracleNotEnabled,
    /// Oracle not yet enabled
    #[error("Oracle not yet enabled")]
    OracleNotYetEnabled,
    /// Insufficient funds for payment
    #[error("Insufficient funds for payment")]
    InsufficientFunds,
    /// Insufficient withdrawable funds
    #[error("Insufficient withdrawable funds")]
    InsufficientWithdrawableFunds,
    /// Insufficient reserve funds
    #[error("Insufficient reserve funds")]
    InsufficientReserveFunds,
    /// Delay cannot exceed total
    #[error("Delay cannot exceed total")]
    DelayGreaterThanTotal,
    /// Cannot report on previous rounds
    #[error("Cannot report on previous rounds")]
    ReportingPreviousRound,
    /// Round not accepting submissions
    #[error("Round not accepting submissions")]
    NotAcceptingSubmissions,
    /// Receive does not expect payload
    #[error("Receive does not expect payload")]
    UnexpectedReceivePayload,
}

impl ContractErr {
    pub fn std(&self) -> StdError {
        StdError::generic_err(format!("{}", self))
    }

    pub fn std_err<T>(&self) -> Result<T, StdError> {
        Err(self.std())
    }
}
