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
};

#[derive(Debug, Clone, Copy)]
enum Protocol {
    UniswapV2(UniswapV2),
    UniswapV3 { fee: u32 },
}

#[inline(always)]
fn order_tokens(token0: ERC20Token, token1: ERC20Token) -> (ERC20Token, ERC20Token, bool) {
    match token0.get_address().cmp(&token1.get_address()) {
        Ordering::Less => (token0, token1, true),
        _ => (token1, token0, false),
    }
}

fn best_uniswapV2(
    uniswapV2_markets: &Matrix3D<UniswapV2Pair>,
    token_in: ERC20Token,
    token_out: ERC20Token,
    amount_in: U256,
) -> (U256, UniswapV2) {
    let (token0, token1, is_same_order) = order_tokens(token_in, token_out);

    let mut best_protocol = UNISWAPV2_PROTOCOLS[0];
    let mut best_amount_out = uniswapV2_markets
        [(best_protocol as usize, token0 as usize, token1 as usize)]
        .get_amounts_out(amount_in, is_same_order);
    println!(
        "is_same_order {}, token_in {}, token_out {}",
        is_same_order,
        token_in.get_symbol(),
        token_out.get_symbol()
    );

    for i in 1..UNISWAPV2_PROTOCOLS.len() {
        let protocol = UNISWAPV2_PROTOCOLS[i];
        let amount_out = uniswapV2_markets[(protocol as usize, token0 as usize, token1 as usize)]
            .get_amounts_out(amount_in, is_same_order);
        println!(
            "   Protocol {}, amount_out {}",
            protocol.get_name(),
            amount_out
        );

        if amount_out > best_amount_out {
            best_protocol = protocol;
            best_amount_out = amount_out;
        }
    }

    (best_amount_out, best_protocol)
}

async fn best_uniswapV3<M: Middleware + Clone>(
    uniswapV3_client: &UniswapV3Client<M>,
    token_in: ERC20Token,
    token_out: ERC20Token,
    amount_in: U256,
) -> (U256, u32) {
    let return_data = uniswapV3_client
        .quote_multicall(token_in, token_out, amount_in)
        .await;

    (return_data.1, return_data.0)
}

// TODO: is this theoretically sound? (always wanting to pick max value at each step)
async fn compute_best_route<M: Middleware + Clone>(
    uniswapV2_markets: &Matrix3D<UniswapV2Pair>,
    uniswapV3_client: &UniswapV3Client<M>,
    token_path: Vec<ERC20Token>,
    amount_in: U256,
) -> (U256, Vec<Protocol>) {
    let mut protocols: Vec<Protocol> = Vec::with_capacity(token_path.len() - 1);

    let mut token_in = token_path[0];
    let mut token_out;
    let mut current_amt = amount_in;
    for i in 1..token_path.len() {
        token_out = token_path[i];
        let (best_amount_out_v3, best_pool_fee) =
            best_uniswapV3(uniswapV3_client, token_in, token_out, current_amt).await;

        let (best_amount_out, uniswapV2_protocol) =
            best_uniswapV2(uniswapV2_markets, token_in, token_out, current_amt);

        if best_amount_out > best_amount_out_v3 {
            current_amt = best_amount_out;
            protocols.push(Protocol::UniswapV2(uniswapV2_protocol));
        } else {
            current_amt = best_amount_out_v3;
            protocols.push(Protocol::UniswapV3 { fee: best_pool_fee });
        }
        token_in = token_out;
    }
    (current_amt, protocols)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load providers
    dotenv().ok();
    let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL")?;
    let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    let provider = Provider::<Http>::try_from(&rpc_node_url).unwrap();
    let provider_ws = Arc::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);

    // define tokens and protocols list
    let mut tokens_list = vec![USDC, USDT, DAI, WBTC, WMATIC, WETH]; // TODO standardize
    let protocols_list = UniswapV2::get_all_protoccols();

    // grab all pair addresses
    tokens_list.sort_by(|x, y| x.get_address().cmp(&y.get_address())); // sort by name as that is order stored on blockchain

    let uniswapV2_client = UniswapV2Client::new(provider_ws.clone()); // initialize interfacer w/ blockchain

    let mut pair_address_multicall_input: Vec<(UniswapV2, ERC20Token, ERC20Token)> = Vec::new();
    for protocol in &protocols_list {
        for i in 0..tokens_list.len() {
            let token0 = tokens_list[i];
            for j in i + 1..tokens_list.len() {
                let token1 = tokens_list[j];
                pair_address_multicall_input.push((*protocol, token0, token1));
            }
        }
    }

    let pair_addresses = uniswapV2_client
        .get_pair_address_multicall(provider.clone(), pair_address_multicall_input)
        .await;

    let pair_reserves = uniswapV2_client
        .get_pair_reserves_multicall(provider.clone(), &pair_addresses)
        .await;

    // populate UniswapV2Pair matrix and reverse lookup table
    let mut matrix = Matrix3D::new(
        protocols_list.len(),
        tokens_list.len(),
        tokens_list.len(),
        UniswapV2Pair::default(),
    );

    let mut pair_lookup: HashMap<Address, (UniswapV2, ERC20Token, ERC20Token)> = HashMap::new();

    let mut curr_idx = 0;
    for protocol in &protocols_list {
        for i in 0..tokens_list.len() {
            let token0 = tokens_list[i];
            for j in (i + 1)..tokens_list.len() {
                let token1 = tokens_list[j];
                let reserve0 = pair_reserves[curr_idx].0;
                let reserve1 = pair_reserves[curr_idx].1;
                matrix[(*protocol as usize, token0 as usize, token1 as usize)]
                    .update_metadata(*protocol, token0, token1);
                matrix[(*protocol as usize, token0 as usize, token1 as usize)]
                    .update_reserves(reserve0, reserve1);
                pair_lookup.insert(pair_addresses[curr_idx], (*protocol, token0, token1));
                curr_idx += 1;
            }
        }
    }

    let uniswapV3_client = UniswapV3Client::new(Provider::<Ws>::connect(&rpc_node_ws_url).await?);
    let token_path = vec![WETH, USDT, WETH];
    let amount_in = U256::from(30_000000);
    let now = Instant::now();
    let (amount_out, protocol_route) =
        compute_best_route(&matrix, &uniswapV3_client, token_path, amount_in).await;
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
