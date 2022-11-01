use std::{
    ops::{Add, Mul},
    sync::Arc,
};

use ethers::{
    abi::Token::{self, *},
    contract::Contract,
    core::abi::Abi,
    prelude::{abigen, Multicall},
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};

use crate::constants::{protocol::UniswapV2, token::ERC20Token};

abigen!(
    IUniswapV2Router02,
    "abis/uniswap/v2/IUniswapV2Router02.json"
);
abigen!(IUniswapV2Factory, "abis/uniswap/v2/IUniswapV2Factory.json");
abigen!(IUniswapV2Pair, "abis/uniswap/v2/IUniswapV2Pair.json");

#[derive(Debug, Clone, Copy)]
pub struct UniswapV2Pair {
    protocol: UniswapV2,
    token0: ERC20Token,
    token1: ERC20Token,
    reserve0: U256,
    reserve1: U256,
    fee: Option<u32>,
}

// TODO implement correct MeshSwap implementation?
impl UniswapV2Pair {
    pub fn default() -> Self {
        Self {
            protocol: UniswapV2::SUSHISWAP,
            token0: ERC20Token::USDC,
            token1: ERC20Token::USDC,
            reserve0: U256::zero(),
            reserve1: U256::zero(),
            fee: None,
        }
    }

    pub fn update_metadata(&mut self, protocol: UniswapV2, token0: ERC20Token, token1: ERC20Token) {
        self.protocol = protocol;
        self.token0 = token0;
        self.token1 = token1;
    }

    pub fn update_reserves(&mut self, reserve0: U256, reserve1: U256) {
        self.reserve0 = reserve0;
        self.reserve1 = reserve1;
    }

    // TODO clean up repetition code later
    fn get_amount_out(self, amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
        let amount = match self.protocol {
            // UniswapV2::MESHSWAP => {
            //     let amount_in_with_fee: U256 = amount_in.mul(9990);
            //     let numerator: U256 = amount_in_with_fee.mul(reserve_out);
            //     let denominator: U256 = reserve_in.mul(10000_u32).add(amount_in_with_fee);
            //     numerator / denominator
            // }
            UniswapV2::POLYCAT => {
                let amount_in_with_fee: U256 = amount_in.mul(9976);
                let numerator: U256 = amount_in_with_fee.mul(reserve_out);
                let denominator: U256 = reserve_in.mul(10000_u32).add(amount_in_with_fee);
                numerator / denominator
            }
            UniswapV2::APESWAP => {
                let amount_in_with_fee: U256 = amount_in.mul(998);
                let numerator: U256 = amount_in_with_fee.mul(reserve_out);
                let denominator: U256 = reserve_in.mul(1000_u32).add(amount_in_with_fee);
                numerator / denominator
            }
            _ => {
                let amount_in_with_fee: U256 = amount_in.mul(997);
                let numerator: U256 = amount_in_with_fee.mul(reserve_out);
                let denominator: U256 = reserve_in.mul(1000_u32).add(amount_in_with_fee);
                numerator / denominator
            }
        };
        amount
    }

    pub fn get_amounts_out(&self, amount_in: U256, token0: bool) -> U256 {
        if token0 {
            return self.get_amount_out(amount_in, self.reserve0, self.reserve1);
        }
        return self.get_amount_out(amount_in, self.reserve1, self.reserve0);
    }
}

pub struct UniswapV2Client<M> {
    provider: Arc<M>,
    router_mapping: Vec<IUniswapV2Router02<M>>,
    factory_mapping: Vec<IUniswapV2Factory<M>>,
}

impl<M: Middleware> UniswapV2Client<M> {
    pub fn new(provider: Arc<M>) -> Self {
        let protocols_list = UniswapV2::get_all_protoccols();

        let mut router_list: Vec<IUniswapV2Router02<M>> = Vec::new();
        let mut factory_list: Vec<IUniswapV2Factory<M>> = Vec::new();

        for protocol in protocols_list {
            router_list.push(IUniswapV2Router02::new(
                protocol.get_router_address(),
                provider.clone(),
            ));
            factory_list.push(IUniswapV2Factory::new(
                protocol.get_factory_address(),
                provider.clone(),
            ));
        }

        Self {
            provider: provider.clone(),
            router_mapping: router_list,
            factory_mapping: factory_list,
        }
    }

