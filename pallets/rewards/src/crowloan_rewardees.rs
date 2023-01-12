use once_cell::unsync::Lazy;
use polkadex_primitives::AccountId;
use sp_std::{cell::Cell, collections::btree_map::BTreeMap, fmt::Error, ops::Deref, vec::Vec};

/// Hashmap of beneficiary
/// Vec<u8> = beneficiary account
/// (u128,u128,u128) = (total rewards, initial rewards, factor)
/// Map will be added when values are provided
pub const HASHMAP: Lazy<BTreeMap<Vec<u8>, (u128, u128, u128)>> = Lazy::new(|| {
	BTreeMap::from([
		(
			Vec::from([
				172, 185, 2, 30, 96, 137, 244, 223, 129, 86, 149, 197, 1, 240, 129, 19, 222, 226,
				107, 15, 174, 234, 10, 88, 6, 155, 69, 18, 98, 76, 247, 124,
			]),
			(51370400000000, 12842600000000, 102),
		),
		(
			Vec::from([
				1, 185, 2, 30, 96, 137, 244, 223, 129, 86, 149, 197, 1, 240, 129, 19, 222, 226,
				107, 15, 174, 234, 10, 88, 6, 155, 69, 18, 98, 76, 247, 124,
			]),
			(51370400000000, 12842600000000, 102),
		),
		(
			Vec::from([
				1, 1, 2, 30, 96, 137, 244, 223, 129, 86, 149, 197, 1, 240, 129, 19, 222, 226, 107,
				15, 174, 234, 10, 88, 6, 155, 69, 18, 98, 76, 247, 124,
			]),
			(51370400000000, 12842600000000, 102),
		),
	])
});
