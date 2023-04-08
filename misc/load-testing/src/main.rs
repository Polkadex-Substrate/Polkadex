use jsonrpsee::{core::client::ClientT, rpc_params, ws_client::WsClientBuilder};
use orderbook_primitives::types::{ObMessage, UserActions};

#[tokio::main]
async fn main() {
	let url = String::from("ws://localhost:9944");

	let client = WsClientBuilder::default().build(&url).await.unwrap();

	let message =
		ObMessage { stid: 1, action: UserActions::Snapshot, signature: Default::default() };
	client.request("ob_submitAction", rpc_params![message]).await.unwrap()
}
