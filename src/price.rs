use std::clone::Clone;
use std::str::FromStr;
use std::sync::Arc;

use ethers::core::types::Bytes;
use ethers::prelude::abigen;
use ethers::providers::Middleware;
use ethers::types::Address;
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
                provider.into(),
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
        if protocol.is_uniswapV2_protocol() {
            let path: Vec<Address> = path
                .iter()
                .map(|x| parse_address(x.get_token_addr()))
                .collect();
            let result = self.uniswap_v2[protocol as usize]
                .get_amounts_out(amount_in, path)
                .call()
                .await
                .unwrap();
            return result[result.len() - 1];
        }

        // note UniswapV3 does not work at this moment,
        // the below code is balony
        let mut vec = Vec::new();
        println!(
            "SIZE: {}",
            Bytes::from_str(path[0].get_token_addr())
                .unwrap()
                .to_vec()
                .len()
        );
        for i in 0..path.len() - 1 {
            let mut p1 = Bytes::from_str(path[i].get_token_addr()).unwrap().to_vec();
            p1.reverse(); // reverse for little endian form
            vec.extend(p1);

            let mut p2 = Bytes::from_str(path[i + 1].get_token_addr())
                .unwrap()
                .to_vec();
            p2.reverse();
            vec.extend(p2);

            vec.extend(vec![244, 1, 0]); // 500 in u8 binary form (little endian)
        }

        println!("Size of vec: {}", vec.len());
        println!("Size of vec: {:?}", vec);

        let result = self
            .uniswap_v3
            .quote_exact_input(Bytes::from(vec), amount_in)
            .call()
            .await
            .unwrap();

        result
    }
}
