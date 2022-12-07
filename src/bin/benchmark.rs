use std::sync::Arc;

use ethers::{
    providers::{Middleware, Provider},
    types::GethDebugTracingOptions,
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let mut pending_tx_stream = provider_ipc.subscribe_pending_txs().await?;
    while let Some(tx_hash) = pending_tx_stream.next().await {
        let b = provider_ipc
            .debug_trace_transaction(
                tx_hash,
                GethDebugTracingOptions {
                    disable_storage: None,
                    disable_stack: None,
                    enable_memory: None,
                    enable_return_data: None,
                    tracer: None,
                    timeout: None,
                },
            )
            .await;
        match b {
            Ok(trace) => println!("TRACE: {:?}", trace),
            Err(e) => println!("ERROR: {:?}", e),
        };
    }

    Ok(())
}
