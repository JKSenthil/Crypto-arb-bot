#![allow(dead_code)]

use dotenv::dotenv;
use ethers::{prelude::*, providers};
use price::Price;
use std::sync::Arc;
use std::{str, vec};

mod consts;
mod price;
mod utils;

abigen!(IUniswapV2Router02, "abis/IUniswapV2Router02.json");
abigen!(Quoter, "abis/Quoter.json");

enum DEX {
    SUSHISWAP,
    QUICKSWAP,
    JETSWAP,
    POLYCAT,
    APESWAP,
}

impl DEX {
    const ROUTER_ADDRESSES: &'static [&'static str] = &[
        "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
        "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
        "0x5C6EC38fb0e2609672BDf628B1fD605A523E5923",
        "0x94930a328162957FF1dd48900aF67B5439336cBD",
        "0xC0788A3aD43d79aa53B09c2EaCc313A787d1d607",
    ];

    fn get_router_address(self) -> &'static str {
        return DEX::ROUTER_ADDRESSES[self as usize];
    }
}

fn str_to_addr(addr: &str) -> H160 {
    addr.parse::<Address>().unwrap()
}

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

async fn swap_test() {
    dotenv().ok();
    let polygon_rpc_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_URL").expect("Polygon RPC url expected in .env file");

    let client = Provider::<Http>::try_from(polygon_rpc_url).unwrap();
    let client = Arc::new(client);

    let router_addr = str_to_addr(DEX::SUSHISWAP.get_router_address());

    let sushiswap = IUniswapV2Router02::new(router_addr, Arc::clone(&client));

    let usdc_addr = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
        .parse::<Address>()
        .unwrap();

    let dai_addr = "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
        .parse::<Address>()
        .unwrap();

    let tokens = vec![usdc_addr, dai_addr, usdc_addr];

    let result = sushiswap
        .get_amounts_out(U256::from(10_i32.pow(6)), tokens)
        .call()
        .await
        .unwrap();

    println!("Index 0: {}", result[0]);
    println!("Index 1: {}", result[1]);
    println!("Index 2: {}", result[2]);
}

async fn uniswap_price_v3() {
    dotenv().ok();
    let polygon_rpc_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_URL").expect("Polygon RPC url expected in .env file");
    let client = Provider::<Http>::try_from(polygon_rpc_url).unwrap();
    let client = Arc::new(client);

    let quoter_address = str_to_addr("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6");
    let router = Quoter::new(quoter_address, Arc::clone(&client));

    let usdc_addr = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
        .parse::<Address>()
        .unwrap();

    let dai_addr = "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
        .parse::<Address>()
        .unwrap();

    let result = router
        .quote_exact_input_single(
            usdc_addr,
            dai_addr,
            500_u32,
            U256::from(1000000),
            U256::zero(),
        )
        .call()
        .await
        .unwrap();

    println!("Output is: {}", result);
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let polygon_rpc_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_URL").expect("Polygon RPC url expected in .env file");
    let provider = Provider::<Http>::try_from(polygon_rpc_url).unwrap();
    let protocol = Price::new(provider);

    let ans = protocol
        .quote(
            consts::Protocol::SUSHISWAP,
            consts::ERC20Token::USDC,
            consts::ERC20Token::DAI,
            U256::from(1000000),
        )
        .await;

    println!("Dai returned: {}", ans);
}
