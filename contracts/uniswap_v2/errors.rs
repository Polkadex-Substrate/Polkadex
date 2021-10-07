use err_derive::Error;

/// Error types
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode, err_derive::Error)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    #[error(display = "TokenAddress is invalid")]
    InvalidTokenAddress,
    #[error(display = "LiquidityIncrement is invalid")]
    InvalidLiquidityIncrement,
    #[error(display = "Arithmetic Overflow occured")]
    ArithmeticOverflow,
    #[error(display = "Arithmetic Underflow occured")]
    ArithmeticUnderflow,
    #[error(display = "Share Increment is invalid")]
    UnacceptableShareIncrement,
    #[error(display = "Unacceptable Liqudity withdrawn")]
    UnacceptableLiquidityWithdrawn,
    #[error(display = "Invalid Trading Path Length")]
    InvalidTradingPathLength,
    #[error(display = "Zero Target Amount")]
    ZeroTargetAmount,
    #[error(display = "ExceedPriceImpactLimit")]
    ExceedPriceImpactLimit,
    #[error(display = "InsufficientLiquidity")]
    InsufficientLiquidity,
    #[error(display = "InsufficientTargetAmount")]
    InsufficientTargetAmount,
    #[error(display = "InvariantCheckFailed")]
    InvariantCheckFailed,
    #[error(display = "ExcessiveSupplyAmount")]
    ExcessiveSupplyAmount,
    #[error(display = "Invalid TradingPair")]
    InvalidTradingPair,
    #[error(display = "Invalid Path/Amounts Length")]
    InvalidPathAmountsLength,
    #[error(display = "Invalid Amounts Length")]
    InvalidAmountsLength,
    #[error(display = "TransferFailed")]
    TransferFailed,
}
