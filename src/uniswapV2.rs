use std::{
    ops::{Add, Mul},
    sync::Arc,
};

use ethers::{
    contract::Contract,
    prelude::{abigen, Multicall},
    providers::{Http, Middleware, Provider, Ws},
    types::{Address, U256},
};

use crate::constants::{protocol::UniswapV2, token::ERC20Token};

abigen!(IUniswapV2Router02, "abis/IUniswapV2Router02.json");
abigen!(IUniswapV2Factory, "abis/IUniswapV2Factory.json");
abigen!(IUniswapV2Pair, "abis/IUniswapV2Pair.json");

pub struct UniswapV2Pair {
    protocol: UniswapV2,
    address: Address,
    token0: ERC20Token,
    token1: ERC20Token,
    reserve0: U256,
    reserve1: U256,
}

impl UniswapV2Pair {
    // TODO - verify all dexes have same get_amount_out implementation!
    fn get_amount_out(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
        let amount_in_with_fee: U256 = amount_in.mul(997);
        let numerator: U256 = amount_in_with_fee.mul(reserve_out);
        let denominator: U256 = reserve_in.mul(1000_u32).add(amount_in_with_fee);
        return numerator / denominator;
    }

    pub fn get_amounts_out(&self, amount_in: U256, token0: bool) -> U256 {
        if token0 {
            return UniswapV2Pair::get_amount_out(amount_in, self.reserve0, self.reserve1);
        }
        return UniswapV2Pair::get_amount_out(amount_in, self.reserve1, self.reserve0);
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
        pairs_list: Vec<(UniswapV2, ERC20Token, ERC20Token)>,
    ) {
        // use makerdao's multicall contract
        let mut multicall = Multicall::new(
            &self.provider,
            Some(
                "0x11ce4B23bD875D7F5C6a31084f55fDe1e9A87507"
                    .parse::<Address>()
                    .unwrap(),
            ),
        );

        for pair in pairs_list {
            let (protocol, token0, token1) = pair;
            let factory = &self.factory_mapping[protocol as usize];

            let abi = IUNISWAPV2FACTORY_ABI;
            let contract =
                Contract::<Provider<Http>>::new(protocol.get_factory_address(), abi, self.provider);

            let call = factory.get_pair(token0.get_address(), token1.get_address());
            multicall = multicall.add_call(call);
        }
    }

    pub async fn get_pair_reserves(&self, pair_address: Address) -> (u128, u128) {
        let pair_contract = IUniswapV2Pair::new(pair_address, self.provider.clone());
        let (reserve0, reserve1, _): (u128, u128, u32) =
            pair_contract.get_reserves().call().await.unwrap();
        return (reserve0, reserve1);
    }
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use ethers::providers::{Provider, Ws};
    use ethers::types::Address;

    use crate::constants::protocol::UniswapV2::SUSHISWAP;
    use crate::constants::token::ERC20Token::{USDC, WETH};

    use super::UniswapV2Client;

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
}
