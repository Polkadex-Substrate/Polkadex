use clap::Parser;
use rust_decimal::{
    prelude::{ToPrimitive, Zero},
    Decimal,
};
use sp_core::{
    bytes::to_hex,
    crypto::{Ss58AddressFormat, Ss58Codec},
    ByteArray,
};
use std::{
    collections::BTreeMap,
    ops::{Add, Div},
    str::FromStr,
};

use pallet_rewards::crowdloan_rewardees::HASHMAP;
use polkadex_primitives::{AccountId, UNIT_BALANCE};

#[derive(Parser)]
struct Cli {
    /// Path to excel worksheet
    #[arg(short, long)]
    path: std::path::PathBuf,
    /// User address to search rewards details.
    #[arg(short, long)]
    user: Option<String>,
    /// Convert excel to sheet
    #[arg(short, long)]
    convert: bool,
}

fn main() {
    let args = Cli::parse();

    let polkadex_version = Ss58AddressFormat::from(88u16);
    let polkadot_version = Ss58AddressFormat::from(0u16);
    let unit: Decimal = Decimal::from(UNIT_BALANCE);

    if args.user.is_some() {
        // Check a specific account inside the hashmap.
        if let Ok(user) = AccountId::from_str(&args.user.unwrap()) {
            println!("User Account Info ");
            println!("---------------------------------------------------------------------------");
            println!(
                "User ( Polkadex ): {:?}",
                user.to_ss58check_with_version(polkadex_version)
            );
            println!(
                "User ( Polkadot ): {:?}",
                user.to_ss58check_with_version(polkadot_version)
            );
            println!("---------------------------------------------------------------------------");
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some((_, details)) = HASHMAP.iter().find(|inner| inner.0 == user) {
                println!("Reward Details ");
                println!(
                    "---------------------------------------------------------------------------"
                );
                println!(
                    "Total Rewards: {:?} PDEX",
                    Decimal::from(details.0).div(unit)
                );
                println!("25% Cliff: {:?} PDEX", Decimal::from(details.1).div(unit));
                println!(
                    "Amount claimable per block: {:?} PDEX",
                    Decimal::from(details.1).div(unit)
                );
                println!(
                    "---------------------------------------------------------------------------"
                );
                return;
            } else {
                println!("User not found in contributor list.");
                return;
            }
        } else {
            println!("Not a valid user address");
            return;
        }
    }
    // Open CSV file
    let mut rdr = csv::Reader::from_path(args.path).unwrap();
    // Check if CSV file and HASHMAP has same number of addresses
    #[allow(clippy::borrow_interior_mutable_const)]
    let map_len = HASHMAP.len();
    let unit_balance = Decimal::from(UNIT_BALANCE);

    if !args.convert {
        let mut map: BTreeMap<AccountId, (u128, u128, u128)> = BTreeMap::new();
        for result in rdr.records() {
            let record = result.unwrap();
            let user = AccountId::from_str(record.get(0).unwrap()).unwrap();
            let total_rewards = Decimal::from_str(record.get(2).unwrap()).unwrap();
            let cliff_amt = Decimal::from_str(record.get(3).unwrap()).unwrap();
            let claim_per_blk = Decimal::from_str(record.get(4).unwrap()).unwrap();

            let t_new = total_rewards
                .saturating_mul(unit_balance)
                .to_u128()
                .unwrap();
            let i_new = cliff_amt.saturating_mul(unit_balance).to_u128().unwrap();
            let f_new = claim_per_blk
                .saturating_mul(unit_balance)
                .to_u128()
                .unwrap();

            map.entry(user)
                .and_modify(|(t, i, f)| {
                    *t = t.saturating_add(t_new);
                    *i = i.saturating_add(i_new);
                    *f = f.saturating_add(f_new);
                })
                .or_insert((t_new, i_new, f_new));
        }
        assert_eq!(map_len, map.len(), "Number of users doesn't match!");
        // Check all addresses and their corresponding reward details, print to screen on error.
        for result in rdr.records() {
            let record = result.unwrap();
            let user = AccountId::from_str(record.get(0).unwrap()).unwrap();
            let total_rewards = Decimal::from_str(record.get(1).unwrap()).unwrap();
            let cliff_amt = Decimal::from_str(record.get(2).unwrap()).unwrap();
            let claim_per_blk = Decimal::from_str(record.get(3).unwrap()).unwrap();
            let dot_contributed = Decimal::from_str(record.get(4).unwrap()).unwrap();
            #[allow(clippy::borrow_interior_mutable_const)]
            if let Some((_, details)) = HASHMAP.iter().find(|inner| inner.0 == user) {
                let total_rewards_list = Decimal::from(details.0).div(unit);
                let cliff_amt_list = Decimal::from(details.1).div(unit);
                let claim_per_blk_list = Decimal::from(details.2).div(unit);
                if (total_rewards != total_rewards_list)
                    || (cliff_amt != cliff_amt_list)
                    || (claim_per_blk != claim_per_blk_list)
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
                    println!("Total Rewards: {total_rewards_list:?} PDEX");
                    println!("25% Cliff: {cliff_amt_list:?} PDEX");
                    println!("Amount claimable per block: {claim_per_blk_list:?} PDEX");
                    println!();
                    println!("Reward details in CSV File");
                    println!(
						"---------------------------------------------------------------------------"
					);
                    println!("Total Rewards: {total_rewards:?} PDEX");
                    println!("25% Cliff: {cliff_amt:?} PDEX");
                    println!("Amount claimable per block: {claim_per_blk:?} PDEX");
                    println!("DOT contributed: {dot_contributed:?} DOT");
                    return;
                }
            } else {
                println!("User Account Info ");
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
                println!("USER NOT FOUND IN LIST");
                println!(
                    "---------------------------------------------------------------------------"
                );
                return;
            }
        }
        println!("Excel and Source code account lists match, All good!")
    } else {
        // AccountID => (total rewards, initial rewards, reward per blk)
        let mut map: BTreeMap<AccountId, (u128, u128, u128)> = BTreeMap::new();
        let mut total_pdex = 0;
        let mut total_cliff = 0;
        let mut total_factor = 0;
        let mut total_dot = Decimal::zero();

        for result in rdr.records() {
            let record = result.unwrap();
            let user = AccountId::from_str(record.get(0).unwrap()).unwrap();
            let total_rewards = Decimal::from_str(record.get(2).unwrap()).unwrap();
            let cliff_amt = Decimal::from_str(record.get(3).unwrap()).unwrap();
            let claim_per_blk = Decimal::from_str(record.get(4).unwrap()).unwrap();
            let dot_contributed = Decimal::from_str(record.get(1).unwrap()).unwrap();

            let t_new = total_rewards
                .saturating_mul(unit_balance)
                .to_u128()
                .unwrap();
            let i_new = cliff_amt.saturating_mul(unit_balance).to_u128().unwrap();
            let f_new = claim_per_blk
                .saturating_mul(unit_balance)
                .to_u128()
                .unwrap();
            total_pdex = total_pdex.add(t_new);
            total_cliff = total_cliff.add(i_new);
            total_factor = total_factor.add(f_new);
            total_dot = total_dot.add(dot_contributed);

            map.entry(user)
                .and_modify(|(t, i, f)| {
                    *t = t.saturating_add(t_new);
                    *i = i.saturating_add(i_new);
                    *f = f.saturating_add(f_new);
                })
                .or_insert((t_new, i_new, f_new));
        }

        for (user, values) in map.iter() {
            println!("// {:?} ", to_hex(&user.to_raw_vec(), false));
            println!(
                "(AccountId::new({:?}),{:?}),",
                <sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(user),
                values
            )
        }

        println!("Map len: {:?}", map.len());
        println!(
            "Total pdex rewards: {:?}, cliff: {:?}, factor: {:?}, total_dot: {:?}",
            Decimal::from(total_pdex).div(unit_balance),
            Decimal::from(total_cliff).div(unit_balance),
            Decimal::from(total_factor).div(unit_balance),
            total_dot
        )
    }
}
