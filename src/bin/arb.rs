use clap::Parser;
use dotenv::dotenv;
use ethers::{
    prelude::{abigen, SignerMiddleware},
    providers::{Http, Middleware, Provider, PubsubClient, Ws},
    signers::{LocalWallet, Signer},
    types::{Address, U256},
};
use futures_util::StreamExt;
use log::{debug, error, info};
use std::{process, sync::Arc, time::Instant};

use tsuki::{
    constants::{
        protocol::{
            UniswapV2::{self},
            UNISWAP_V3,
        },
        token::ERC20Token::{self, *},
    },
    tx_pool::TxPool,
    utils::price_utils::amount_to_U256,
    world::{Protocol, WorldState},
};

abigen!(Flashloan, "abis/FlashloanV3.json");

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
fn is_profitable(token: ERC20Token, profit: U256, txn_fees: U256) -> bool {
    // normalize profit to 18 decimals for ease of comparison
    let profit = profit * U256::exp10((18 - token.get_decimals()).into());
    // assume 1 MATIC = $0.85
    let txn_fee_usd = txn_fees
        .checked_mul(U256::from(85))
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
                protocol_path.push(UNISWAP_V3.router_address);
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
    let global_provider = Provider::<Http>::try_from("https://polygon-rpc.com").unwrap();
    let global_provider = Arc::new(global_provider);

    let tokens_list = vec![USDC, USDT, DAI, WBTC, WMATIC, WETH];

    let txpool = TxPool::init(provider.clone(), 1000);
    let txpool = Arc::new(txpool);
    tokio::spawn(txpool.clone().stream_mempool());

    let ws = WorldState::init(
        provider.clone(),
        stream_provider,
        tokens_list,
        UniswapV2::get_all_protoccols(),
    )
    .await;

    let ws = Arc::new(ws);
    tokio::spawn(ws.clone().stream_data());

    let wallet = std::env::var("PRIVATE_KEY")
        .unwrap()
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(137u64);
    let client = SignerMiddleware::new(global_provider.clone(), wallet);
    let arbitrage_contract = Flashloan::new(
        "0x7472bacc648111408497c087826739e7a1e0a6d2"
            .parse::<Address>()
            .unwrap(),
        Arc::new(client),
    );

    info!("Setup complete. Detecting arbitrage opportunities...");
    let mut block_stream = provider.subscribe_blocks().await.unwrap();
    let mut txn_count = 0;
    while let Some(block) = block_stream.next().await {
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

                let params =
                    construct_arb_params(amount_in, &routes[i].token_path, &protocol_route);

                let est_gas_usage = U256::from(500000);
                let gas_price = txpool.get_90th_percentile_gas_price().await + U256::from(100);
                let txn_fees = gas_price.checked_mul(est_gas_usage).unwrap();
                if !is_profitable(token, profit, txn_fees) {
                    debug!(
                        "  Arb not profitable, fee: {:?}, profit: {:?}",
                        gas_price, profit
                    );
                    continue;
                }

                let current_block_number = block.number.unwrap();
                let target_block_number = U256::from(current_block_number.as_u64() + 1);
                let contract_call =
                    arbitrage_contract.execute_arbitrage(params, target_block_number);
                match contract_call.gas_price(gas_price).send().await {
                    Ok(pending_txn) => {
                        let _ = pending_txn.confirmations(1).await;
                        info!("  Txn submitted, curr block: {:?}", block.number.unwrap());
                    }
                    Err(_) => {
                        error!(
                            "  Err received in sending txn. Expected profit: {:?}, Route: {:?}){:?}",
                            profit,
                            i,
                            protocol_route
                                .into_iter()
                                .map(|x| match x {
                                    Protocol::UniswapV2(v) => v.get_name().to_string(),
                                    Protocol::UniswapV3 { fee } => format!("UniswapV3 {fee}"),
                                })
                                .collect::<Vec<String>>()
                        );
                        continue;
                    }
                }

                info!("  expected profit: {:?}, gas {:?}", profit, gas_price);
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
                txn_count += 1;
                if txn_count > 5 {
                    process::exit(1);
                }

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
            amount_in: U256::from(10000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(10000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(10000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(10000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        Route {
            amount_in: U256::from(5000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(5000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(5000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(5000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        Route {
            amount_in: U256::from(1000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(1000) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(1000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(1000) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WETH, USDC],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDC.get_decimals().into()),
            token_path: vec![USDC, WMATIC, USDC],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WETH, USDT],
        },
        Route {
            amount_in: U256::from(300) * U256::exp10(USDT.get_decimals().into()),
            token_path: vec![USDT, WMATIC, USDT],
        },
        // Route {
        //     amount_in: U256::from(300) * U256::exp10(DAI.get_decimals().into()),
        //     token_path: vec![DAI, WETH, DAI],
        // },
        // Route {
        //     amount_in: U256::from(300) * U256::exp10(DAI.get_decimals().into()),
        //     token_path: vec![DAI, WMATIC, DAI],
        // },
    ];

    let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    let alc_provider_ws = Arc::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);
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

        run_loop(
            alc_provider_ws.clone(),
            Provider::<Ws>::connect(&rpc_node_ws_url).await?,
            routes,
        )
        .await;
    }

    Ok(())
}
