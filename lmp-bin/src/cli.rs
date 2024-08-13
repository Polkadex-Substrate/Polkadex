use rust_decimal::Decimal;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(
        short = "e",
        long = "sub-url",
        default_value = "127.0.0.1:9944"
    )]
    pub sub_url: String,
    #[structopt(
        short = "q",
        long = "sub-phase",
        default_value = "c05c6ae125754dd17f36bcc5318498ce5c6c2f0e9e1116c68b77889a8be2ff02"
    )]
    pub sub_phase: String,
    #[structopt(
        short = "t",
        long = "total-reward-distribution"
    )]
    pub total_reward_be_distributed: Decimal,
}