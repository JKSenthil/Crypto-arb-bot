use dotenv::dotenv;
use ethers::{
    prelude::{abigen, SignerMiddleware},
    providers::{Middleware, Provider, Ws},
    signers::{LocalWallet, Signer},
    types::{Address, U256},
};
use futures_util::StreamExt;
use std::{sync::Arc, time::Instant};

use tsuki::{
    constants::{
        protocol::UniswapV2::{self},
        token::ERC20Token::{self, *},
    },
    world::{Protocol, WorldState},
};

abigen!(Flashloan, "abis/Flashloan.json");

#[inline(always)]
fn threshold(token: ERC20Token, amount_diff: f64) -> bool {
    match token {
        USDC => amount_diff >= 0.02,
        USDT => amount_diff >= 0.02,
        DAI => amount_diff >= 0.02,
        WMATIC => amount_diff >= 0.02,
        WETH => amount_diff >= 0.00005,
        _ => false,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load providers
    dotenv().ok();
    // let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    // let provider = Arc::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);
    let provider = Provider::connect_ipc("/mountdrive/.bor/data/bor.ipc").await?;
    let provider = Arc::new(provider);

    let tokens_list = vec![USDC, USDT, DAI, WBTC, WMATIC, WETH];
    let uniswapV2_list = UniswapV2::get_all_protoccols();
    let ws = WorldState::init(
        provider.clone(),
        Provider::connect_ipc("/mountdrive/.bor/data/bor.ipc").await?,
        tokens_list,
        uniswapV2_list,
    )
    .await;
    let ws = Arc::new(ws);
    tokio::spawn(ws.clone().listen_and_update_uniswapV2());

    let amount_in = U256::from(3000);

    let routes = vec![
        vec![USDC, WETH, USDC],
        vec![USDC, WMATIC, USDC],
        vec![USDT, WETH, USDT],
        vec![USDT, WMATIC, USDT],
        vec![DAI, WETH, DAI],
        vec![DAI, WMATIC, DAI],
        // vec![WMATIC, USDC, WMATIC],
        // vec![WMATIC, DAI, WMATIC],
        // vec![WMATIC, USDT, WMATIC],
        // vec![WMATIC, WETH, WMATIC],
        // vec![WETH, USDC, WETH],
        // vec![WETH, DAI, WETH],
        // vec![WETH, USDT, WETH],
        // vec![WETH, WMATIC, WETH],
    ];

    let wallet = std::env::var("PRIVATE_KEY")?
        .parse::<LocalWallet>()?
        .with_chain_id(137u64);
    let client = SignerMiddleware::new(provider.clone(), wallet);
    let client = Arc::new(client);
    let arbitrage_contract = Flashloan::new(
        "0x52415ffd6d6f604224fe0FbBA2395fFBa10C1F7D"
            .parse::<Address>()
            .unwrap(),
        client,
    );

    println!("DETECTING ARBITRAGE");

    let mut stream = provider.subscribe_blocks().await?;
    while let Some(block) = stream.next().await {
        let block_number = provider.get_block_number().await;
        match block_number {
            Ok(num) => {
                if num != block.number.unwrap() {
                    println!("skipping to latest block");
                    continue;
                }
            }
            Err(e) => {
                println!("error {:?} in retrieving block number, skipping...", e);
                continue;
            }
        };

        // when new block arrives, check arbitrage opportunity
        let now = Instant::now();
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
                    println!("{}) time elasped: {:?}ms", i, now.elapsed().as_millis());
                    let a = amount_in * U256::exp10(routes[i][0].get_decimals() as usize);
                    if amount_out > a {
                        let profit = amount_out - a;
                        let profit = profit.as_u128() as f64;
                        if threshold(routes[i][0], profit) {
                            println!(
                                "Sending txn..., expected profit: {:?}, amount_out: {:?}",
                                profit, amount_out
                            );

                            // send transaction order
                            let tp = routes[i]
                                .clone()
                                .into_iter()
                                .map(|x| x.get_address())
                                .collect();
                            let mut pp = Vec::with_capacity(protocol_route.len());
                            let mut pt = Vec::with_capacity(protocol_route.len());
                            let mut fees = Vec::with_capacity(protocol_route.len());
                            for protocol in &protocol_route {
                                match protocol {
                                    Protocol::UniswapV2(p) => {
                                        pp.push(p.get_router_address());
                                        pt.push(0);
                                        fees.push(0);
                                    }
                                    Protocol::UniswapV3 { fee } => {
                                        pp.push(
                                            "0xE592427A0AEce92De3Edee1F18E0157C05861564"
                                                .parse::<Address>()
                                                .unwrap(),
                                        );
                                        pt.push(1);
                                        fees.push(*fee);
                                    }
                                };
                            }
                            let params = ArbParams {
                                amount_in: a,
                                token_path: tp,
                                protocol_path: pp,
                                protocol_types: pt,
                                fees: fees,
                            };
                            // 20% markup on gas
                            let mut val = provider.clone().get_gas_price().await.unwrap();
                            val = val.checked_mul(U256::from(120)).unwrap();
                            val = val.checked_div(U256::from(100)).unwrap();
                            match arbitrage_contract
                                .execute_arbitrage(params)
                                .gas_price(val)
                                .send()
                                .await
                            {
                                Ok(pending_txn) => {
                                    println!("  Txn submitted: {}", pending_txn.tx_hash());
                                }
                                Err(e) => println!("    Err received: {}", e),
                            }

                            println!(
                                "({i}), block_hash: {:?}, {:?}",
                                block.hash.unwrap(),
                                protocol_route.into_iter().map(|x| match x {
                                    Protocol::UniswapV2(v) => v.get_name().to_string(),
                                    Protocol::UniswapV3 { fee } => format!("UniswapV3 {fee}"),
                                }),
                            );

                            break;
                        }
                    }
                }
                Err(_) => {}
            };
        }
    }

    Ok(())
}
