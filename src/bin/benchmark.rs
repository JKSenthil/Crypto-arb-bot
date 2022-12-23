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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let block = BlockNumber::Latest;
    let block = utils::serialize(&block);
    let now = Instant::now();
    let _res = provider_ipc
        .request("debug_traceBlockByNumber", [block])
        .await?;
    println!("TIME ELAPSED: {:?}ms", now.elapsed().as_millis());
    Ok(())
}
