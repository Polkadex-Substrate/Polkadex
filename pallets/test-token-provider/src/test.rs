use crate::mock::*;
use codec::Encode;
use frame_support::assert_ok;
// use sp_application_crypto::RuntimePublic;
use crate::pallet::Pallet;
use sp_core::sr25519::Signature;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use std::convert::TryInto;

#[test]
fn test_minting_token() {
	const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"thea");
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.unwrap();

	let account_id_2 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter2", PHRASE)),
	)
	.unwrap();
	new_test_ext().execute_with(|| {
		assert_ok!(Token::credit_account_with_tokens_unsigned(Origin::none(), account_id));
		assert_ok!(Token::credit_account_with_tokens_unsigned(Origin::none(), account_id_2));
	});
}
