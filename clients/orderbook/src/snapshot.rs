use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};

/// This is a dummy struct used to serialize memory db
/// We cannot serialize the hashmap below because of non-string type in key.
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct SnapshotStore {
	#[serde_as(as = "JsonString<Vec<(JsonString, _)>>")]
	pub map: BTreeMap<[u8; 32], (Vec<u8>, i32)>,
}

impl SnapshotStore {
	pub fn new<T: IntoIterator<Item = ([u8; 32], (Vec<u8>, i32))>>(iter: T) -> Self {
		Self { map: BTreeMap::from_iter(iter) }
	}

	pub fn convert_to_hashmap(self) -> HashMap<[u8; 32], (Vec<u8>, i32)> {
		HashMap::from_iter(self.map.into_iter())
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use memory_db::{HashKey, MemoryDB};
	use parity_scale_codec::{Decode, Encode};
	use reference_trie::{ExtensionLayout, RefHasher};
	use rust_decimal::Decimal;
	use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

	use orderbook_primitives::types::AccountAsset;
	use polkadex_primitives::AssetId;

	use crate::{
		snapshot::SnapshotStore, worker::*, worker_tests::get_alice_main_and_proxy_account,
	};

	#[test]
	pub fn test_snapshot_deterministic_serialization() {
		// The snapshot generate same data on serialization irrespective
		// of internal map's iter()'s behaviour

		let mut map1 = HashMap::new();

		map1.insert([0; 32], (Vec::new(), 1));
		map1.insert([1; 32], (Vec::new(), 1));
		map1.insert([2; 32], (Vec::new(), 1));

		let mut map2 = HashMap::new();

		map2.insert([1; 32], (Vec::new(), 1));
		map2.insert([0; 32], (Vec::new(), 1));
		map2.insert([2; 32], (Vec::new(), 1));

		let store1 = SnapshotStore::new(map1.into_iter());
		let store2 = SnapshotStore::new(map2.into_iter());

		let data1 = serde_json::to_vec(&store1).unwrap();
		let data2 = serde_json::to_vec(&store2).unwrap();

		assert_eq!(data1, data2);
	}

	#[test]
	pub fn test_snaphot_with_data_ok() {
		let mut working_state_root = [0u8; 32];
		let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
		let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
		let asset_id = AssetId::Polkadex;
		let starting_balance = Decimal::new(10, 0);
		{
			let mut trie: TrieDBMut<ExtensionLayout> =
				TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();

			assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
			assert!(
				deposit(&mut trie, alice_main.clone(), asset_id.clone(), starting_balance).is_ok()
			);

			trie.commit();
		}

		let store = SnapshotStore::new(memory_db.data().clone().into_iter());

		let data = serde_json::to_vec(&store).unwrap();

		let mut chunks = data.chunks(10 * 1024 * 1024);

		let mut data_restored = Vec::new();
		for chunk in chunks {
			data_restored.append(&mut chunk.to_vec());
		}

		let store_restored: SnapshotStore = serde_json::from_slice(&data_restored).unwrap();
		assert_eq!(store_restored, store);

		let mut memory_db_restored: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> =
			Default::default();
		memory_db_restored.load_from(store_restored.convert_to_hashmap());
		let mut trie: TrieDBMut<ExtensionLayout> =
			TrieDBMutBuilder::from_existing(&mut memory_db_restored, &mut working_state_root)
				.build();
		let account_asset = AccountAsset { main: alice_main.clone(), asset: asset_id };
		let balance_encoded = trie.get(&account_asset.encode()).unwrap().unwrap();
		let balance = Decimal::decode(&mut &balance_encoded[..]).unwrap();

		assert_eq!(starting_balance, balance);
	}
}
