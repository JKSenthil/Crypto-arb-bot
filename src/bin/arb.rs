use clap::Parser;
use dotenv::dotenv;
use ethers::{
    prelude::{abigen, SignerMiddleware},
    providers::{Middleware, Provider, PubsubClient, Ws},
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// use ipc (if running on node)
    #[arg(short, long)]
    use_ipc: bool,
}

#[inline(always)]
fn threshold(token: ERC20Token, amount_diff: U256) -> bool {
    match token {
        USDC => amount_diff >= U256::from(10000),
        USDT => amount_diff >= U256::from(10000),
        DAI => amount_diff >= U256::from(1) * U256::exp10((DAI.get_decimals() - 2) as usize),
        // WMATIC => amount_diff >= 0.02,
        // WETH => amount_diff >= 0.00005,
        _ => false,
    }
}

fn construct_arb_params(
    amount_in: U256,
    token_path: &Vec<ERC20Token>,
    protocol_route: &Vec<Protocol>,
) -> ArbParams {
    let token_path = token_path.iter().map(|x| x.get_address()).collect();
    let mut protocol_path = Vec::with_capacity(protocol_route.len());
    let mut protocol_types = Vec::with_capacity(protocol_route.len());
    let mut fees = Vec::with_capacity(protocol_route.len());
    for protocol in protocol_route {
        match protocol {
            Protocol::UniswapV2(p) => {
                protocol_path.push(p.get_router_address());
                protocol_types.push(0);
                fees.push(0);
            }
            Protocol::UniswapV3 { fee } => {
                protocol_path.push(
                    "0xE592427A0AEce92De3Edee1F18E0157C05861564"
                        .parse::<Address>()
                        .unwrap(),
                );
                protocol_types.push(1);
                fees.push(*fee);
            }
        };
    }

    ArbParams {
        amount_in: amount_in,
        token_path: token_path,
        protocol_path: protocol_path,
        protocol_types: protocol_types,
        fees: fees,
    }
}

async fn run_loop<P: PubsubClient + Clone + 'static>(
    provider: Arc<Provider<P>>,
    stream_provider: Provider<P>,
    routes: Vec<Vec<ERC20Token>>,
) {
    let tokens_list = vec![USDC, USDT, DAI, WBTC, WMATIC, WETH];
    let ws = WorldState::init(
        provider.clone(),
        stream_provider,
        tokens_list,
        UniswapV2::get_all_protoccols(),
    )
    .await;

    let ws = Arc::new(ws);
    tokio::spawn(ws.clone().listen_and_update_uniswapV2());

    let amount_in = U256::from(30);

    let wallet = std::env::var("PRIVATE_KEY")
        .unwrap()
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(137u64);
    let client = SignerMiddleware::new(provider.clone(), wallet);
    let arbitrage_contract = Flashloan::new(
        "0x7586b61cd07d3f7b1e701d0ab719f9feea4674af"
            .parse::<Address>()
            .unwrap(),
        Arc::new(client),
    );

    println!("DETECTING ARBITRAGE");
    let mut stream = provider.subscribe_blocks().await.unwrap();
    while let Some(block) = stream.next().await {
        let gas_price_future = provider.get_gas_price();
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

        let mut gas_price = gas_price_future.await.unwrap();
        // println!("gas price {:?}", gas_price);

        // when new block arrives, check arbitrage opportunity
        // let now = Instant::now();
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
                Ok((est_amount_out, protocol_route)) => {
                    // println!("{}) time elasped: {:?}ms", i, now.elapsed().as_millis());
                    let amount_in = amount_in * U256::exp10(routes[i][0].get_decimals() as usize);
                    if est_amount_out > amount_in {
                        let profit = est_amount_out - amount_in;
                        // let profit =
                        //     (profit.as_u128() as f64) / (routes[i][0].get_decimals() as f64);
                        if threshold(routes[i][0], profit) {
                            println!(
                                "Sending txn..., expected profit: {:?}, est_amount_out: {:?}",
                                profit, est_amount_out
                            );

                            let params =
                                construct_arb_params(amount_in, &routes[i], &protocol_route);

                            // 20% markup on gas price
                            gas_price = gas_price.checked_mul(U256::from(120)).unwrap();
                            gas_price = gas_price.checked_div(U256::from(100)).unwrap();
                            match arbitrage_contract
                                .execute_arbitrage(params)
                                .gas_price(gas_price)
                                .send()
                                .await
                            {
                                Ok(pending_txn) => {
                                    println!("  Txn submitted: {:?}", pending_txn.tx_hash());
                                }
                                Err(_) => println!("  Err received"),
                            }

                            println!(
                                "({i}), {:?}",
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args = Args::parse();

    let routes = vec![
        vec![USDC, WETH, USDC],
        vec![USDC, WMATIC, USDC],
        vec![USDT, WETH, USDT],
        vec![USDT, WMATIC, USDT],
        // vec![USDC, USDT, USDC],
        // vec![USDC, DAI, USDC],
        // vec![USDT, USDC, USDT],
        // vec![USDT, DAI, USDT],
        // vec![DAI, USDC, DAI],
        // vec![DAI, USDT, DAI],
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

    if args.use_ipc {
        println!("USING IPC!");
        let provider_ipc = Provider::connect_ipc("/mountdrive/.bor/data/bor.ipc").await?;
        let provider_ipc = Arc::new(provider_ipc);
        run_loop(
            provider_ipc,
            Provider::connect_ipc("/mountdrive/.bor/data/bor.ipc").await?,
            routes,
        )
        .await;
    } else {
        println!("USING ALCHEMY WS!");
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
        let provider_ws = Arc::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);
        run_loop(
            provider_ws,
            Provider::<Ws>::connect(&rpc_node_ws_url).await?,
            routes,
        )
        .await;
    }

    Ok(())
}
