use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Arc};

use dotenv::dotenv;
use ethers::{
    prelude::SignerMiddleware,
    providers::{Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, Transaction, U256},
    utils::{self, rlp},
};
use futures_util::StreamExt;
use tokio::time::Duration;
use tsuki::{tx_pool::TxPool, utils::block::Block};

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

fn group_txns_by_sender(mempool_txns: Vec<Transaction>) -> HashMap<Address, Vec<Transaction>> {
    let mut mapping: HashMap<Address, Vec<Transaction>> = HashMap::new();
    for txn in mempool_txns {
        let sender_address = txn.from;
        if !mapping.contains_key(&sender_address) {
            mapping.insert(sender_address, Vec::new());
        }
        mapping.get_mut(&sender_address).unwrap().push(txn);
    }

    // sort by nonce
    for txns in mapping.values_mut() {
        txns.sort_by(|a, b| a.nonce.cmp(&b.nonce));
    }
    return mapping;
}

fn predict_next_block(current_block: Block, mempool_txns: Vec<Transaction>) -> Option<Block> {
    let next_base_fee = compute_next_base_fee(
        current_block.header.base_fee_per_gas.unwrap(),
        current_block.header.gas_used,
        current_block.header.gas_limit,
    );
    let next_gas_limit = compute_next_gas_limit(current_block.header.gas_limit);

    None
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
    tokio::time::sleep(Duration::from_secs(10)).await;

    let mut block_stream = provider_ipc.subscribe_blocks().await.unwrap();
    while let Some(block) = block_stream.next().await {
        /*
        1) predict next block
        2) simulate next block w/ our transactions
        3) If arb, then execute transaction
        */

        break;

        // 1) predict next block
        let block_number = block.number.unwrap();
        let block_number = utils::serialize(&(block_number.as_u64()));

        let bytes = provider_ipc
            .request::<_, Bytes>("debug_getBlockRlp", [block_number])
            .await?;

        let current_block: Block = rlp::decode(&bytes)?;
    }
    Ok(())
}
