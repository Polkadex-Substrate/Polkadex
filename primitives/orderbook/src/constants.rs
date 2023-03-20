use polkadex_primitives::Balance;

pub const GRANDPA_AUTHORITIES_KEY: &[u8] = b":grandpa_authorities";
pub const POLKADEX_MAINNET_SS58: u16 = 88;

pub const MAX_WITHDRAWALS_PER_SNAPSHOT: u8 = 20;
pub const UNIT_BALANCE: Balance = 1_000_000_000_000_u128;
// Range of QTY: 0.00000001 to 10,000,000 UNITs
pub const MIN_QTY: Balance = UNIT_BALANCE / 10000000;
pub const MAX_QTY: Balance = 10000000 * UNIT_BALANCE;
// Range of PRICE: 0.00000001 to 10,000,000 UNITs
pub const MIN_PRICE: Balance = UNIT_BALANCE / 10000000;
pub const MAX_PRICE: Balance = 10000000 * UNIT_BALANCE;
pub const ADDRESSFORMAT: u8 = 88u8;

#[test]
pub fn test_overflow_check() {
	assert!(MAX_PRICE.checked_mul(MAX_QTY).is_some());
}
