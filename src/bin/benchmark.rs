use std::{sync::Arc, time::Instant};

use ethers::{
    providers::{Middleware, Provider},
    types::{
        transaction::eip2718::TypedTransaction, BlockNumber, GethDebugTracingOptions,
        TransactionRequest,
    },
    utils,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TraceConfig {
    pub disable_storage: bool,
    pub disable_stack: bool,
    pub enable_memory: bool,
    pub enable_return_data: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracer: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let block = BlockNumber::Latest;
    let block = utils::serialize(&block);
    let config = TraceConfig {
        disable_storage: true,
        disable_stack: true,
        enable_memory: false,
        enable_return_data: false,
        tracer: None,
    };
    let config = utils::serialize(&config);
    let now = Instant::now();
    let _res = provider_ipc
        .request("debug_traceBlockByNumber", [block, config])
        .await?;
    println!("TIME ELAPSED: {:?}ms", now.elapsed().as_millis());
    Ok(())
}
