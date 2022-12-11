use std::sync::Arc;

use ethers::{
    providers::{Provider, Ws},
    types::{transaction::eip2718::TypedTransaction, U256},
};
use tsuki::constants::{protocol::UniswapV2, token::ERC20Token};
use tsuki::uniswapV2::UniswapV2Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    let provider_ws = Arc::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);

    let client = UniswapV2Client::new(provider_ws.clone());
    let tx = client.get_swapExactTokensForTokens_txn(
        UniswapV2::QUICKSWAP,
        ERC20Token::USDC,
        ERC20Token::USDT,
        U256::from(1_000_000),
    );

    let data = tx.call().await;
    match data {
        Ok(b) => println!("WORKS! {:?}", b),
        Err(e) => println!("ERROR! {:?}", e),
    };

    Ok(())
}
