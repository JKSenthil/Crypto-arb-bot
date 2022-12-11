use std::{
    ops::{Add, Mul},
    sync::Arc,
};

use ethers::{
    abi::Token::{self, *},
    contract::Contract,
    core::abi::Abi,
    prelude::{abigen, builders::ContractCall},
    providers::Middleware,
    types::{Address, U256},
};
use log::{debug, error, warn};

use crate::{
    constants::{
        protocol::UniswapV2,
        token::{ERC20Lookup, ERC20Token},
    },
    utils::multicall::Multicall,
};

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
    fees: U256,
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
            fees: U256::zero(),
        }
    }

    pub fn update_metadata(
        &mut self,
        protocol: UniswapV2,
        token0: ERC20Token,
        token1: ERC20Token,
        fees: U256,
    ) {
        self.protocol = protocol;
        self.token0 = token0;
        self.token1 = token1;
        self.fees = fees;
    }

    pub fn update_reserves(&mut self, reserve0: U256, reserve1: U256) {
        self.reserve0 = reserve0;
        self.reserve1 = reserve1;
    }

    fn get_amount_out(self, amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
        if reserve_in == U256::zero() || reserve_out == U256::zero() {
            return U256::zero();
        }
        // account for each exchange's fees
        let (numerator_fee_mul, denominator_fee_mul) = match self.protocol {
            UniswapV2::MESHSWAP => (10000 - self.fees.as_u32(), 10000_u32),
            UniswapV2::POLYCAT => (9976_u32, 10000_u32),
            UniswapV2::APESWAP => (998_u32, 1000_u32),
            _ => (997_u32, 1000_u32),
        };
        let amount_in_with_fee: U256 = amount_in.mul(numerator_fee_mul);
        let numerator: U256 = amount_in_with_fee.mul(reserve_out);
        let denominator: U256 = reserve_in.mul(denominator_fee_mul).add(amount_in_with_fee);
        numerator / denominator
    }

    pub fn get_amounts_out(&self, amount_in: U256, token: ERC20Token) -> U256 {
        if token == self.token0 {
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

    pub fn get_quote_txn(
        &self,
        protocol: UniswapV2,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> ContractCall<M, Vec<ethers::prelude::U256>> {
        let router = &self.router_mapping[protocol as usize];
        return router.get_amounts_out(
            amount_in,
            vec![token_in.get_address(), token_out.get_address()],
        );
    }

    // function swapExactTokensForTokens(
    //     uint256 amountIn,
    //     uint256 amountOutMin,
    //     address[] calldata path,
    //     address to,
    //     uint256 deadline
    // ) external returns (uint256[] memory amounts);
    pub fn get_swapExactTokensForTokens_txn(
        &self,
        protocol: UniswapV2,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> ContractCall<M, Vec<U256>> {
        let router = &self.router_mapping[protocol as usize];
        let path = vec![token_in, token_out]
            .into_iter()
            .map(|x| x.get_address())
            .collect();
        return router.swap_exact_tokens_for_tokens(
            amount_in,
            U256::one(),
            path,
            "0x06a92D032d97D5a3c9F550e551B4B6f42518A07B"
                .parse::<Address>()
                .unwrap(),
            U256::from(2670725202_u32),
        );
    }

    pub async fn quote(
        &self,
        protocol: UniswapV2,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> U256 {
        let result = self
            .get_quote_txn(protocol, token_in, token_out, amount_in)
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
        pairs_list: Vec<(UniswapV2, ERC20Token, ERC20Token)>,
    ) -> Vec<Address> {
        let mut multicall = Multicall::new(self.provider.clone());

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

            let contract = Contract::new(
                protocol.get_factory_address(),
                uniswapV2_pair_abi,
                self.provider.clone(),
            );

            let call = contract
                .method::<_, Address>("getPair", (token0.get_address(), token1.get_address()))
                .unwrap();

            multicall.add_call(call);
        }

        let return_data = multicall.call_raw().await;
        let mut data: Vec<Address> = Vec::with_capacity(return_data.len());
        for token in return_data {
            let address: Address;
            match token {
                Some(tokens) => {
                    match &tokens[0] {
                        Address(a) => {
                            address = *a;
                        }
                        _ => {
                            address = "0x0".parse::<Address>().unwrap();
                        }
                    };
                }
                None => {
                    address = "0x0".parse::<Address>().unwrap();
                }
            };
            data.push(address);
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
        pair_addresses: &Vec<Address>,
    ) -> Vec<(U256, U256)> {
        let mut multicall = Multicall::new(self.provider.clone());

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

            let contract = Contract::new(*pair_address, uniswapV2_pair_abi, self.provider.clone());

            let call = contract
                .method::<_, (u128, u128)>("getReserves", ())
                .unwrap();

            multicall.add_call(call);
        }

        let return_data: Vec<Option<Vec<Token>>> = multicall.call_raw().await;
        let mut data: Vec<(U256, U256)> = Vec::new();
        for token in return_data {
            match token {
                Some(tokens) => {
                    let val1 = &tokens[0];
                    let val2 = &tokens[1];
                    let val1 = match val1 {
                        Uint(a) => *a,
                        _ => U256::zero(),
                    };
                    let val2 = match val2 {
                        Uint(a) => *a,
                        _ => U256::zero(),
                    };
                    data.push((val1, val2));
                }
                None => {
                    data.push((U256::zero(), U256::zero()));
                }
            }
        }
        return data;
    }

    pub async fn get_pair_metadata(&self, pair_address: Address) -> (ERC20Token, ERC20Token, U256) {
        let pair_contract = IUniswapV2Pair::new(pair_address, self.provider.clone());
        let token_0_address = pair_contract.token_0().call().await.unwrap();
        let token_1_address = pair_contract.token_1().call().await.unwrap();
        let fees = pair_contract.fee().call().await.unwrap_or(U256::zero());
        (
            ERC20Lookup(token_0_address),
            ERC20Lookup(token_1_address),
            fees,
        )
    }

    pub async fn get_pair_metadata_multicall(
        &self,
        pair_addresses: &Vec<Address>,
    ) -> Vec<(ERC20Token, ERC20Token, U256)> {
        let mut multicall0 = Multicall::new(self.provider.clone());
        let mut multicall1 = Multicall::new(self.provider.clone());
        let mut multicall_fees = Multicall::new(self.provider.clone());

        for pair_address in pair_addresses {
            let contract = IUniswapV2Pair::new(*pair_address, self.provider.clone());
            let contract_call0 = contract.token_0();
            let contract_call1 = contract.token_1();
            let contract_call_fee = contract.fee();
            multicall0.add_call(contract_call0);
            multicall1.add_call(contract_call1);
            multicall_fees.add_call(contract_call_fee);
        }
        let return_data0: Vec<Option<Vec<Token>>> = multicall0.call_raw().await;
        let return_data1: Vec<Option<Vec<Token>>> = multicall1.call_raw().await;
        let return_data_fee: Vec<Option<Vec<Token>>> = multicall_fees.call_raw().await;
        let mut data: Vec<(ERC20Token, ERC20Token, U256)> = Vec::new();
        for (i, tokens0) in return_data0.into_iter().enumerate() {
            let mut tuple = (ERC20Token::USDC, ERC20Token::USDC, U256::zero());
            match &return_data1[i] {
                Some(tokens) => {
                    let token = &tokens[0];
                    match token {
                        Address(addr) => {
                            tuple.1 = ERC20Lookup(*addr);
                        }
                        _ => {
                            error!("error in parsing token in metadata multicall");
                        }
                    };
                }
                _ => {
                    warn!(
                        "error in getting token in metadata multicall, pair address: {:?}",
                        pair_addresses[i]
                    );
                }
            };
            match &tokens0 {
                Some(tokens) => {
                    let token = &tokens[0];
                    match token {
                        Address(addr) => {
                            tuple.0 = ERC20Lookup(*addr);
                        }
                        _ => {
                            error!("error in parsing token in metadata multicall");
                        }
                    };
                }
                _ => {
                    warn!("error in getting token in metadata multicall");
                }
            };

            match &return_data_fee[i] {
                Some(tokens) => {
                    let token = &tokens[0];
                    match token {
                        Uint(num) => {
                            debug!("FEE%: {:?}", *num);
                            tuple.2 = *num;
                        }
                        _ => {}
                    }
                }
                None => {}
            }
            data.push(tuple);
        }
        return data;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use ethers::providers::{Provider, Ws};
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
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);

        let pairs_list = vec![(SUSHISWAP, USDC, USDT), (SUSHISWAP, USDC, WETH)];

        let results = uniswapV2_client
            .get_pair_address_multicall(pairs_list)
            .await;
        println!("{:?}", results);
    }

    #[tokio::test]
    async fn test_get_pair_reserves_multicall() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);
        let pair_addresses = vec![
            "0x34965ba0ac2451a34a0471f04cca3f990b8dea26"
                .parse::<Address>()
                .unwrap(),
            "0x34965ba0ac2451a34a0471f04cca3f990b8dea27"
                .parse::<Address>()
                .unwrap(),
        ];
        let result = uniswapV2_client
            .get_pair_reserves_multicall(&pair_addresses)
            .await;
        println!("{:?}", result);
    }

    #[tokio::test]
    async fn test_get_pair_metadata_multicall() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let uniswapV2_client = UniswapV2Client::new(provider_ws);
        let pair_addresses = vec!["0x34965ba0ac2451a34a0471f04cca3f990b8dea27"
            .parse::<Address>()
            .unwrap()];
        let result = uniswapV2_client
            .get_pair_metadata_multicall(&pair_addresses)
            .await;
        println!(
            "{:?}",
            result
                .into_iter()
                .map(|x| (x.0.get_symbol(), x.1.get_symbol()))
        );
    }

    #[tokio::test]
    async fn test_get_amount_out() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);
        let uniswapV2_client = UniswapV2Client::new(provider_ws);

        let routes = [
            (MESHSWAP, USDC, WETH),
            (SUSHISWAP, USDC, WETH),
            (APESWAP, USDC, WETH),
            (POLYCAT, USDC, WETH),
        ];

        for route in routes {
            let amount_in = U256::from(1000) * U256::exp10(route.1.get_decimals().into());
            // load in pair and save reserve data
            let pair_address = uniswapV2_client
                .get_pair_address(route.0, route.1, route.2)
                .await;
            let amount_out = uniswapV2_client
                .quote(route.0, route.1, route.2, amount_in)
                .await;

            let (reserve0, reserve1) = uniswapV2_client.get_pair_reserves(pair_address).await;
            let reserve0 = U256::from(reserve0);
            let reserve1 = U256::from(reserve1);
            let mut pair = UniswapV2Pair::default();
            let (token0, token1, fees) = uniswapV2_client.get_pair_metadata(pair_address).await;
            pair.update_metadata(route.0, token0, token1, fees);
            pair.update_reserves(reserve0, reserve1);
            let i_amount_out = pair.get_amounts_out(amount_in, route.1);
            assert_eq!(amount_out, i_amount_out);
        }
    }
}
