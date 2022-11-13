use ethers::prelude::abigen;

use crate::constants::token::ERC20Token::{self, *};

abigen!(Flashloan, "abis/Flashloan.json");

#[inline(always)]
fn threshold(token: ERC20Token, amount_diff: f64) -> bool {
    match token {
        USDC => amount_diff >= 0.02,
        USDT => amount_diff >= 0.02,
        DAI => amount_diff >= 0.02,
        WMATIC => amount_diff >= 0.02,
        WETH => amount_diff >= 0.00005,
        _ => false,
    }
}
