use hash_db::Hasher;
use sp_core::H256;

use hash256_std_hasher::Hash256StdHasher;

// This is duplicated to make it work with
// our fork of memory db
#[derive(Debug)]
pub struct CustomBlake2Hasher;

impl Hasher for CustomBlake2Hasher {
	type Out = H256;
	type StdHasher = Hash256StdHasher;
	const LENGTH: usize = 32;

	fn hash(x: &[u8]) -> Self::Out {
		sp_core::hashing::blake2_256(x).into()
	}
}
