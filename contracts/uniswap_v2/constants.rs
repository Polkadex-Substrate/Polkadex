/// Trading fee rate
/// The first item of the tuple is the numerator of the fee rate, second
/// item is the denominator, fee_rate = numerator / denominator,
/// use (u32, u32) over `Rate` type to minimize internal division
/// operation.
pub const GET_EXCHANGE_FEE: (u32, u32) = (3, 1000);

/// The limit for length of trading path
pub const TRADING_PATH_LIMIT: u32 = 8;
