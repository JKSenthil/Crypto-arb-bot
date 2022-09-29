use std::clone::Clone;
use std::sync::Arc;

use ethers::abi;
use ethers::abi::Token;
use ethers::prelude::abigen;
use ethers::providers::Middleware;
use ethers::types::Address;
use ethers::types::Bytes;
use ethers::types::U256;

use crate::consts::ERC20Token;
use crate::consts::Protocol;
use crate::utils::parse_address;

abigen!(IUniswapV2Router02, "abis/IUniswapV2Router02.json");
abigen!(Quoter, "abis/Quoter.json");

pub struct Price<M> {
    uniswap_v2: [IUniswapV2Router02<M>; 5],
    uniswap_v3: Quoter<M>,
}

impl<M: Middleware> Price<M> {
    pub fn new<T: Into<Arc<M>> + Clone>(provider: T) -> Self {
        return Self {
            uniswap_v2: Protocol::uniswap_v2_protocols().map(|protocol| {
                IUniswapV2Router02::new(
                    parse_address(protocol.get_router_address()),
                    provider.clone().into(),
                )
            }),
            uniswap_v3: Quoter::new(
                parse_address(Protocol::UNISWAP_V3.get_router_address()),
                provider.clone().into(),
            ),
        };
    }

    pub async fn quote(
        &self,
        protocol: Protocol,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> U256 {
        if protocol.is_uniswapV2_protocol() {
            let result = self.uniswap_v2[protocol as usize]
                .get_amounts_out(
                    amount_in,
                    vec![
                        parse_address(token_in.get_token_addr()),
                        parse_address(token_out.get_token_addr()),
                    ],
                )
                .call()
                .await
                .unwrap();
            return result[1];
        }

        let result = self
            .uniswap_v3
            .quote_exact_input_single(
                parse_address(token_in.get_token_addr()),
                parse_address(token_out.get_token_addr()),
                100, // hardcode uniswapV3 fee for now
                amount_in,
                U256::zero(),
            )
            .call()
            .await
            .unwrap();
        result
    }

    pub async fn quote_route(
        &self,
        protocol: Protocol,
        path: Vec<ERC20Token>,
        amount_in: U256,
    ) -> U256 {
        let path: Vec<Address> = path
            .iter()
            .map(|x| parse_address(x.get_token_addr()))
            .collect();
        if protocol.is_uniswapV2_protocol() {
            let result = self.uniswap_v2[protocol as usize]
                .get_amounts_out(amount_in, path)
                .call()
                .await
                .unwrap();
            return result[result.len() - 1];
        }

        // note UniswapV3 does not work at this moment,
        // the below code is balony
        let mut p = Vec::new();
        for i in 0..path.len() - 1 {
            p.push(Token::Address(path[i]));
            p.push(Token::Address(path[i + 1]));
            p.push(Token::Uint(U256::from(500)));
        }

        println!("Size of vec: {}", p.len());
        println!("Size of vec: {:?}", p);

        let result = self
            .uniswap_v3
            .quote_exact_input(Bytes::from(abi::encode(&p)), amount_in)
            .call()
            .await
            .unwrap();

        result
    }
}