    pub async fn quote(
        &self,
        protocol: UniswapV2,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> U256 {
        let router = &self.router_mapping[protocol as usize];
        let result = router
            .get_amounts_out(
                amount_in,
                vec![token_in.get_address(), token_out.get_address()],
            )
            .call()
            .await
            .unwrap();
        return result[1];
    }

    pub async fn get_pair_address(
        &self,
        protocol: UniswapV2,
        token0: ERC20Token,
        token1: ERC20Token,
    ) -> Address {
        let factory = &self.factory_mapping[protocol as usize];
        let pair_address: Address = factory
            .get_pair(token0.get_address(), token1.get_address())
            .call()
            .await
            .unwrap();
        return pair_address;
    }

    pub async fn get_pair_address_multicall(
        &self,
        http_provider: Provider<Http>,
        pairs_list: Vec<(UniswapV2, ERC20Token, ERC20Token)>,
    ) -> Vec<Address> {
        let provider = Arc::new(http_provider);

        let mut multicall = Multicall::new(Arc::clone(&provider), None).await.unwrap();

        for pair in pairs_list {
            let (protocol, token0, token1) = pair;
            let uniswapV2_pair_abi: Abi = serde_json::from_str(
                r#"[{
                "constant": true,
                "inputs": [
                    {
                        "internalType": "address",
                        "name": "tokenA",
                        "type": "address"
                    },
                    {
                        "internalType": "address",
                        "name": "tokenB",
                        "type": "address"
                    }
                ],
                "name": "getPair",
                "outputs": [
                    {
                        "internalType": "address",
                        "name": "pair",
                        "type": "address"
                    }
                ],
                "payable": false,
                "stateMutability": "view",
                "type": "function"
            }]"#,
            )
            .unwrap();

            let contract = Contract::<Provider<Http>>::new(
                protocol.get_factory_address(),
                uniswapV2_pair_abi,
                Arc::clone(&provider),
            );

            let call = contract
                .method::<_, Address>("getPair", (token0.get_address(), token1.get_address()))
                .unwrap();

            multicall.add_call(call, false);
        }

