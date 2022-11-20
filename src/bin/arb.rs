use clap::Parser;
use dotenv::dotenv;
use ethers::{
    prelude::{abigen, SignerMiddleware},
    providers::{Middleware, Provider, PubsubClient, Ws},
    signers::{LocalWallet, Signer},
    types::{Address, U256},
};
use futures_util::StreamExt;
use log::{debug, error, info};
use std::{sync::Arc, time::Instant};

use tsuki::{
    constants::{
        protocol::UniswapV2::{self},
        token::ERC20Token::{self, *},
    },
    utils::price_utils::amount_to_U256,
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

struct Route {
    amount_in: U256,
    token_path: Vec<ERC20Token>,
}

#[inline(always)]
fn threshold(token: ERC20Token, amount_diff: U256) -> bool {
    match token {
        USDC => amount_diff >= U256::from(10000),
        USDT => amount_diff >= U256::from(10000),
        DAI => amount_diff >= amount_to_U256(0.01, 2, DAI),
        WMATIC => amount_diff >= amount_to_U256(0.01, 2, WMATIC),
        WETH => amount_diff >= amount_to_U256(0.00005, 5, WETH),
        _ => false,
    }
}

#[inline(always)]
fn is_profitable(token: ERC20Token, profit: U256, txn_fees: U256) -> bool {
    // normalize profit to 18 decimals for ease of comparison
    let profit = profit * U256::exp10((18 - token.get_decimals()).into());
    // assume 1 MATIC = $0.90
    let txn_fee_usd = txn_fees
        .checked_mul(U256::from(90))
        .unwrap()
        .checked_div(U256::from(100))
        .unwrap();
    profit > txn_fee_usd
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
    routes: Vec<Route>,
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

    info!("Setup complete. Detected arbitrage opportunities...");
    let mut stream = provider.subscribe_blocks().await.unwrap();
    while let Some(block) = stream.next().await {
        // ensure latest block
        match provider.get_block_number().await {
            Ok(num) => {
                if num != block.number.unwrap() {
                    info!("skipping to latest block");
                    continue;
                }
            }
            Err(e) => {
                error!("error {:?} in retrieving block number, skipping...", e);
                continue;
            }
        };

        let now = Instant::now();
        let mut futures = Vec::with_capacity(routes.len());
        for route in &routes {
            // calc arb opportunity on each route
            futures.push(tokio::spawn(
                ws.clone()
                    .compute_best_route(route.token_path.to_vec(), route.amount_in),
            ))
        }

        for (i, future) in futures.into_iter().enumerate() {
            let token = routes[i].token_path[0];
            let (est_amount_out, protocol_route) = future.await.unwrap_or_default();
            let amount_in = routes[i].amount_in;
            if est_amount_out > amount_in {
                let profit = est_amount_out - amount_in;
                // ensure profit minimum threshold is met
                if !threshold(token, profit) {
                    continue;
                }

                let params =
                    construct_arb_params(amount_in, &routes[i].token_path, &protocol_route);

                let est_gas_usage: U256;
                let contract_call = arbitrage_contract.execute_arbitrage(params);
                match contract_call.estimate_gas().await {
                    Ok(usage) => est_gas_usage = usage,
                    Err(_) => {
                        error!("  Err received in estimating gas");
                        continue;
                    }
                };

                // 30% markup on gas price
                let mut gas_price = provider.get_gas_price().await.unwrap();
                gas_price = gas_price.checked_mul(U256::from(130)).unwrap();
                gas_price = gas_price.checked_div(U256::from(100)).unwrap();

                let txn_fees = gas_price * est_gas_usage;

                if !is_profitable(token, profit, txn_fees) {
                    debug!("  Arb not profitable");
                    continue;
                }

                match contract_call.gas_price(gas_price).send().await {
                    Ok(pending_txn) => {
                        let _ = pending_txn.confirmations(1).await;
                        info!("  Txn submitted");
                    }
                    Err(_) => {
                        error!("  Err received in sending txn");
                        continue;
                    }
                }

                info!("  expected profit: {:?}", profit);
                info!(
                    "  ({i}), {:?}",
                    protocol_route
                        .into_iter()
                        .map(|x| match x {
                            Protocol::UniswapV2(v) => v.get_name().to_string(),
                            Protocol::UniswapV3 { fee } => format!("UniswapV3 {fee}"),
                        })
                        .collect::<Vec<String>>(),
                );

                break;
            }
        }
        debug!("Time elasped: {:?}ms", now.elapsed().as_millis());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    let routes = vec![
        Route {
            amount_in: U256::from(30) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(3000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(30) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(3000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(30) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(3000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(30) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        Route {
            amount_in: U256::from(3000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(DAI.get_decimals().into()),
            token_path: vec![DAI, WETH, DAI],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(DAI.get_decimals().into()),
            token_path: vec![DAI, WMATIC, DAI],
        },
    ];

    if args.use_ipc {
        info!("Using IPC");
        let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
        let provider_ipc = Arc::new(provider_ipc);
        run_loop(
            provider_ipc,
            Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?,
            routes,
        )
        .await;
    } else {
        info!("Using Alchemy");
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
