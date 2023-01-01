use dotenv::dotenv;
use ethers::prelude::SignerMiddleware;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{BigEndianHash, BlockNumber, H256, H64};
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
use tsuki::constants::protocol::UniswapV2;
use tsuki::tx_pool::TxPool;
use tsuki::uniswapV2::UniswapV2Client;
use tsuki::utils::block::{self, Block, PartialHeader};
use tsuki::utils::transaction::{EIP1559Transaction, EIP2930Transaction, TypedTransaction};

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

// TODO manually copy over from one txn type to the other.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let wallet = std::env::var("PRIVATE_KEY")
        .unwrap()
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(137u64);
    let signer_client = SignerMiddleware::new(provider_ipc.clone(), wallet);

    // generate one transaction, see what happens
    let uniswap_client = UniswapV2Client::new(provider_ipc.clone());
    let mut txn = uniswap_client
        .get_quote_txn(
            UniswapV2::SUSHISWAP,
            tsuki::constants::token::ERC20Token::USDC,
            tsuki::constants::token::ERC20Token::USDT,
            U256::from(1_000_000),
        )
        .tx;

    txn.set_from(signer_client.address());
    txn.set_chain_id(137);
    txn.set_nonce(
        signer_client
            .get_transaction_count(signer_client.address(), None)
            .await?,
    );
    txn.set_gas_price(provider_ipc.get_gas_price().await?);
    let signature = signer_client.signer().sign_transaction(&txn).await?;
    let txn = txn.as_eip1559_ref().unwrap();
    let txn: EIP1559Transaction = tsuki::utils::transaction::EIP1559Transaction {
        chain_id: txn.chain_id.unwrap().as_u64(),
        nonce: txn.nonce.unwrap(),
        max_priority_fee_per_gas: txn.max_priority_fee_per_gas.unwrap(),
        max_fee_per_gas: txn.max_fee_per_gas.unwrap(),
        gas_limit: 2_000_000.into(),
        kind: tsuki::utils::transaction::TransactionKind::Call(
            UniswapV2::SUSHISWAP.get_router_address(),
        ),
        value: U256::zero(),
        input: txn.data.clone().unwrap(),
        access_list: txn.access_list.clone(),
        odd_y_parity: false,
        r: H256::from_uint(&signature.r),
        s: H256::from_uint(&signature.s),
    };
    // let tx = txn.rlp_signed(&signature);

    // let txn: TypedTransaction = rlp::decode(&tx)?;

    let block_number = provider_ipc.get_block_number().await?.as_u64();
    let block_number = utils::serialize(&block_number);

    let bytes = provider_ipc
        .request::<_, Bytes>("debug_getBlockRlp", [block_number])
        .await?;

    let block: Block = rlp::decode(&bytes)?;
    let parent_hash = block.header.hash();

    let next_partial_header = PartialHeader {
        parent_hash: parent_hash,
        beneficiary: block.header.beneficiary,
        state_root: block.header.state_root,
        receipts_root: block.header.receipts_root,
        logs_bloom: block.header.logs_bloom,
        difficulty: block.header.difficulty,
        number: block.header.number + 1,
        gas_limit: block.header.gas_limit,
        gas_used: block.header.gas_used,
        timestamp: block.header.timestamp,
        extra_data: block.header.extra_data,
        mix_hash: block.header.mix_hash,
        nonce: H64::zero(),
        base_fee: block.header.base_fee_per_gas,
    };

    let next_block = Block::new(
        next_partial_header,
        vec![tsuki::utils::transaction::TypedTransaction::EIP1559(txn)],
        vec![],
    );

    let next_block_rlp = rlp::encode(&next_block);
    let next_block_rlp = ["0x", &hex::encode(next_block_rlp)].join("");
    let next_block_rlp = utils::serialize(&next_block_rlp);

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
        .request::<_, Vec<Res>>("debug_traceBlock", [next_block_rlp, config])
        .await?;

    println!("Number in result: {:?}", result.len());
    println!("{:?}", result);
    Ok(())
}

async fn txpool() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);
    let txpool = TxPool::init(provider_ipc.clone(), 1000);
    let txpool = Arc::new(txpool);
    txpool.stream_mempool().await;
    Ok(())
}

#[tokio::main]
async fn txpool_content() -> Result<(), Box<dyn std::error::Error>> {
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