        let return_data: Vec<Token> = multicall.call_raw().await.unwrap();
        let mut data: Vec<Address> = Vec::new();
        for token in return_data {
            let token = token.into_tuple().unwrap();
            let token = &token[1];
            let val = match token {
                Address(a) => *a,
                _ => "0x0".parse::<Address>().unwrap(),
            };
            data.push(val);
        }
        return data;
    }

    pub async fn get_pair_reserves(&self, pair_address: Address) -> (u128, u128) {
        let pair_contract = IUniswapV2Pair::new(pair_address, self.provider.clone());
        let (reserve0, reserve1, _): (u128, u128, u32) =
            pair_contract.get_reserves().call().await.unwrap();
        return (reserve0, reserve1);
    }

    pub async fn get_pair_reserves_multicall(
        &self,
        http_provider: Provider<Http>,
        pair_addresses: &Vec<Address>,
    ) -> Vec<(U256, U256)> {
        let provider = Arc::new(http_provider);

        let mut multicall = Multicall::new(Arc::clone(&provider), None).await.unwrap();

        for pair_address in pair_addresses {
            let uniswapV2_pair_abi: Abi = serde_json::from_str(
                r#"[{
                    "constant": true,
                    "inputs": [],
                    "name": "getReserves",
                    "outputs": [
                        {
                            "internalType": "uint112",
                            "name": "reserve0",
                            "type": "uint112"
                        },
                        {
                            "internalType": "uint112",
                            "name": "reserve1",
                            "type": "uint112"
                        },
                        {
                            "internalType": "uint32",
                            "name": "blockTimestampLast",
                            "type": "uint32"
                        }
                    ],
                    "payable": false,
                    "stateMutability": "view",
                    "type": "function"
                }]"#,
            )
            .unwrap();

            let contract = Contract::<Provider<Http>>::new(
                *pair_address,
                uniswapV2_pair_abi,
                Arc::clone(&provider),
            );

            let call = contract
                .method::<_, (u128, u128)>("getReserves", ())
                .unwrap();

            multicall.add_call(call, false);
        }

        let return_data: Vec<Token> = multicall.call_raw().await.unwrap();
        let mut data: Vec<(U256, U256)> = Vec::new();
        for token in return_data {
            let token = token.into_tuple().unwrap();
            let token = match &token[1] {
                Tuple(tuple) => tuple,
                _ => todo!(),
            };
            let val1 = match token[0] {
                Uint(a) => a,
                _ => U256::zero(),
            };
            let val2 = match token[1] {
                Uint(a) => a,
                _ => U256::zero(),
            };
            data.push((val1, val2));
        }
        return data;
    }
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use ethers::providers::{Http, Provider, Ws};
    use ethers::types::{Address, U256};

    use crate::constants::protocol::UniswapV2::*;
    use crate::constants::token::ERC20Token::{USDC, USDT, WETH, WMATIC};

    use super::{UniswapV2Client, UniswapV2Pair};

    #[tokio::test]
    async fn test_get_pair_address() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);
        let pair_address = uniswapV2_client
            .get_pair_address(SUSHISWAP, USDC, WETH)
            .await;
        assert_eq!(
            Address::from_str("0x34965ba0ac2451a34a0471f04cca3f990b8dea27").unwrap(),
            pair_address
        );
    }

    #[tokio::test]
    async fn test_get_pair_reserves() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);
        let pair_address = uniswapV2_client
            .get_pair_address(SUSHISWAP, USDC, WETH)
            .await;

        // TODO - assert_eq! to something here (or add any general check)
        uniswapV2_client.get_pair_reserves(pair_address).await;
    }

    #[tokio::test]
    async fn test_get_pair_address_multicall() {
        dotenv::dotenv().ok();
        let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL").unwrap();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider = Provider::<Http>::try_from(&rpc_node_url).unwrap();
        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);

        let pairs_list = vec![(SUSHISWAP, USDC, USDT), (SUSHISWAP, USDC, WETH)];

        let results = uniswapV2_client
            .get_pair_address_multicall(provider, pairs_list)
            .await;
        println!("{:?}", results);
    }

    #[tokio::test]
    async fn test_get_pair_reserves_multicall() {
        dotenv::dotenv().ok();
        let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL").unwrap();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider = Provider::<Http>::try_from(&rpc_node_url).unwrap();
        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);
        let pair_addresses = vec![
            "0x34965ba0ac2451a34a0471f04cca3f990b8dea27"
                .parse::<Address>()
                .unwrap(),
            "0x34965ba0ac2451a34a0471f04cca3f990b8dea27"
                .parse::<Address>()
                .unwrap(),
        ];
        let result = uniswapV2_client
            .get_pair_reserves_multicall(provider, &pair_addresses)
            .await;
        println!("{:?}", result);
    }

    #[tokio::test]
    async fn test_get_amount_out() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);
        let uniswapV2_client = UniswapV2Client::new(provider_ws);

        let route = (SUSHISWAP, USDT, WMATIC);

        // load in pair and save reserve data
        let amount_in = U256::from(1000) * U256::exp10(route.1.get_decimals().into());
        println!("AMOUTN INT {:?}", amount_in);
        let pair_address = uniswapV2_client
            .get_pair_address(route.0, route.1, route.2)
            .await;
        let amount_out = uniswapV2_client
            .quote(route.0, route.1, route.2, amount_in)
            .await;

        let (reserve0, reserve1) = uniswapV2_client.get_pair_reserves(pair_address).await;
        println!("{:?}", pair_address);
        println!("{}, {}", reserve0, reserve1);
        let reserve0 = U256::from(reserve0);
        let reserve1 = U256::from(reserve1);
        let mut pair = UniswapV2Pair::default();
        pair.update_metadata(route.0, route.1, route.2);
        pair.update_reserves(reserve0, reserve1);
        let i_amount_out = pair.get_amounts_out(amount_in, false);
        println!(
            "Uniswap get_amounts_out: {}, internal get_amounts_out: {}",
            amount_out, i_amount_out
        );
    }
}
