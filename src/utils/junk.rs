use ethers::types::U256;
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    RequestBuilder,
};

pub fn to_U256(input: u32, decimal: u8) -> U256 {
    U256::from(input) * U256::exp10(decimal.into())
}

struct DebugTxClient {
    request_builder: RequestBuilder,
}

impl DebugTxClient {
    pub fn new(uri: &str) -> Self {
        let client = reqwest::Client::new();
        let request_builder = client
            .post(uri)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json");
        return Self {
            request_builder: request_builder,
        };
    }

    pub async fn debug_trace_transaction(
        &self,
        tx_hash: &str,
        encoded_function_preface: &str,
    ) -> Option<String> {
        let args = format!(
            "
        {{
            \"id\": 1,
            \"jsonrpc\": \"2.0\",
            \"method\": \"debug_traceTransaction\",
            \"params\": [
                \"{}\",
                {{
                    \"tracer\": \"callTracer\",
                    \"timeout\": \"5s\"
                }}
            ]
        }}
        ",
            tx_hash
        );
        let response = self
            .request_builder
            .try_clone()
            .unwrap()
            .body(args)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        match response.find(encoded_function_preface) {
            Some(index) => {
                let str = &response[index..index + 330];
                Some(str.to_string())
            }
            None => None,
        }
    }
}

// use ethers::types::U256;
// use lazy_static::lazy_static;

// use crate::utils::convert_to_U256;

// use super::ERC20Token::*;
// use super::Protocol::*;
// use super::{ERC20Token, Protocol};

// pub struct Route {
//     pub path: Vec<ERC20Token>,
//     pub protocols: Vec<Protocol>,
//     pub amount_in: U256,
// }

// lazy_static! {
//     pub static ref ROUTES: Vec<Route> = vec![
//         Route {
//             path: vec![USDC, WETH, USDC],
//             protocols: vec![UNISWAP_V3, SUSHISWAP],
//             amount_in: convert_to_U256(1300, USDC.get_token_decimal())
//         },
//         Route {
//             path: vec![USDC, WBTC, USDC],
//             protocols: vec![UNISWAP_V3, SUSHISWAP],
//             amount_in: convert_to_U256(19000, USDC.get_token_decimal())
//         },
//         Route {
//             path: vec![WETH, USDC, WETH],
//             protocols: vec![UNISWAP_V3, SUSHISWAP],
//             amount_in: convert_to_U256(1, WETH.get_token_decimal())
//         },
//         Route {
//             path: vec![WBTC, USDC, WBTC],
//             protocols: vec![UNISWAP_V3, SUSHISWAP],
//             amount_in: convert_to_U256(1, WBTC.get_token_decimal())
//         }
//     ];
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct ERC20Token {
//     pub address: Address,
//     pub name: String,
//     pub symbol: String,
//     pub decimals: u8,
// }

// let token_list_data = fs::read_to_string("data/polygon_tokens.json")?;
// let token_list: Vec<ERC20Token> = serde_json::from_str(&token_list_data)?;

// println!("Num tokens: {}", token_list.len());
// println!("{:?}", token_list[1]);

// println!("{:?}", USDC);
