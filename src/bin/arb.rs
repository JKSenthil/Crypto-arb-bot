use dotenv::dotenv;
use enum_map::enum_map;
use ethers::{prelude::k256::sha2::digest::typenum::Le, types::Address};
use serde::{Deserialize, Serialize};
use std::fs;

use cryptorocket::constants::token::USDC;

#[derive(Serialize, Deserialize, Debug)]
pub struct ERC20Token {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let token_list_data = fs::read_to_string("data/polygon_tokens.json")?;
    let token_list: Vec<ERC20Token> = serde_json::from_str(&token_list_data)?;

    println!("Num tokens: {}", token_list.len());
    println!("{:?}", token_list[1]);

    println!("{:?}", USDC);

    Ok(())
}
