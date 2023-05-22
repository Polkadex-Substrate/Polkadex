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
