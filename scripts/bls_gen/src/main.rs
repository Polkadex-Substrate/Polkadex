use anyhow::Error;
use bls_primitives::Pair;
use sp_core::crypto::Pair as PTrait;

fn main() -> Result<(), Error> {
	const PHRASE: &str =
		"owner word vocal dose decline sunset battle example forget excite gentle waste";
	let args = std::env::args();
	if args.len() != 4 {
		return Err(Error::msg("please provide derivation [String], starting prefix digit and number of keys to derive"));
	}
	let p = Pair::from_phrase(PHRASE, None)
		.map_err(|_| Error::msg("Failed to recover from given seed"))?;
	let args: Vec<String> = args.into_iter().collect();
	let start = args[2].parse::<usize>()?;
	let count = args[3].parse::<usize>()?;
	for d_int in start..count + start {
		let derivatives = [d_int.to_string(), args[1].clone()];
		let derived =
			p.0.derive(derivatives.into_iter().map(|d| d.into()), Some(p.1))
				.map_err(|_| Error::msg("Failed to derive given path from given seed"))?
				.0;
		let public = hex::encode(derived.public().0);
		println!("{public}");
	}
	Ok(())
}
