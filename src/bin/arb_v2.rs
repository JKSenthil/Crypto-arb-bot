use lazy_static::lazy_static;
use std::{
    collections::{BinaryHeap, HashMap},
    sync::Arc,
    time::Instant,
};

use dotenv::dotenv;
use ethers::{
    prelude::{k256::ecdsa::SigningKey, SignerMiddleware},
    providers::{Ipc, Middleware, Provider},
    signers::{LocalWallet, Signer, Wallet},
    types::{Address, Bytes, Transaction, U256},
    utils::{self, hex, rlp},
};
use futures_util::StreamExt;
use tokio::time::Duration;
use tsuki::{
    tx_pool::TxPool,
    utils::{
        block::Block,
        serialize_structs::{Res, TraceConfig, TracerConfig},
        transaction::{build_typed_transaction, EthTransactionRequest, TypedTransaction},
        txstructs::TxLinkedList,
    },
};

lazy_static! {
    static ref ELASTICITY_MULTIPLIER: U256 = U256::from(2);
    static ref BASE_FEE_CHANGE_DENOMINATOR: U256 = U256::from(8);
    static ref MIN_GAS_LIMIT: U256 = U256::from(500);
    static ref GAS_LIMIT_BOUND_DIVISOR: U256 = U256::from(1024);
    static ref DESIRED_GAS_LIMIT: U256 = U256::from(30_000_000);
}

// https://github.com/maticnetwork/bor/blob/ad69ccd0ba6aac4a690e6b4778987242609f4845/core/block_validator.go#L108
fn compute_next_gas_limit(current_gas_limit: U256) -> U256 {
    let delta = current_gas_limit
        .checked_div(*GAS_LIMIT_BOUND_DIVISOR)
        .unwrap()
        - 1;
    let mut limit = current_gas_limit;
    if current_gas_limit < *DESIRED_GAS_LIMIT {
        limit = current_gas_limit + delta;
        if limit > *DESIRED_GAS_LIMIT {
            limit = *DESIRED_GAS_LIMIT;
        }
    } else if current_gas_limit > *DESIRED_GAS_LIMIT {
        limit = current_gas_limit - delta;
        if limit < *DESIRED_GAS_LIMIT {
            limit = *DESIRED_GAS_LIMIT;
        }
    }
    return limit;
}

// https://github.com/maticnetwork/bor/blob/ad69ccd0ba6aac4a690e6b4778987242609f4845/consensus/misc/eip1559.go#L99
fn compute_next_base_fee(current_base_fee: U256, gas_used: U256, gas_limit: U256) -> U256 {
    let gas_target = gas_limit.checked_div(*ELASTICITY_MULTIPLIER).unwrap();
    if gas_used == gas_target {
        return current_base_fee;
    } else if gas_used > gas_target {
        let gas_used_delta = gas_used - gas_target;
        let x = current_base_fee.checked_mul(gas_used_delta).unwrap();
        let y = x.checked_div(gas_target).unwrap();
        let base_fee_delta = U256::max(
            y.checked_div(*BASE_FEE_CHANGE_DENOMINATOR).unwrap(),
            U256::one(),
        );
        return current_base_fee + base_fee_delta;
    } else {
        let gas_used_delta = gas_target - gas_used;
        let x = current_base_fee.checked_mul(gas_used_delta).unwrap();
        let y = x.checked_div(gas_target).unwrap();
        let base_fee_delta = y.checked_div(*BASE_FEE_CHANGE_DENOMINATOR).unwrap();

        return current_base_fee - base_fee_delta;
    }
}

fn heapify_mempool(mut mempool_txns: Vec<Transaction>) -> BinaryHeap<TxLinkedList> {
    mempool_txns.sort_by(|a, b| a.nonce.cmp(&b.nonce));
    let mut mapping: HashMap<Address, TxLinkedList> = HashMap::new();
    for txn in mempool_txns {
        let sender_address = txn.from;
        if !mapping.contains_key(&sender_address) {
            mapping.insert(sender_address, TxLinkedList::new());
        }
        mapping
            .get_mut(&sender_address)
            .unwrap()
            .linked_list
            .push_back(txn);
    }
    let mut heap = BinaryHeap::<TxLinkedList>::new();

    for (_, lls) in mapping {
        heap.push(lls);
    }

    return heap;
}

