use lazy_static::lazy_static;
use std::sync::Arc;

use dotenv::dotenv;
use ethers::{
    prelude::SignerMiddleware,
    providers::{Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Bytes, Transaction, U256},
    utils::{self, rlp},
};
use futures_util::StreamExt;
use tsuki::{tx_pool::TxPool, utils::block::Block};

lazy_static! {
    static ref ELASTICITY_MULTIPLIER: U256 = U256::from(2);
    static ref BASE_FEE_CHANGE_DENOMINATOR: U256 = U256::from(8);
}

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
        let base_fee_delta = U256::max(
            y.checked_div(*BASE_FEE_CHANGE_DENOMINATOR).unwrap(),
            U256::one(),
        );
        return current_base_fee - base_fee_delta;
    }
}

fn predict_next_block(current_block: Block, mempool_txns: Vec<Transaction>) -> Option<Block> {
    // TODO predict next block gas_limit
    // TODO compute next base_fee
    // current_block.header.
    // let base_fee = U256::zero();
    // let mempool_txns: Vec<Transaction> = mempool_txns
    //     .into_iter()
    //     .filter(|txn| txn.max_fee_per_gas.unwrap() > base_fee)
    //     .collect();

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

    // TODO wait 20 seconds
    let mut block_stream = provider_ipc.subscribe_blocks().await.unwrap();
    while let Some(block) = block_stream.next().await {
        /*
        1) predict next block
        2) simulate next block w/ our transactions
        3) If arb, then execute transaction
        */

        println!("actual base fee: {}", block.base_fee_per_gas.unwrap());

        // 1) predict next block
        let block_number = block.number.unwrap();
        let block_number = utils::serialize(&block_number);

        let bytes = provider_ipc
            .request::<_, Bytes>("debug_getBlockRlp", [block_number])
            .await?;

        let current_block: Block = rlp::decode(&bytes)?;
        let next_base_fee = compute_next_base_fee(
            current_block.header.base_fee_per_gas.unwrap(),
            current_block.header.gas_used,
            current_block.header.gas_limit,
        );
        println!("predicted base fee: {}", next_base_fee);
        // let next_block = predict_next_block(current_block, txpool.get_mempool().await);
    }
    Ok(())
}
