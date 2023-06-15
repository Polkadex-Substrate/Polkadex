#[test]
fn generate_bls_fixtures() {
	use bls_primitives::Pair;
	use parity_scale_codec::Encode;
	use sp_core::Pair as CorePair;
	use thea_primitives::types::Message;
	let pair = Pair::from_seed_slice(&[2u8; 256]).unwrap();
	let message = Message {
		block_no: u64::MAX,
		nonce: 1,
		data: [255u8; 576].into(), //10 MB
		network: 0u8,
		is_key_change: false,
		validator_set_id: 0,
		validator_set_len: 1,
	};
	let payload = message.encode();
	let signature = pair.sign(payload.as_ref());
	let pk = pair.public().encode();
	println!("payload: {:?}\nsignature: {:?}\npublic: {:?}", payload, signature.encode(), pk);
}
