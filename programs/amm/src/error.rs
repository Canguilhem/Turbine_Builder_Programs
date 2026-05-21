use anchor_lang::prelude::*;
use constant_product_curve::CurveError;

#[error_code]
pub enum AmmError {
    #[msg("Custom error message")]
    CustomError,
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Invalid precision")]
    InvalidPrecision,
    #[msg("Overflow")]
    Overflow,
    #[msg("Underflow")]
    Underflow,
    #[msg("Invalid fee amount")]
    InvalidFeeAmount,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Zero Balance")]
    ZeroBalance,
    #[msg("Slippage exceeded")]
    SlippageLimitExceeded,
    #[msg("Invalid Amount")]
    InvalidAmount,
    #[msg("Unauthorized")]
    Unauthorized,
}

impl From<CurveError> for AmmError {
    fn from(error: CurveError) -> AmmError {
        match error {
            CurveError::InvalidPrecision => AmmError::InvalidPrecision,
            CurveError::Overflow => AmmError::Overflow,
            CurveError::Underflow => AmmError::Underflow,
            CurveError::InvalidFeeAmount => AmmError::InvalidFeeAmount,
            CurveError::InsufficientBalance => AmmError::InsufficientBalance,
            CurveError::ZeroBalance => AmmError::ZeroBalance,
            CurveError::SlippageLimitExceeded => AmmError::SlippageLimitExceeded,
        }
    }
}
