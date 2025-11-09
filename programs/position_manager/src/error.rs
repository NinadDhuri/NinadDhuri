use anchor_lang::prelude::*;

#[error_code]
pub enum PositionError {
    #[msg("Symbol length exceeds maximum supported length")]
    SymbolTooLong,
    #[msg("Invalid leverage requested")]
    InvalidLeverage,
    #[msg("Position size must be greater than zero")]
    InvalidPositionSize,
    #[msg("Position with the provided identifier already exists")]
    PositionAlreadyInitialized,
    #[msg("User account does not match the provided owner")]
    Unauthorized,
    #[msg("Insufficient collateral available to lock the required margin")]
    InsufficientCollateral,
    #[msg("Math operation resulted in overflow")]
    MathOverflow,
    #[msg("Requested leverage exceeds the configured tier limits")]
    LeverageExceeded,
    #[msg("Provided position identifier does not match the stored account state")]
    PositionIdMismatch,
    #[msg("Position is already closed")]
    PositionClosed,
    #[msg("Cannot remove more margin than currently locked on the position")]
    MarginTooLow,
    #[msg("Cannot decrease position below zero size")]
    PositionSizeTooSmall,
    #[msg("Funding payment would overflow tracked amount")]
    FundingOverflow,
}
