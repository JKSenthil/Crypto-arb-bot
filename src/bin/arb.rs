use dotenv::dotenv;
use ethers::providers::{Provider, Ws};
use ethers::{prelude::abigen, types::Address};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;

use tsuki::constants::protocol::*;
use tsuki::constants::token::USDC;

// #[derive(Serialize, Deserialize, Debug)]
// pub struct ERC20Token {
//     pub address: Address,
//     pub name: String,
//     pub symbol: String,
//     pub decimals: u8,
// }

abigen!(IUniswapV2Router02, "abis/IUniswapV2Router02.json");
abigen!(IUniswapV2Factory, "abis/IUniswapV2Factory.json");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // let token_list_data = fs::read_to_string("data/polygon_tokens.json")?;
    // let token_list: Vec<ERC20Token> = serde_json::from_str(&token_list_data)?;

    // println!("Num tokens: {}", token_list.len());
    // println!("{:?}", token_list[1]);

    // println!("{:?}", USDC);

    let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await?;
    let provider_ws = Arc::new(provider_ws);

    let router = IUniswapV2Router02::new(SUSHISWAP.get_router_address(), provider_ws.clone());
    let factory = IUniswapV2Factory::new(SUSHISWAP.get_factory_address(), provider_ws);

    Ok(())
}
