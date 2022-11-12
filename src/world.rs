use ethers::{
    abi::{parse_abi, Address},
    prelude::BaseContract,
    providers::{Http, Middleware, Provider, PubsubClient, SubscriptionStream, Ws},
    types::{Log, U256},
};
use futures_util::StreamExt;
use std::{cmp::Ordering, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    constants::{
        protocol::{UniswapV2, UNISWAPV2_PROTOCOLS},
        token::ERC20Token,
    },
    event_monitor::get_pair_sync_stream,
    uniswapV2::{UniswapV2Client, UniswapV2Pair},
    uniswapV3::UniswapV3Client,
    utils::matrix::Matrix3D,
};

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
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

pub struct WorldState<M, P> {
    provider: Arc<M>,
    stream_provider: Provider<P>,
    uniswapV2_markets: RwLock<Matrix3D<UniswapV2Pair>>,
    uniswapV2_pair_lookup: HashMap<Address, (UniswapV2, ERC20Token, ERC20Token)>,
    pub uniswapV2_pair_addresses: Vec<Address>,
    uniswapV3_client: UniswapV3Client<M>,
}

impl<M: Middleware + Clone, P: PubsubClient> WorldState<M, P> {
    pub async fn init(
        provider: Arc<M>,
        stream_provider: Provider<P>,
        mut tokens_list: Vec<ERC20Token>,
        uniswapV2_list: Vec<UniswapV2>,
    ) -> Self {
        // initialize uniswap v2 client to get initial data
        let uniswapV2_client = UniswapV2Client::new(provider.clone()); // initialize interfacer w/ blockchain

        // sort tokens by pair addresses
        tokens_list.sort_by(|x, y| x.get_address().cmp(&y.get_address()));

        // grab all pair addresses across all pairs, protocols
        let mut pair_address_multicall_input: Vec<(UniswapV2, ERC20Token, ERC20Token)> = Vec::new();
        for protocol in &uniswapV2_list {
            for i in 0..tokens_list.len() {
                let token0 = tokens_list[i];
                for j in i + 1..tokens_list.len() {
                    let token1 = tokens_list[j];
                    pair_address_multicall_input.push((*protocol, token0, token1));
                }
            }
        }

        let pair_addresses = uniswapV2_client
            .get_pair_address_multicall(pair_address_multicall_input)
            .await;

        // grab all reserves for pair addresses
        let pair_reserves = uniswapV2_client
            .get_pair_reserves_multicall(&pair_addresses)
            .await;

        // populate UniswapV2Pair matrix and reverse lookup table
        let mut matrix = Matrix3D::new(
            uniswapV2_list.len(),
            tokens_list.len(),
            tokens_list.len(),
            UniswapV2Pair::default(),
        );

        // create pair addresses to information mapping
        let mut pair_lookup: HashMap<Address, (UniswapV2, ERC20Token, ERC20Token)> = HashMap::new();

        let mut curr_idx = 0;
        for protocol in &uniswapV2_list {
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

        WorldState {
            provider: provider.clone(),
            stream_provider: stream_provider,
            uniswapV2_markets: RwLock::new(matrix),
            uniswapV2_pair_lookup: pair_lookup,
            uniswapV2_pair_addresses: pair_addresses,
            uniswapV3_client: UniswapV3Client::new(provider.clone()),
        }
    }

    pub async fn listen_and_update_uniswapV2(self: Arc<Self>) {
        // get sync stream
        let mut stream = get_pair_sync_stream(
            &self.stream_provider,
            self.uniswapV2_pair_addresses.to_vec(),
        )
        .await;
        let pair_sync_abi = BaseContract::from(
            parse_abi(&["event Sync(uint112 reserve0, uint112 reserve1)"]).unwrap(),
        );

        while let Some(log) = stream.next().await {
            let (reserve0, reserve1): (U256, U256) = pair_sync_abi
                .decode_event("Sync", log.topics, log.data)
                .unwrap();
            let (protocol, token0, token1) = self.uniswapV2_pair_lookup[&log.address];
            self.uniswapV2_markets.write().await
                [(protocol as usize, token0 as usize, token1 as usize)]
                .update_reserves(reserve0, reserve1);
            println!(
                "Transaction Hash: {:?} --- Block#:{}, Pair reserves updated on {:?} protocol, pair {}-{}",
                log.transaction_hash.unwrap(),
                log.block_number.unwrap(),
                protocol.get_name(),
                token0.get_symbol(),
                token1.get_symbol()
            );
        }
    }

    pub async fn compute_best_route(
        self: Arc<Self>,
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
                self.best_uniswapV3(token_in, token_out, current_amt).await;

            let (best_amount_out, uniswapV2_protocol) =
                self.best_uniswapV2(token_in, token_out, current_amt).await;

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

    async fn best_uniswapV2(
        &self,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> (U256, UniswapV2) {
        let (token0, token1, is_same_order) = order_tokens(token_in, token_out);

        let mut best_protocol = UNISWAPV2_PROTOCOLS[0];
        let mut best_amount_out = self.uniswapV2_markets.read().await
            [(best_protocol as usize, token0 as usize, token1 as usize)]
            .get_amounts_out(amount_in, is_same_order);

        for i in 1..UNISWAPV2_PROTOCOLS.len() {
            let protocol = UNISWAPV2_PROTOCOLS[i];
            let amount_out = self.uniswapV2_markets.read().await
                [(protocol as usize, token0 as usize, token1 as usize)]
                .get_amounts_out(amount_in, is_same_order);

            if amount_out > best_amount_out {
                best_protocol = protocol;
                best_amount_out = amount_out;
            }
        }

        (best_amount_out, best_protocol)
    }

    async fn best_uniswapV3(
        &self,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> (U256, u32) {
        let return_data = self
            .uniswapV3_client
            .quote_multicall(token_in, token_out, amount_in)
            .await;

        (return_data.1, return_data.0)
    }
}
