use std::{sync::Arc, time::Instant};

use ethers::{
    providers::{Middleware, Provider},
    types::{
        transaction::eip2718::TypedTransaction, BlockNumber, GethDebugTracingOptions,
        TransactionRequest,
    },
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let latest_block_number = provider_ipc.get_block_number().await?;
    let now = Instant::now();
    let _ = provider_ipc
        .trace_block(BlockNumber::Number(latest_block_number))
        .await?;
    println!("TIME ELAPSED: {:?}ms", now.elapsed().as_millis());
    Ok(())
}
