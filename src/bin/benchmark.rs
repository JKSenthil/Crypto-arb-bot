use std::{sync::Arc, time::Instant};

use ethers::{
    providers::{Middleware, Provider},
    types::{
        transaction::eip2718::TypedTransaction, Address, BlockNumber, Bytes,
        GethDebugTracingOptions, TransactionRequest, U256,
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
    pub tracer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracer_config: Option<TracerConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TracerConfig {
    pub only_top_call: bool,
    pub with_log: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockTraceResult {
    pub from: Address,
    pub gas: U256,
    pub gas_used: U256,
    pub input: Bytes,
    pub output: Bytes, // TODO right?
    pub to: Address,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls: Option<Vec<BlockTraceResult>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let block_number = provider_ipc.get_block_number().await?;
    let block_number = utils::serialize(&block_number);
    let config = TraceConfig {
        disable_storage: true,
        disable_stack: true,
        enable_memory: false,
        enable_return_data: false,
        tracer: "callTracer".to_string(),
        tracer_config: Some(TracerConfig {
            only_top_call: true,
            with_log: false,
        }),
    };
    let config = utils::serialize(&config);
    let now = Instant::now();
    let _res: Vec<BlockTraceResult> = provider_ipc
        .request("debug_traceBlockByNumber", [block_number, config])
        .await?;
    println!("TIME ELAPSED: {:?}ms", now.elapsed().as_millis());
    Ok(())
}
