use std::collections::BTreeMap;
use polkadex_primitives::rewards::Reward;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sp_core::crypto::AccountId32;
use sp_core::{Decode, Encode};
use structopt::StructOpt;
use crate::cli::Cli;
use crate::client::SubstrateClient;

mod aws;
mod client;
mod cli;

#[tokio::main]
async fn main() {
    let opt: Cli = cli::Cli::from_args();
    let trades = aws::AwsClient::get_trades();
    let mut fee_collected = BTreeMap::new();
    let substrate_client = SubstrateClient::initialize(opt.sub_url, opt.sub_phase).await.unwrap();
    let mut total_fee_collected = Decimal::from_u128(0).unwrap();
    for trade in trades {
        let maker_account = trade.maker.main_account;
        let taker_account = trade.taker.main_account;
        let maker_fee = trade.maker.fee;
        let taker_fee = trade.taker.fee;
        fee_collected
            .entry(maker_account)
            .and_modify(|fee| *fee += maker_fee)
            .or_insert(maker_fee);
        fee_collected
            .entry(taker_account)
            .and_modify(|fee| *fee += taker_fee)
            .or_insert(taker_fee);
        total_fee_collected = total_fee_collected.saturating_add(maker_fee).saturating_add(taker_fee);
    }
    let reward_map = create_reward_map(fee_collected, total_fee_collected, opt.total_reward_be_distributed);
    substrate_client.submit_proposal(reward_map).await.unwrap();
}

fn create_reward_map(fee_info: BTreeMap<AccountId32, Decimal>, total_fee_collected: Decimal, total_reward_be_distributed: Decimal) -> BTreeMap<subxt::utils::AccountId32, Reward> {
    let mut reward_map: BTreeMap<subxt::utils::AccountId32, Reward> = BTreeMap::new();
    for (account,fee) in fee_info {
        let prop = fee.checked_div(total_fee_collected).unwrap_or_default();
        let assigned_reward = prop.saturating_mul(total_reward_be_distributed);
        let reward = Reward { amount: assigned_reward, is_claimed: false };
        let account: subxt::utils::AccountId32 = Decode::decode(&mut &account.encode()[..]).unwrap();
        reward_map.insert(account, reward);
    }
    reward_map
}
