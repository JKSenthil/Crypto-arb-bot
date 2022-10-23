use std::clone::Clone;
use std::sync::Arc;

use enum_map::EnumMap;
use ethers::prelude::abigen;
use ethers::providers::Middleware;
use ethers::types::U256;

use crate::constants::token::ERC20Token;
use crate::constants::Protocol;

abigen!(IUniswapV2Router02, "abis/IUniswapV2Router02.json");
abigen!(Quoter, "abis/Quoter.json");

enum ProtocolRouter<M> {
    UniswapV2 { router: IUniswapV2Router02<M> },
    UniswapV3 { router: Quoter<M> },
}
pub struct Price<M> {
    router_mapping: EnumMap<Protocol, ProtocolRouter<M>>,
}

impl<M: Middleware> Price<M> {
    pub fn new<T: Into<Arc<M>> + Clone>(provider: T, protocols: Vec<Protocol>) -> Self {
        let mut router_mapping: EnumMap<Protocol, ProtocolRouter<M>> = EnumMap::default();

        for protocol in protocols {
            router_mapping[protocol] = match protocol {
                Protocol::UniswapV2 { address } => ProtocolRouter::UniswapV2 {
                    router: IUniswapV2Router02::new(address, provider.clone().into()),
                },
                Protocol::UniswapV3 { fee } => ProtocolRouter::UniswapV3 {
                    router: Quoter::new(Protocol::V3_ADDRESS, provider.clone().into()),
                },
            }
        }

        return Self {
            router_mapping: router_mapping,
        };
    }

    pub async fn quote(
        &self,
        protocol: Protocol,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> U256 {
        match protocol {
            Protocol::UniswapV2 { .. } => {
                let router = self.router_mapping[protocol];
                let result = router
                    .get_amounts_out(amount_in, vec![token_in.address, token_out.address])
                    .call()
                    .await
                    .unwrap();
                result[1]
            }
            Protocol::UniswapV3 { fee } => {
                let router = self.router_mapping[protocol];
                let result = router
                    .quote_exact_input_single(
                        token_in.address,
                        token_out.address,
                        fee,
                        amount_in,
                        U256::zero(),
                    )
                    .call()
                    .await
                    .unwrap();
                result
            }
        }
    }
}
