#![allow(dead_code)]

use std::sync::Arc;
use std::time::Instant;
use std::vec;

use aave::Aave;
use dotenv::dotenv;
use ethers::prelude::*;
use price::Price;

mod aave;
mod consts;
mod price;
mod utils;

use consts::ERC20Token::*;
use consts::Protocol::*;
use consts::Route;
use consts::ROUTES;
use utils::convert_to_U256;

async fn aave_example() {
    dotenv().ok();
    let polygon_rpc_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_URL").expect("Polygon RPC url expected in .env file");
    let provider = Provider::<Http>::try_from(polygon_rpc_url).unwrap();
    let aave = Aave::new(provider);

    let account = "0x53cbe6e60b4f8186e5307a86476b9c3fa4b0ba2b";

    let (_, _, _, _, _, health_factor) = aave.get_user_account_data(account).await;
    println!("Address {} has health factor: {}", account, health_factor);
}

async fn pull_latest_block_hash_v2() {
    dotenv().ok();
    let polygon_ws_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_WS_URL").expect("Polygon RPC url expected in .env file");

    let provider = Provider::<Ws>::connect(polygon_ws_url).await.unwrap();

    // let filter = Filter::new().select(FilterKind::PendingTransactions);

    let mut subscription_stream = provider.subscribe_blocks().await.unwrap();

    while let Some(block) = subscription_stream.next().await {
        println!("block hash: {:?},", block.number.unwrap());
    }
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

async fn route_example() {
    // let r = Route {
    //     path: vec![USDC, WETH, USDC],
    //     protocols: vec![SUSHISWAP, UNISWAP_V3],
    //     amount_in: convert_to_U256(100, USDC.get_token_decimal()),
    // };

    // let now = Instant::now();
    // let result = expected_out(price, r).await;
    // println!(
    //     "Expected USDC from route: {}, retrieval time: {}",
    //     result,
    //     now.elapsed().as_millis()
    // );
}

// TODO account for gas (based on network) and fees (if flashloaning from Aave)
fn is_profitable(start_amount: U256, end_amount: U256) -> bool {
    end_amount > start_amount
}

async fn expected_out<M: Middleware>(price: Arc<Price<M>>, route: &Route) -> U256 {
    let price = price.as_ref();
    let mut token_in = route.path[0];
    let mut current_amount = route.amount_in;

    let mut protocol;
    let mut token_out;
    for i in 1..route.path.len() {
        protocol = route.protocols[i - 1];
        token_out = route.path[i];

        current_amount = price
            .quote(protocol, token_in, token_out, current_amount)
            .await;
        token_in = token_out;
    }

    current_amount
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let polygon_ws_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_WS_URL").expect("Polygon RPC url expected in .env file");

    let provider = Provider::<Ws>::connect(polygon_ws_url).await.unwrap();
    let price = Arc::new(Price::new(provider));

    // let mut subscription_stream = provider.subscribe_blocks().await.unwrap();
    //while let Some(block) = subscription_stream.next().await {
    // }

    let now = Instant::now();
    for route in ROUTES.iter() {
        expected_out(price.clone(), route).await;
    }
    println!(
        "Time elapsed: {}ms for {} routes",
        now.elapsed().as_millis(),
        ROUTES.len()
    );

    let now = Instant::now();
    let mut futures = Vec::new();
    for route in ROUTES.iter() {
        let task = tokio::spawn(expected_out(price.clone(), route));
        futures.push(task);
    }

    for f in futures.into_iter() {
        let a = f.await.unwrap();
        println!("TEST {}", a);
    }
    println!(
        "Multithreading Time Elapsed: {}ms for {} routes",
        now.elapsed().as_millis(),
        ROUTES.len()
    );
}
