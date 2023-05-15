use bls_primitives::Pair;
use sp_core::crypto::Pair as PTrait;

fn main() {
	let p = Pair::generate().0;
	println!("{p:#?}");
}
