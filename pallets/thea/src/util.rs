use crate::{mock::*, pallet::*};
use blst::min_sig::*;
use sp_core::crypto::AccountId32;
use sp_keystore::{testing::KeyStore, SyncCryptoStore};
use thea_primitives::BLSPublicKey;

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");
pub const DST: &[u8; 43] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

pub(crate) fn set_kth_bit(number: u128, k_value: u8) -> u128 {
	(1 << k_value) | number
}

pub(crate) fn create_three_bls_keys() -> Vec<SecretKey> {
	let seed_1 = [1_u8; 32];
	let secret_key_1 = SecretKey::key_gen(&seed_1, &[]).unwrap();
	let seed_2 = [2_u8; 32];
	let secret_key_2 = SecretKey::key_gen(&seed_2, &[]).unwrap();
	let seed_3 = [3_u8; 32];
	let secret_key_3 = SecretKey::key_gen(&seed_3, &[]).unwrap();
	vec![secret_key_1, secret_key_2, secret_key_3]
}

pub(crate) fn create_bls_public_keys(secret_keys: Vec<SecretKey>) -> Vec<BLSPublicKey> {
	secret_keys
		.into_iter()
		.map(|key| BLSPublicKey(key.sk_to_pk().serialize()))
		.collect::<Vec<BLSPublicKey>>()
}

pub(crate) fn sign_payload_with_keys(payload: Vec<u8>, keys: Vec<SecretKey>) -> [u8; 96] {
	let mut signatures: Vec<Signature> = vec![];
	for x in keys {
		let signature = x.sign(&payload, DST, &[]);
		signatures.push(signature)
	}
	let mut aggregate_signature = AggregateSignature::from_signature(&signatures[0]);
	aggregate_signature.add_signature(&signatures[1], true).unwrap();
	aggregate_signature.add_signature(&signatures[2], true).unwrap();
	aggregate_signature.to_signature().serialize()
}

pub(crate) fn create_account_id() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
}

pub(crate) type PrivateKeys = Vec<SecretKey>;
pub(crate) type PublicKeys = Vec<BLSPublicKey>;

pub(crate) fn get_bls_keys() -> (PrivateKeys, PublicKeys) {
	let mut private_keys: PrivateKeys = vec![];
	let ikm = [0 as u8; 32];
	let sk_1 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_1 = sk_1.sk_to_pk();
	private_keys.push(sk_1.clone());
	let ikm = [1 as u8; 32];
	let sk_2 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_2 = sk_2.sk_to_pk();
	private_keys.push(sk_2.clone());
	let ikm = [2 as u8; 32];
	let sk_3 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_3 = sk_3.sk_to_pk();
	private_keys.push(sk_3.clone());
	let bls_public_key_1 = BLSPublicKey(pk_1.serialize().into());
	let bls_public_key_2 = BLSPublicKey(pk_2.serialize().into());
	let bls_public_key_3 = BLSPublicKey(pk_3.serialize().into());
	let public_keys: PublicKeys = vec![bls_public_key_1, bls_public_key_2, bls_public_key_3];
	(private_keys, public_keys)
}

pub(crate) fn register_bls_public_keys() {
	let (_, public_keys) = get_bls_keys();
	RelayersBLSKeyVector::<Test>::insert(1, public_keys);
}

pub(crate) fn sign_payload(payload: Vec<u8>) -> [u8; 96] {
	let (private_keys, _) = get_bls_keys();
	let sig_1 = private_keys[0].sign(&payload, DST, &[]);
	let sig_2 = private_keys[1].sign(&payload, DST, &[]);
	let mut agg_sig = AggregateSignature::from_signature(&sig_1);
	agg_sig.add_signature(&sig_2, false).unwrap();
	agg_sig.to_signature().serialize()
}
