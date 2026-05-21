use anchor_lang::prelude::*;
use constant_product_curve::CurveError;

#[error_code]
pub enum AmmError {
    #[msg("Custom error message")]
    CustomError,
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Pool is locked")]
    InvalidPrecision,
    #[msg("Pool is locked")]
    Overflow,
    #[msg("Pool is locked")]
    Underflow,
    #[msg("Pool is locked")]
    InvalidFeeAmount,
    #[msg("Pool is locked")]
    InsufficientBalance,
    #[msg("Pool is locked")]
    ZeroBalance,
    #[msg("Slippage exceeded")]
    SlippageLimitExceeded,
    #[msg("Invalid Amount")]
    InvalidAmount,
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
