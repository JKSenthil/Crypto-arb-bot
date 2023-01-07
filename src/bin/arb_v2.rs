use lazy_static::lazy_static;
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    sync::Arc,
    time::Instant,
    vec,
};

use dotenv::dotenv;
use ethers::{
    prelude::{k256::ecdsa::SigningKey, SignerMiddleware},
    providers::{JsonRpcClient, Middleware, Provider},
    signers::{LocalWallet, Signer, Wallet},
    types::{Address, Bytes, Transaction, H256, U256},
    utils::{self, hex, rlp},
};
use futures_util::StreamExt;
use tsuki::{
    tx_pool::TxPool,
    utils::{
        batch::{common::BatchRequest, custom_ipc::Ipc, BatchProvider},
        block::{Block, Header, PartialHeader},
        block_oracle::BlockOracle,
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
// TODO: need to account for https://github.com/maticnetwork/bor/blob/ad69ccd0ba6aac4a690e6b4778987242609f4845/miner/worker.go#L1020
// where if gas limit is reached it ignores the other transactions from same sender (may impact our block predict algorithm)
fn filter_mempool(
    mempool_txns: Vec<Transaction>,
    mut account_nonces: HashMap<Address, U256>,
    next_base_fee: U256,
) -> (Vec<Transaction>, Vec<Transaction>) {
    let mut heap = heapify_mempool(mempool_txns);
    let mut rejected_txns: Vec<Transaction> = Vec::new();
    let mut final_txns: Vec<Transaction> = Vec::new();
    while heap.len() != 0 {
        let mut ll = heap.pop().unwrap();
        let txn = ll.linked_list.pop_front().unwrap();
        let current_account_nonce = account_nonces.get(&txn.from).unwrap();
        if txn.nonce < *current_account_nonce {
            rejected_txns.push(txn);
            if ll.linked_list.front().is_some() {
                heap.push(ll);
            }
            continue;
        } else if txn.nonce > *current_account_nonce {
            continue;
        } else {
            account_nonces.insert(txn.from, *current_account_nonce + 1);
        }

        let gas_fee = match txn.max_fee_per_gas {
            Some(val) => val,
            None => txn.gas_price.unwrap(),
        };

        if gas_fee < next_base_fee || gas_fee < U256::from(22916) {
            continue;
        }

        // https://github.com/maticnetwork/bor/blob/ad69ccd0ba6aac4a690e6b4778987242609f4845/core/types/transaction.go#L426
        if let Some(max_priority_gas_fee) = txn.max_priority_fee_per_gas {
            let tip = gas_fee - next_base_fee;
            if max_priority_gas_fee < tip {
                continue;
            }
        }

        if ll.linked_list.front().is_some() {
            heap.push(ll);
        }
        final_txns.push(txn);
    }
    (final_txns, rejected_txns)
}

async fn retrieve_account_nonces(
    batch_provider_ipc: &BatchProvider<Ipc>,
    txns: &Vec<Transaction>,
) -> HashMap<Address, U256> {
    let mut batch = BatchRequest::new();
    let mut addresses: Vec<Address> = Vec::new();
    let mut seen: HashSet<Address> = HashSet::new();
    let mut result: HashMap<Address, U256> = HashMap::new();
    for txn in txns {
        let address = txn.from;
        if !seen.contains(&address) {
            batch
                .add_request("eth_getTransactionCount", (address, "latest"))
                .unwrap();
            seen.insert(address);
            addresses.push(address);
        }
    }
    let mut i = 0;
    let mut responses = batch_provider_ipc.execute_batch(&mut batch).await.unwrap();
    while let Some(Ok(num)) = responses.next_response::<U256>() {
        result.insert(addresses[i], num);
        i += 1;
    }
    return result;
}

async fn debug_traceBlock<M: JsonRpcClient>(
    provider_ipc: Arc<Provider<M>>,
    header: Header,
    transactions: Vec<TypedTransaction>,
) -> Vec<Res> {
    let sim_block: Block = Block::new(header.into(), transactions, vec![]);
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

    // provider_ipc.get_transaction_count(from, block);
    let result = provider_ipc
        .request::<_, Vec<Res>>("debug_traceBlock", [sim_block_rlp, config])
        .await
        .unwrap();
    return result;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);
    let batch_provider_ipc = BatchProvider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;

    // let wallet = std::env::var("PRIVATE_KEY")
    //     .unwrap()
    //     .parse::<LocalWallet>()
    //     .unwrap()
    //     .with_chain_id(137u64);
    // let signer_client = SignerMiddleware::new(provider_ipc.clone(), wallet);

    let txpool = TxPool::init(provider_ipc.clone(), 1000);
    let txpool = Arc::new(txpool);
    tokio::spawn(txpool.clone().stream_mempool());

    let start_block_number = provider_ipc.get_block_number().await?;
    let mut block_stream = provider_ipc.subscribe_blocks().await.unwrap();
    while let Some(block) = block_stream.next().await {
        // wait for three blocks to warm up mempool
        if block.number.unwrap() <= start_block_number + 3 {
            let block = provider_ipc
                .get_block(block.number.unwrap())
                .await?
                .unwrap();
            // update local mempool
            let mut txn_hashes: Vec<H256> = Vec::new();
            for hash in &block.transactions {
                txn_hashes.push(*hash);
            }
            let num_removed = txpool.remove_transactions(txn_hashes).await;
            println!(
                "Num txns removed from mempool while warming up: {}",
                num_removed
            );
            continue;
        }

        /*
        1) predict next block
        2) simulate next block w/ our transactions
            -> TODO filter out transactions with no data
            -> and find other ways to make our simulated block smaller for speed
        3) If arb, then execute transaction
        */

        let now = Instant::now();

        let oracle_cache_size = 5 as usize;
        let mut block_oracle = BlockOracle::new(oracle_cache_size);

        // pull next block details
        let block_number = block.number.unwrap();
        let block_number = utils::serialize(&(block_number.as_u64()));

        // get the current block
        let bytes = provider_ipc
            .request::<_, Bytes>("debug_getBlockRlp", [block_number])
            .await?;
        let current_block: Block = rlp::decode(&bytes)?;

        // add current block copy to oracle and verify previous prediction
        block_oracle.append_block(current_block.clone());
        block_oracle.display_accuracy();

        let block_rlp_now = Instant::now();

        let next_base_fee = compute_next_base_fee(
            current_block.header.base_fee_per_gas.unwrap(),
            current_block.header.gas_used,
            current_block.header.gas_limit,
        );

        // update local mempool
        let mut txn_hashes: Vec<H256> = Vec::new();
        for txn in &current_block.transactions {
            txn_hashes.push(txn.hash());
        }
        let num_removed = txpool.remove_transactions(txn_hashes).await;
        println!("Num txns removed from mempool: {}", num_removed);

        let mempool_txns = txpool.get_mempool().await;
        let account_nonces = retrieve_account_nonces(&batch_provider_ipc, &mempool_txns).await;
        let nonce_now = Instant::now();

        let (mempool_txns, rejected_txns) =
            filter_mempool(mempool_txns, account_nonces, next_base_fee);
        let num_removed = txpool
            .remove_transactions(rejected_txns.into_iter().map(|t| t.hash()).collect())
            .await;
        println!(
            "Num txns removed from mempool after block sim: {}",
            num_removed
        );

        // convert all mempool tx into TypedTransaction
        let mempool_txns: Vec<TypedTransaction> = mempool_txns
            .into_iter()
            .map(|t| TypedTransaction::from(t))
            .collect();

        // add state of mempool to current block
        let mut txn_list: Vec<TypedTransaction> = current_block.transactions;
        txn_list.extend(mempool_txns);

        // use our prediction algo and compare with previously known block
        block_oracle.predict_next_block(txn_list.clone());

        // simulate that block
        let result = debug_traceBlock(provider_ipc.clone(), current_block.header, txn_list).await;
        println!(
            "First Block: {}ms, Batch nonce call: {}ms, Total Time elapsed: {}ms",
            (block_rlp_now - now).as_millis(),
            (nonce_now - block_rlp_now).as_millis(),
            now.elapsed().as_millis()
        );
        println!("Number in result: {:?}", result.len());
        println!("\n\n");
    }
    Ok(())
}
