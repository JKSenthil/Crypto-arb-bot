use ethers::types::U256;
use lazy_static::lazy_static;

use crate::utils::convert_to_U256;

use super::ERC20Token::*;
use super::Protocol::*;
use super::{ERC20Token, Protocol};

pub struct Route {
    pub path: Vec<ERC20Token>,
    pub protocols: Vec<Protocol>,
    pub amount_in: U256,
}

lazy_static! {
    pub static ref ROUTES: Vec<Route> = vec![
        Route {
            path: vec![USDC, WETH, USDC],
            protocols: vec![UNISWAP_V3, SUSHISWAP],
            amount_in: convert_to_U256(1300, USDC.get_token_decimal())
        },
        Route {
            path: vec![USDC, WBTC, USDC],
            protocols: vec![UNISWAP_V3, SUSHISWAP],
            amount_in: convert_to_U256(19000, USDC.get_token_decimal())
        },
        Route {
            path: vec![WETH, USDC, WETH],
            protocols: vec![UNISWAP_V3, SUSHISWAP],
            amount_in: convert_to_U256(1, WETH.get_token_decimal())
        },
        Route {
            path: vec![WBTC, USDC, WBTC],
            protocols: vec![UNISWAP_V3, SUSHISWAP],
            amount_in: convert_to_U256(1, WBTC.get_token_decimal())
        }
    ];
}
