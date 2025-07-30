use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Name cannot be undefined")]
    UndefinedName,
    #[msg("Name cannot be longer than 32 characters")]
    NameTooLong,
    #[msg("Error while performing arithmetic probable overflow")]
    MathOverflowError,
}