use once_cell::unsync::Lazy;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

/// Hashmap of beneficiary
/// Vec<u8> = beneficiary account
/// (u128,u128,u128) = (total rewards, initial rewards, factor)
/// Map will be added when values are provided
//ToDo: Issue no #2(Reward-Calculation) should modify the map with correct values.
#[allow(clippy::borrow_interior_mutable_const)]
#[allow(clippy::declare_interior_mutable_const)]
pub const HASHMAP: Lazy<BTreeMap<Vec<u8>, (u128, u128, u128)>> = Lazy::new(|| {
	BTreeMap::from([
		(
			Vec::from([
				7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
				7, 7, 7, 7,
			]),
			(200000000000000, 50000000000000, 1500000000000),
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
