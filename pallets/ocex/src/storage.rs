use hash_db::{AsHashDB, HashDB, Prefix};
use sp_core::Hasher;
use sp_runtime::{offchain::storage::StorageValueRef, traits::BlakeTwo256};
use trie_db::DBValue;

pub struct State;

pub const HASHED_NULL_NODE: [u8; 31] = *b"offchain-ocex::hashed_null_node";
pub const NULL_NODE_DATA: [u8; 29] = *b"offchain-ocex::null_node_data";

impl State {
	pub fn hashed_null_node(&self) -> <BlakeTwo256 as Hasher>::Out {
		let s_r = StorageValueRef::persistent(&HASHED_NULL_NODE);
		match s_r.get::<<BlakeTwo256 as Hasher>::Out>() {
			Ok(Some(x)) => x,
			Ok(None) => BlakeTwo256::hash(&[0u8]),
			Err(_) => BlakeTwo256::hash(&[0u8]),
		}
	}

	pub fn null_node_data(&self) -> Vec<u8> {
		let s_r = StorageValueRef::persistent(&NULL_NODE_DATA);
		match s_r.get::<Vec<u8>>() {
			Ok(Some(x)) => x,
			Ok(None) => [0u8].to_vec(),
			Err(_) => [0u8].to_vec(),
		}
	}

	pub fn db_get(&self, key: &<BlakeTwo256 as Hasher>::Out) -> Option<(DBValue, i32)> {
		let s_ref = StorageValueRef::persistent(key.as_bytes());
		match s_ref.get::<(DBValue, i32)>() {
			Ok(d) => d,
			Err(_) => None,
		}
	}

	pub fn db_insert(&self, key: <BlakeTwo256 as Hasher>::Out, value: (DBValue, i32)) {
		let s_ref = StorageValueRef::persistent(key.as_bytes());
		s_ref.set(&value);
	}
}

impl AsHashDB<BlakeTwo256, DBValue> for State {
	fn as_hash_db(&self) -> &dyn HashDB<BlakeTwo256, DBValue> {
		self
	}

	fn as_hash_db_mut<'a>(&'a mut self) -> &'a mut (dyn HashDB<BlakeTwo256, DBValue> + 'a) {
		self
	}
}

impl HashDB<BlakeTwo256, DBValue> for State {
	fn get(&self, key: &<BlakeTwo256 as Hasher>::Out, _: Prefix) -> Option<DBValue> {
		if key == &self.hashed_null_node() {
			return Some(self.null_node_data().clone())
		}

		match self.db_get(&key) {
			Some((ref d, rc)) if rc > 0 => Some(d.clone()),
			_ => None,
		}
	}

	fn contains(&self, key: &<BlakeTwo256 as Hasher>::Out, _: Prefix) -> bool {
		if key == &self.hashed_null_node() {
			return true
		}

		match self.db_get(&key) {
			Some((_, x)) if x > 0 => true,
			_ => false,
		}
	}

	fn insert(&mut self, prefix: Prefix, value: &[u8]) -> <BlakeTwo256 as Hasher>::Out {
		if DBValue::from(value) == self.null_node_data() {
			return self.hashed_null_node()
		}
		let key = BlakeTwo256::hash(value);
		HashDB::emplace(self, key, prefix, value.into());
		key
	}

	fn emplace(&mut self, key: <BlakeTwo256 as Hasher>::Out, _: Prefix, value: DBValue) {
		if value == self.null_node_data() {
			return
		}

		match self.db_get(&key) {
			Some((mut old_value, mut rc)) => {
				if rc <= 0 {
					old_value = value;
				}
				rc += 1;
				self.db_insert(key, (old_value, rc));
			},
			None => {
				self.db_insert(key, (value, 1));
			},
		}
	}

	fn remove(&mut self, key: &<BlakeTwo256 as Hasher>::Out, _: Prefix) {
		if key == &self.hashed_null_node() {
			return
		}

		match self.db_get(key) {
			Some((value, mut rc)) => {
				rc -= 1;
				self.db_insert(*key, (value, rc));
			},
			None => {
				let value = DBValue::default();
				self.db_insert(*key, (value, -1));
			},
		}
	}
}
