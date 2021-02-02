use crate::chain_spec::ChainSpec;
use log::info;
use wasm_bindgen::prelude::*;
use browser_utils::{
    Client,
    browser_configuration, set_console_error_panic_hook, init_console_log,
};
use std::str::FromStr;

/// Starts the client.
#[wasm_bindgen]
pub async fn start_client(chain_spec: Option<String>, log_level: String) -> Result<Client, JsValue> {
    start_inner(chain_spec, log_level)
        .await
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn start_inner(chain_spec: Option<String>, log_level: String) -> Result<Client, Box<dyn std::error::Error>> {
    set_console_error_panic_hook();
    init_console_log(log::Level::from_str(&log_level)?)?;
    let chain_spec = match chain_spec {
        Some(chain_spec) => ChainSpec::from_json_bytes(chain_spec.as_bytes().to_vec())
            .map_err(|e| format!("{:?}", e))?,
        None => crate::chain_spec::development_config(),
    };

    let config = browser_configuration(chain_spec).await?;

    info!("Substrate browser node");
    info!("✌️  version {}", config.impl_version);
    info!("❤️  by Parity Technologies, 2017-2020");
    info!("📋 Chain specification: {}", config.chain_spec.name());
    info!("🏷 Node name: {}", config.network.node_name);
    info!("👤 Role: {:?}", config.role);

    // Create the service. This is the most heavy initialization step.
    let (task_manager, rpc_handlers) =
        crate::service::new_light_base(config)
            .map(|(components, rpc_handlers, _, _, _)| (components, rpc_handlers))
            .map_err(|e| format!("{:?}", e))?;

    Ok(browser_utils::start_client(task_manager, rpc_handlers))
}