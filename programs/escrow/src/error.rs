use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowErrors {
    #[msg("Custom error message")]
    CustomError,
    #[msg("Escrow expired")]
    EscrowExpired,
    #[msg("Expiration must be in the future")]
    ExpirationInPast,
}
