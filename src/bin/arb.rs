use dotenv::dotenv;
use ethers::{
    abi::parse_abi,
    prelude::BaseContract,
    providers::{Http, Middleware, Provider, Ws},
    types::{Address, U256},
};
use futures_util::StreamExt;
use std::{cmp::Ordering, collections::HashMap, sync::Arc, time::Instant};

use tsuki::{
    constants::{
        protocol::{
            UniswapV2::{self},
            UNISWAPV2_PROTOCOLS,
        },
        token::ERC20Token::{self, *},
    },
    event_monitor::get_pair_sync_stream,
    uniswapV2::{UniswapV2Client, UniswapV2Pair},
    uniswapV3::UniswapV3Client,
    utils::matrix::Matrix3D,
    world::{Protocol, WorldState},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load providers
    dotenv().ok();
    let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL")?;
    let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    let provider = Provider::<Http>::try_from(&rpc_node_url).unwrap();
    let provider_ws = Arc::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);

    let tokens_list = vec![USDC, USDT, DAI, WBTC, WMATIC, WETH];
    let uniswapV2_list = UniswapV2::get_all_protoccols();
    let ws = WorldState::init(provider, provider_ws, tokens_list, uniswapV2_list).await;
    // let ws = Arc::new(ws);

    let mut futures = Vec::new();
    let task1 = tokio::spawn(async move {
        ws.compute_best_route(vec![USDC, DAI, USDC], U256::from(30_000000))
            .await
    });
    futures.push(task1);
    let task2 = tokio::spawn(async move {
        ws.compute_best_route(vec![USDC, WMATIC, USDC], U256::from(30_000000))
            .await
    });

    let token_path = vec![WETH, USDT, WETH];
    let amount_in = U256::from(30) * U256::exp10(WETH.get_decimals().into());
    let now = Instant::now();
    let (amount_out, protocol_route) = ws.compute_best_route(token_path, amount_in).await;
    println!("TIME ELAPSED: {}ms", now.elapsed().as_millis());
    println!(
        "{:?}",
        protocol_route.into_iter().map(|x| match x {
            Protocol::UniswapV2(v) => v.get_name().to_string(),
            Protocol::UniswapV3 { fee } => format!("UniswapV3 {fee}"),
        })
    );
    println!("Amount in: {amount_in}, Amount Out: {amount_out}");

    // listen to pair sync events on blockchain
    // let mut stream = get_pair_sync_stream(&provider_ws, pair_addresses).await;
    // let pair_sync_abi =
    //     BaseContract::from(parse_abi(&["event Sync(uint112 reserve0, uint112 reserve1)"]).unwrap());

    // while let Some(log) = stream.next().await {
    //     let (reserve0, reserve1): (U256, U256) = pair_sync_abi
    //         .decode_event("Sync", log.topics, log.data)
    //         .unwrap();
    //     let (protocol, token0, token1) = pair_lookup[&log.address];
    //     matrix[(protocol as usize, token0 as usize, token1 as usize)]
    //         .update_reserves(reserve0, reserve1);
    //     println!(
    //         "Transaction Hash: {:?} --- Block#:{}, Pair reserves updated on {:?} protocol, pair {}-{}",
    //         log.transaction_hash.unwrap(),
    //         log.block_number.unwrap(),
    //         protocol.get_name(),
    //         token0.get_symbol(),
    //         token1.get_symbol()
    //     );
    // }

    Ok(())
}

// TIME ELAPSED: 557ms
// Map { iter: Iter([UniswapV3 { fee: 500 }, UniswapV3 { fee: 3000 }, UniswapV2(QUICKSWAP)]) }
// Amount in: 30000000, Amount Out: 3299990550688903951382293
