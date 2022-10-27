use dotenv::dotenv;
use ethers::{
    providers::{Http, Provider, Ws},
    types::{Address, Filter},
};
use std::{collections::HashMap, sync::Arc};

use tsuki::{
    constants::{
        protocol::UniswapV2,
        token::ERC20Token::{self, *},
    },
    uniswapV2::{UniswapV2Client, UniswapV2Pair},
    utils::matrix::Matrix3D,
};

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
    tokens_list.sort_by(|x, y| x.get_symbol().cmp(y.get_symbol())); // sort by name as that is order stored on blockchain

    let uniswapV2_client = UniswapV2Client::new(provider_ws); // initialize interfacer w/ blockchain

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

    // populate UniswapV2Pair matrix
    let mut matrix = Matrix3D::new(
        protocols_list.len(),
        tokens_list.len(),
        tokens_list.len(),
        UniswapV2Pair::default(),
    );

    let mut curr_idx = 0;
    for protocol in &protocols_list {
        for i in 0..tokens_list.len() {
            let token0 = tokens_list[i];
            for j in i + 1..tokens_list.len() {
                let token1 = tokens_list[j];
                let reserve0 = pair_reserves[curr_idx].0;
                let reserve1 = pair_reserves[curr_idx].1;
                matrix[(*protocol as usize, i, j)].update_metadata(*protocol, token0, token1);
                matrix[(*protocol as usize, i, j)].update_reserves(reserve0, reserve1);
                curr_idx += 1;
            }
        }
    }

    // populate reverse lookup table
    // TODO: why can't we combine this logic to the above loop?
    let mut pair_lookup: HashMap<Address, &UniswapV2Pair> = HashMap::new();

    let mut curr_idx = 0;
    for protocol in &protocols_list {
        for i in 0..tokens_list.len() {
            for j in i + 1..tokens_list.len() {
                let pair = &matrix[(*protocol as usize, i, j)];
                let address = &pair_addresses[curr_idx];
                pair_lookup.insert(*address, pair);
                curr_idx += 1;
            }
        }
    }

    // listen to pair sync events on blockchain
    let pair_sync_filter = Filter::new().event("Sync(uint112,uint112)");

    // provider_ws.subscribe_logs(filter)

    Ok(())
}
