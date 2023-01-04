use relayer::native_connector::*;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
	#[structopt(short = "r", long = "blockchain-url", default_value = "ws://127.0.0.1:9944")]
	pub blockchain_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let opt = Cli::from_args();
	env_logger::init();
	// Initialize substrate Reader
	let mut substrate_reader = SubstrateReader::new(&opt.blockchain_url).await;
	// Read Ingress Message
	let ingress_messagers = substrate_reader.get_messages(3_288_705).await.unwrap();
	println!("Ingress Messages Received: {:?}", ingress_messagers);
	Ok(())
}