use hash_db::{AsHashDB, HashDB, Prefix};
use sp_core::{Hasher, H256};
use sp_runtime::{offchain::storage::StorageValueRef, sp_std, traits::BlakeTwo256};
use sp_std::vec::Vec;
use sp_trie::{trie_types::TrieDBMutBuilderV1, LayoutV1};
use trie_db::{DBValue, TrieDBMut};

pub struct State;

const HASHED_NULL_NODE: [u8; 31] = *b"offchain-ocex::hashed_null_node";
const NULL_NODE_DATA: [u8; 29] = *b"offchain-ocex::null_node_data";
const KEY_PREFIX: [u8; 15] = *b"offchain-ocex::";
const TRIE_ROOT: [u8; 24] = *b"offchain-ocex::trie_root";

impl State {
	fn hashed_null_node(&self) -> <BlakeTwo256 as Hasher>::Out {
		let s_r = StorageValueRef::persistent(&HASHED_NULL_NODE);
		match s_r.get::<<BlakeTwo256 as Hasher>::Out>() {
			Ok(Some(x)) => x,
			Ok(None) => {
				log::trace!(target:"ocex","hashed_null_node not found");
				BlakeTwo256::hash(&[0u8])
			},
			Err(_) => {
				log::trace!(target:"ocex","hashed_null_node get error");
				BlakeTwo256::hash(&[0u8])
			},
		}
	}

	fn null_node_data(&self) -> Vec<u8> {
		let s_r = StorageValueRef::persistent(&NULL_NODE_DATA);
		match s_r.get::<Vec<u8>>() {
			Ok(Some(x)) => x,
			Ok(None) => {
				log::trace!(target:"ocex","null_node_data is default");
				[0u8].to_vec()
			},
			Err(_) => {
				log::trace!(target:"ocex","null_node_data is default");
				[0u8].to_vec()
			},
		}
	}

	fn db_get(&self, key: &<BlakeTwo256 as Hasher>::Out) -> Option<(DBValue, i32)> {
		let derive_key = self.derive_storage_key(*key);
		let s_ref = StorageValueRef::persistent(derive_key.as_slice());
		match s_ref.get::<(DBValue, i32)>() {
			Ok(d) => d,
			Err(_) => None,
		}
	}

	fn db_insert(&self, key: <BlakeTwo256 as Hasher>::Out, value: (DBValue, i32)) {
		let derive_key = self.derive_storage_key(key);
		let s_ref = StorageValueRef::persistent(derive_key.as_slice());
		s_ref.set(&value);
	}

	fn derive_storage_key(&self, key: <BlakeTwo256 as Hasher>::Out) -> Vec<u8> {
		let mut derived = KEY_PREFIX.to_vec();
		derived.append(&mut key.0.to_vec());
		derived
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
			return Some(self.null_node_data())
		}

		match self.db_get(key) {
			Some((ref d, rc)) if rc > 0 => Some(d.clone()),
			_ => None,
		}
	}

	fn contains(&self, key: &<BlakeTwo256 as Hasher>::Out, _: Prefix) -> bool {
		if key == &self.hashed_null_node() {
			return true
		}

		matches!(self.db_get(key), Some((_, x)) if x > 0)
	}

	fn insert(&mut self, prefix: Prefix, value: &[u8]) -> <BlakeTwo256 as Hasher>::Out {
		if *value == self.null_node_data() {
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

pub(crate) fn load_trie_root() -> <BlakeTwo256 as Hasher>::Out {
	let root_ref = StorageValueRef::persistent(&TRIE_ROOT);
	match root_ref.get::<<BlakeTwo256 as Hasher>::Out>() {
		Ok(Some(root)) => root,
		Ok(None) => Default::default(),
		Err(_) => Default::default(),
	}
}

pub(crate) fn store_trie_root(root: <BlakeTwo256 as Hasher>::Out) {
	let root_ref = StorageValueRef::persistent(&TRIE_ROOT);
	root_ref.set(&root);
}

pub(crate) fn get_state_trie<'a>(
	state: &'a mut State,
	root: &'a mut H256,
) -> TrieDBMut<'a, LayoutV1<BlakeTwo256>> {
	if *root == H256::zero() {
		TrieDBMutBuilderV1::new(state, root).build()
	} else {
		TrieDBMutBuilderV1::from_existing(state, root).build()
	}
}

#[cfg(test)]
mod tests {
	use trie_db::TrieMut;

	use crate::{
		mock::new_test_ext,
		storage::{get_state_trie, load_trie_root, store_trie_root, State},
		tests::register_offchain_ext,
	};

	#[test]
	pub fn test_trie_storage() {
		let mut ext = new_test_ext();
		register_offchain_ext(&mut ext);
		log::trace!(target:"ocex","test_trie_storage test starting..");
		ext.execute_with(|| {
			let mut root = load_trie_root();
			{
				let mut storage = State;

				let mut state = get_state_trie(&mut storage, &mut root);

				state.insert(b"1", b"a").unwrap();
				state.insert(b"2", b"b").unwrap();
				state.insert(b"3", b"c").unwrap();

				state.commit();
			}

			store_trie_root(root);

			{
				let mut root = load_trie_root();
				let mut storage = State;

				let state = get_state_trie(&mut storage, &mut root);
				assert_eq!(state.get(b"1").unwrap().unwrap(), b"a");
				assert_eq!(state.get(b"2").unwrap().unwrap(), b"b");
				assert_eq!(state.get(b"3").unwrap().unwrap(), b"c");
			}
		})
	}
}
