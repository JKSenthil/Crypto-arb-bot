use ethers::types::{Transaction, TxHash};
use ethers::utils::{hex, rlp};
use ethers::{
    providers::{Middleware, Provider},
    types::{Address, Bytes, GethDebugTracingOptions, TransactionRequest, U256},
    utils,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::Arc, time::Instant};
use tsuki::utils::block::Block;

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
pub struct Res {
    pub result: BlockTraceResult,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockTraceResult {
    pub from: Address,
    pub gas: U256,
    pub gas_used: U256,
    pub input: Bytes,
    pub output: Bytes,
    pub to: Address,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub value: U256,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls: Option<Vec<BlockTraceResult>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let block_number = provider_ipc.get_block_number().await?.as_u64();
    let block_number = utils::serialize(&block_number);

    let bytes = provider_ipc
        .request::<_, Bytes>("debug_getBlockRlp", [block_number])
        .await?;

    let block: Block = rlp::decode(&bytes)?;
    println!("Number of txns: {:?}", block.transactions.len());
    // let block_rlp = rlp::encode(&block);
    let block_rlp = ["0x", &hex::encode(bytes)].join("");
    // println!("{:?}", block_rlp);
    let block_rlp = utils::serialize(&block_rlp);

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

    let result = provider_ipc
        .request::<_, Vec<Res>>("debug_traceBlock", [block_rlp, config])
        .await?;

    println!("Number in result: {:?}", result.len());
    println!("{:?}", result);

    Ok(())
}

async fn debug_traceBlockByNumber() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let block_number = provider_ipc.get_block_number().await?;
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
    let mut results = vec![];
    let now = Instant::now();
    for i in 0..4 {
        let block_number = utils::serialize(&(block_number - i));
        let config = utils::serialize(&config);
        results.push(provider_ipc.request::<_, Vec<BlockTraceResult>>(
            "debug_traceBlockByNumber",
            [block_number, config],
        ));
    }
    for result in results {
        let _res = result.await?;
    }
    println!("TIME ELAPSED: {:?}ms", now.elapsed().as_millis());
    Ok(())
}
