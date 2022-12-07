use std::{sync::Arc, time::Instant};

use ethers::{
    providers::{Middleware, Provider},
    types::{transaction::eip2718::TypedTransaction, GethDebugTracingOptions, TransactionRequest},
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let mut pending_tx_stream = provider_ipc
        .subscribe_pending_txs()
        .await?
        .transactions_unordered(16);
    while let Ok(tx) = pending_tx_stream.next().await.unwrap() {
        let now = Instant::now();
        let typed_tx: TypedTransaction = (&tx).into();
        let result = provider_ipc.call(&typed_tx, None).await;
        match result {
            Ok(b) => println!("WORKS! {:?}", b),
            Err(e) => println!("ERROR! {:?}", e),
        };
        println!("Time elapsed: {:?}ms", now.elapsed().as_millis());
    }

    Ok(())
}
