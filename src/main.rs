#![allow(dead_code)]

use dotenv::dotenv;
use ethers::prelude::*;
use price::Price;
use std::vec;

mod consts;
mod price;
mod utils;

use consts::ERC20Token::*;
use consts::Protocol::*;

async fn pull_latest_block_hash() {
    dotenv().ok();

    let polygon_ws_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_WS_URL").expect("Polygon RPC url expected in .env file");

    let provider = Provider::<Ws>::connect(polygon_ws_url).await.unwrap();

    // filter by latest block
    let _filter = Filter::new().select(BlockNumber::Latest);

    // provider.watch(&filter);
    let mut watch_block_stream = provider.watch_blocks().await.unwrap().fuse();

    loop {
        futures_util::select! {
            tx = watch_block_stream.next() => {
                println!("New Block {:#?}", tx.unwrap());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let polygon_rpc_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_URL").expect("Polygon RPC url expected in .env file");
    let provider = Provider::<Http>::try_from(polygon_rpc_url).unwrap();
    let price = Price::new(provider);

    let ans = price
        .quote(UNISWAP_V3, USDC, DAI, U256::from(1000000))
        .await;

    println!("Dai returned: {}", ans);

    let ans = price
        .quote_route(UNISWAP_V3, vec![USDC, DAI, USDC], U256::from(1000000))
        .await;

    println!("USDC returned: {}", ans);
}