// https://github.com/maticnetwork/bor/blob/ad69ccd0ba6aac4a690e6b4778987242609f4845/miner/worker.go#L942
fn filter_mempool(mempool_txns: Vec<Transaction>, next_base_fee: U256) -> Vec<Transaction> {
    let mut heap = heapify_mempool(mempool_txns);
    let mut final_txns: Vec<Transaction> = Vec::new();
    while heap.len() != 0 {
        let mut ll = heap.pop().unwrap();
        let txn = ll.linked_list.pop_front().unwrap();
        let gas_fee = match txn.max_fee_per_gas {
            Some(val) => val,
            None => txn.gas_price.unwrap(),
        };

        if gas_fee < next_base_fee || gas_fee < U256::from(22916) {
            continue;
        }

        // https://github.com/maticnetwork/bor/blob/ad69ccd0ba6aac4a690e6b4778987242609f4845/core/types/transaction.go#L426
        if let Some(max_priority_gas_fee) = txn.max_priority_fee_per_gas {
            let tip = txn.max_fee_per_gas.unwrap() - next_base_fee;
            if max_priority_gas_fee < tip {
                continue;
            }
        }

        if let Some(txn_next) = ll.linked_list.front() {
            if txn.nonce + 1 == txn_next.nonce {
                heap.push(ll);
            }
        }
        final_txns.push(txn);
    }
    final_txns
}

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

    let txpool = TxPool::init(provider_ipc.clone(), 1000);
    let txpool = Arc::new(txpool);
    tokio::spawn(txpool.clone().stream_mempool());

    // wait 10 seconds for local mempool to populate
    tokio::time::sleep(Duration::from_secs(3)).await;

    // TODO: filter out transactions with gas below 22916

    let mut block_stream = provider_ipc.subscribe_blocks().await.unwrap();
    while let Some(block) = block_stream.next().await {
        /*
        1) predict next block
        2) simulate next block w/ our transactions
        3) If arb, then execute transaction
        */

        // 1) predict next block
        let block_number = block.number.unwrap();
        let block_number = utils::serialize(&(block_number.as_u64()));

        let bytes = provider_ipc
            .request::<_, Bytes>("debug_getBlockRlp", [block_number])
            .await?;
        let current_block: Block = rlp::decode(&bytes)?;

        let next_base_fee = compute_next_base_fee(
            current_block.header.base_fee_per_gas.unwrap(),
            current_block.header.gas_used,
            current_block.header.gas_limit,
        );

        let mempool_txns = txpool.get_mempool().await;
        let mempool_txns = filter_mempool(mempool_txns, next_base_fee);
        let mempool_txns: Vec<TypedTransaction> = mempool_txns
            .into_iter()
            .map(|t| TypedTransaction::from(t))
            .collect();

        let mut txns = current_block.transactions;
        // let rlp_bytes = txn.rlp();
        // let txn: TypedTransaction = rlp::decode(&rlp_bytes).unwrap();
        txns.extend(mempool_txns);
        let sim_block: Block = Block::new(current_block.header.into(), txns, current_block.ommers);
        let sim_block_rlp = rlp::encode(&sim_block);
        let sim_block_rlp = ["0x", &hex::encode(sim_block_rlp)].join("");
        let sim_block_rlp = utils::serialize(&sim_block_rlp);

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
        let result = provider_ipc
            .request::<_, Vec<Res>>("debug_traceBlock", [sim_block_rlp, config])
            .await?;
        println!("Time elapsed: {}ms", now.elapsed().as_millis());
        println!("Number in result: {:?}", result.len());
    }
    Ok(())
}
