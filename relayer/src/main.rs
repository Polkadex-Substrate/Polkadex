use sp_core::H256;
use relayer::native_connector::*;
use structopt::StructOpt;
use std::str::FromStr;


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
	let block_import_handler = tokio::spawn(async move {
		substrate_reader.run().await;
	});

	if let Err(err) = tokio::try_join!(block_import_handler) {
		return Err(anyhow::Error::new(err));
	}
	Ok(())
}