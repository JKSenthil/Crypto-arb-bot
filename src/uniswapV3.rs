use std::sync::Arc;

use ethers::{
    abi::{Abi, Token::Uint},
    contract::Contract,
    prelude::abigen,
    providers::Middleware,
    types::{Address, U256},
};

use crate::{constants::token::ERC20Token, utils::multicall::Multicall};

abigen!(Quoter, "abis/uniswap/v3/Quoter.json");

static QUOTE_ABI_STR: &str = r#"[{
    "inputs": [
      {
        "internalType": "address",
        "name": "tokenIn",
        "type": "address"
      },
      {
        "internalType": "address",
        "name": "tokenOut",
        "type": "address"
      },
      {
        "internalType": "uint24",
        "name": "fee",
        "type": "uint24"
      },
      {
        "internalType": "uint256",
        "name": "amountIn",
        "type": "uint256"
      },
      {
        "internalType": "uint160",
        "name": "sqrtPriceLimitX96",
        "type": "uint160"
      }
    ],
    "name": "quoteExactInputSingle",
    "outputs": [
      {
        "internalType": "uint256",
        "name": "amountOut",
        "type": "uint256"
      }
    ],
    "stateMutability": "nonpayable",
    "type": "function"
  }]"#;

pub struct UniswapV3Client<M> {
    provider: Arc<M>,
    quoter: Quoter<M>,
    quote_contract: Contract<M>,
}

impl<M: Middleware + Clone> UniswapV3Client<M> {
    pub fn new(provider: Arc<M>) -> Self {
        let router_address = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
            .parse::<Address>()
            .unwrap();
        let quote_abi: Abi = serde_json::from_str(QUOTE_ABI_STR).unwrap();
        Self {
            provider: provider.clone(),
            quoter: Quoter::new(router_address, provider.clone()),
            quote_contract: Contract::new(router_address, quote_abi, provider.clone()),
        }
    }

    pub async fn quote(
        &self,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
        fee: u32,
    ) -> U256 {
        let amount_out = self
            .quoter
            .quote_exact_input_single(
                token_in.get_address(),
                token_out.get_address(),
                fee,
                amount_in,
                U256::zero(),
            )
            .call()
            .await
            .unwrap();
        amount_out
    }

    /// Returns best quote, returns fee where quote exists
    pub async fn quote_multicall(
        &self,
        token_in: ERC20Token,
        token_out: ERC20Token,
        amount_in: U256,
    ) -> (u32, U256) {
        let fees: [u32; 4] = [100, 500, 3000, 10000];
        let mut multicall = Multicall::new(self.provider.clone());

        for fee in fees {
            let call = self
                .quote_contract
                .method::<_, U256>(
                    "quoteExactInputSingle",
                    (
                        token_in.get_address(),
                        token_out.get_address(),
                        fee,
                        amount_in,
                        U256::zero(),
                    ),
                )
                .unwrap();
            multicall.add_call(call);
        }

        let return_data = multicall.call_raw().await;
        let mut amount_outs: [(u32, U256); 4] = [
            (fees[0], U256::zero()),
            (fees[1], U256::zero()),
            (fees[2], U256::zero()),
            (fees[3], U256::zero()),
        ];

        for (i, token) in return_data.iter().enumerate() {
            match token {
                Some(tokens) => {
                    let val1 = &tokens[0];
                    let val1 = match val1 {
                        Uint(a) => *a,
                        _ => U256::zero(),
                    };
                    amount_outs[i].1 = val1;
                }
                None => {}
            }
        }

        *amount_outs
            .iter()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Instant};

    use ethers::{
        providers::{Http, Provider, Ws},
        types::U256,
    };

    use super::UniswapV3Client;
    use crate::constants::token::ERC20Token::{DAI, USDC, USDT, WETH};

    #[tokio::test]
    async fn test_quote() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);
        let uniswapV3_client = UniswapV3Client::new(provider_ws);

        let token_in = USDC;
        let token_out = USDT;
        let amount_in = U256::from(1000) * U256::exp10(token_in.get_decimals().into());
        let fee = 3000;

        let amounts_out = uniswapV3_client
            .quote(token_in, token_out, amount_in, fee)
            .await;
        println!("{}", amounts_out);
    }

    #[tokio::test]
    async fn test_quote_multicall() {
        dotenv::dotenv().ok();
        let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL").unwrap();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_http = Provider::<Http>::try_from(&rpc_node_url).unwrap();
        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);
        let uniswapV3_client = UniswapV3Client::new(provider_ws);

        let token_in = WETH;
        let token_out = USDT;
        let amount_in = U256::from(30) * U256::exp10(token_in.get_decimals().into());

        let amounts_out = uniswapV3_client
            .quote_multicall(token_in, token_out, amount_in)
            .await;
        println!("{:?}", amounts_out);
    }
}
