use clap::Parser;
use csv::StringRecord;
use rust_decimal::Decimal;
use sp_core::{
	crypto::{Ss58AddressFormat, Ss58Codec},
	ByteArray,
};
use std::{ops::Div, str::FromStr};

use pallet_rewards::crowdloan_rewardees::HASHMAP;
use polkadex_primitives::{AccountId, UNIT_BALANCE};

#[derive(Parser)]
struct Cli {
	/// Path to excel worksheet
	#[arg(short, long)]
	path: std::path::PathBuf,
	/// Search address
	#[arg(short, long)]
	user: Option<String>,
}

fn main() {
	let args = Cli::parse();
	let polkadex_version = Ss58AddressFormat::from(88u16);
	let polkadot_version = Ss58AddressFormat::from(0u16);
	let unit = Decimal::from(UNIT_BALANCE);
	if args.user.is_some() {
		if let Ok(user) = AccountId::from_str(&args.user.unwrap()) {
			println!("User Account Info ");
			println!("---------------------------------------------------------------------------");
			println!("User ( Polkadex ): {:?}", user.to_ss58check_with_version(polkadex_version));
			println!("User ( Polkadot ): {:?}", user.to_ss58check_with_version(polkadot_version));
			println!("---------------------------------------------------------------------------");
			if let Some(details) = HASHMAP.get(user.as_slice()) {
				println!("Reward Details ");
				println!(
					"---------------------------------------------------------------------------"
				);
				println!("Total Rewards: {:?} PDEX", Decimal::from(details.0).div(unit));
				println!("25% Cliff: {:?} PDEX", Decimal::from(details.1).div(unit));
				println!(
					"Amount claimable per block: {:?} PDEX",
					Decimal::from(details.1).div(unit)
				);
				println!(
					"---------------------------------------------------------------------------"
				);
				return
			} else {
				println!("User not found in contributor list.");
				return
			}
		} else {
			println!("Not a valid user address");
			return
		}
	}
	// Open CSV file
	let mut rdr = csv::Reader::from_path(args.path).unwrap();
	assert_eq!(
		HASHMAP.len(),
		rdr.records().collect::<Vec<csv::Result<StringRecord>>>().len(),
		"Number of users doesn't match!"
	);
	for result in rdr.records() {
		let record = result.unwrap();
		let user = AccountId::from_str(record.get(0).unwrap()).unwrap();
		let total_rewards = Decimal::from_str(record.get(1).unwrap()).unwrap();
		let cliff_amt = Decimal::from_str(record.get(2).unwrap()).unwrap();
		let claim_per_blk = Decimal::from_str(record.get(3).unwrap()).unwrap();
		let dot_contributed = Decimal::from_str(record.get(4).unwrap()).unwrap();
		if let Some(details) = HASHMAP.get(user.as_slice()) {
			let total_rewards_list = Decimal::from(details.0).div(unit);
			let cliff_amt_list = Decimal::from(details.1).div(unit);
			let claim_per_blk_list = Decimal::from(details.2).div(unit);
			if (total_rewards != total_rewards_list) ||
				(cliff_amt != cliff_amt_list) ||
				(claim_per_blk != claim_per_blk_list)
			{
				println!("ERROR IN REWARDS INFO");
				println!(
					"---------------------------------------------------------------------------"
				);
				println!(
					"User ( Polkadex ): {:?}",
					user.to_ss58check_with_version(polkadex_version)
				);
				println!(
					"User ( Polkadot ): {:?}",
					user.to_ss58check_with_version(polkadot_version)
				);
				println!();
				println!("Reward details in Pallet Hashmap");
				println!(
					"---------------------------------------------------------------------------"
				);
				println!("Total Rewards: {:?} PDEX", total_rewards_list);
				println!("25% Cliff: {:?} PDEX", cliff_amt_list);
				println!("Amount claimable per block: {:?} PDEX", claim_per_blk_list);
				println!();
				println!("Reward details in CSV File");
				println!(
					"---------------------------------------------------------------------------"
				);
				println!("Total Rewards: {:?} PDEX", total_rewards);
				println!("25% Cliff: {:?} PDEX", cliff_amt);
				println!("Amount claimable per block: {:?} PDEX", claim_per_blk);
				println!("DOT contributed: {:?} DOT", dot_contributed);
				return
			}
		} else {
			println!("User Account Info ");
			println!("---------------------------------------------------------------------------");
			println!("User ( Polkadex ): {:?}", user.to_ss58check_with_version(polkadex_version));
			println!("User ( Polkadot ): {:?}", user.to_ss58check_with_version(polkadot_version));
			println!("USER NOT FOUND IN LIST");
			println!("---------------------------------------------------------------------------");
			return
		}
	}
}
