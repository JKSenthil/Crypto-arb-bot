use dotenv::dotenv;
use ethers::{
    providers::{Http, Middleware, Provider, Ws},
    types::U256,
};
use futures_util::StreamExt;
use std::sync::Arc;

use tsuki::{
    constants::{
        protocol::UniswapV2::{self},
        token::ERC20Token::*,
    },
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
    let ws = WorldState::init(
        provider,
        Provider::<Ws>::connect(&rpc_node_ws_url).await?,
        provider_ws.clone(),
        tokens_list,
        uniswapV2_list,
    )
    .await;
    let ws = Arc::new(ws);

    tokio::spawn(ws.clone().listen_and_update_uniswapV2());

    let amount_in = U256::from(300);

    let routes = vec![
        vec![USDC, DAI, USDC],
        vec![USDC, USDT, USDC],
        vec![USDC, WETH, USDC],
        vec![USDC, WMATIC, USDC],
        vec![WMATIC, WETH, WMATIC],
    ];

    println!("DETECTING ARBITRAGE");

    let mut stream = provider_ws.subscribe_blocks().await?;
    while let Some(block) = stream.next().await {
        // when new block arrives, check arbitrage opportunity
        let mut futures = Vec::with_capacity(routes.len());
        for route in &routes {
            futures.push(tokio::spawn(ws.clone().compute_best_route(
                route.to_vec(),
                amount_in * U256::exp10(route[0].get_decimals() as usize),
            )))
        }
        for (i, future) in futures.into_iter().enumerate() {
            let result = future.await;
            match result {
                Ok((amount_out, protocol_route)) => {
                    let a = amount_in * U256::exp10(routes[i][0].get_decimals() as usize);
                    if amount_out > a {
                        println!(
                            "({i}), block_hash: {:?}, {:?}",
                            block.hash.unwrap(),
                            protocol_route.into_iter().map(|x| match x {
                                Protocol::UniswapV2(v) => v.get_name().to_string(),
                                Protocol::UniswapV3 { fee } => format!("UniswapV3 {fee}"),
                            }),
                        );
                        println!("Amount in: {a}, Amount Out: {amount_out}");
                    }
                }
                Err(_) => {}
            };
        }
    }

    Ok(())
}
