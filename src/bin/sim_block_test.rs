use std::sync::Arc;

use ethers::providers::{Middleware, Provider, Ws};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ws = Arc::new(provider_ipc);
    let mut block_stream = provider_ws.subscribe_blocks().await.unwrap();
    while let Some(block) = block_stream.next().await {
        println!("{:?}", block);
        break;
    }
    Ok(())
}
