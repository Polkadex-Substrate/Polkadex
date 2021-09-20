/// Trait that retrieves call details for Polkapool
pub trait DynamicStaker<C,B> {
    /// Filters call after checking for feeless transaction calls
    fn filter(_: &C) -> bool;
    /// Get stake amount for the given call
    fn get_stake(_: &C) -> B;
}