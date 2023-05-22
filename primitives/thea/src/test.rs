use crate::types::Withdraw;
use sp_core::Pair;
use sp_runtime::traits::IdentifyAccount;

#[test]
fn get_thea_public_key() {
	let seed = "owner word vocal dose decline sunset battle example forget excite gentle waste//";
	let idx = 3;
	let thea = crate::crypto::Pair::from_string(
		&(seed.to_owned() + idx.to_string().as_str() + "//thea"),
		None,
	)
	.unwrap();
	println!("public_key : {:?}", hex::encode(thea.public().into_account()));
}

use parity_scale_codec::Decode;

#[test]
fn decode_withdraw() {
	let data = "04807a9d37eb4c4026ad79e3513d598c316720cda462eb5739954a52247291404f5388d9c8207fa69f3812744433c6417f3a00008a5d7845630100000000000000007001010200a10f0300838559281acd8ba8ebce4c4bbe8d6aeb85cc03f20000";

	let bytes = hex::decode(data).unwrap();

	let payload: Vec<Withdraw> = Decode::decode(&mut &bytes[..]).unwrap();

	println!("payload: {:?}", payload);
}
