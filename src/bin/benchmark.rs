use ethers::types::H256;
use ethers::types::{Transaction, TxHash, U64};
use ethers::utils::{hex, rlp};
use ethers::{
    providers::{Middleware, Provider},
    types::{Address, Bytes, GethDebugTracingOptions, TransactionRequest, U256},
    utils,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::{sync::Arc, time::Instant};
use tsuki::utils::block::{self, Block};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Bytes>,
    pub to: Address,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls: Option<Vec<BlockTraceResult>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TxpoolEntry {
    pub hash: H256,
    pub gas_price: U256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TxpoolContent {
    pub pending: HashMap<Address, HashMap<U256, TxpoolEntry>>,
    pub queued: HashMap<Address, HashMap<U256, TxpoolEntry>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let mut block_stream = provider_ipc.subscribe_blocks().await.unwrap();
    let start_block_num = provider_ipc.get_block_number().await?;
    let mut pending_txn_hashs = HashSet::<H256>::new();
    let mut gas_prices = Vec::<U256>::new();
    let mut mapping: HashMap<H256, U256> = HashMap::new();

    while let Some(block) = block_stream.next().await {
        if block.number.unwrap() == start_block_num + 2 {
            // pull mempool transactions
            let content = provider_ipc
                .request::<_, TxpoolContent>("txpool_content", ())
                .await?;
            let pending = content.pending;
            for (_address, nonce_map) in pending {
                for (_nonce, entry) in nonce_map {
                    pending_txn_hashs.insert(entry.hash);
                    gas_prices.push(entry.gas_price);
                    mapping.insert(entry.hash, entry.gas_price);
                }
            }
        } else if block.number.unwrap() == start_block_num + 3 {
            let mut local_gas_prices = Vec::<U256>::new();
            let block = provider_ipc
                .get_block(block.number.unwrap())
                .await?
                .unwrap();
            let txns = block.transactions;
            let num_txns = txns.len();
            let mut num_txns_in_mempool = 0;
            for txn_hash in txns {
                if pending_txn_hashs.contains(&txn_hash) {
                    num_txns_in_mempool += 1;
                    local_gas_prices.push(mapping[&txn_hash]);
                }
            }
            println!(
                "{}/{} transactions from mempool were in mined blocked.",
                num_txns_in_mempool, num_txns
            );
            gas_prices.sort();
            gas_prices.reverse();
            println!("--------------");
            println!("Local gas prices: {:?}", local_gas_prices);
            println!("______________");
            println!(
                "Mempool gas prices: {:?}",
                gas_prices[0..local_gas_prices.len()].to_vec()
            );

            break;
        }
    }
    Ok(())
}

async fn debug_traceBlock() -> Result<(), Box<dyn std::error::Error>> {
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
