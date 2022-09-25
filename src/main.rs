#![allow(dead_code)]

use dotenv::dotenv;
use ethers::prelude::*;
use price::Price;

mod consts;
mod price;
mod utils;

use consts::ERC20Token::*;
use consts::Protocol::*;
use consts::Route;

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

// TODO account for gas (based on network) and fees (if flashloaning from Aave)
fn is_profitable(start_amount: U256, end_amount: U256) -> bool {
    end_amount > start_amount
}

async fn expected_out<M: Middleware>(price: Price<M>, route: Route) -> U256 {
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
    let polygon_rpc_url =
        dotenv::var("ALCHEMY_POLYGON_RPC_URL").expect("Polygon RPC url expected in .env file");
    let provider = Provider::<Http>::try_from(polygon_rpc_url).unwrap();
    let price = Price::new(provider);

    let ans = price.quote(JETSWAP, USDC, WETH, U256::from(1000000)).await;

    println!("WETH returned: {}", ans);

    // let ans = price
    //     .quote_route(UNISWAP_V3, vec![USDC, DAI, USDC], U256::from(1000000))
    //     .await;

    // println!("USDC returned: {}", ans);
}
