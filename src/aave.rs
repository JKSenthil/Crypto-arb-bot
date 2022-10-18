use std::sync::Arc;

use ethers::{
    prelude::abigen,
    providers::Middleware,
    types::{Address, Bytes, U256},
};

use crate::utils::parse_address;

abigen!(Pool, "abis/AavePool.json");

static POOL_ADDR: &str = "0x794a61358D6845594F94dc1DB02A252b5b4814aD";

#[derive(Debug)]
pub struct LiquidationCallArgs {
    collateral: Address,
    debt: Address,
    user: Address,
    debtToCover: U256,
    receiveAToken: bool,
}

pub struct Aave<M> {
    pool: Pool<M>,
}

impl<M: Middleware> Aave<M> {
    pub fn new<T: Into<Arc<M>> + Clone>(provider: T) -> Self {
        return Self {
            pool: Pool::new(parse_address(POOL_ADDR), provider.into()),
        };
    }

    pub fn parse_liquidation_call_input(&self, txn_input: Bytes) -> Option<LiquidationCallArgs> {
        if let Ok((collateral, debt, user, debtToCover, receiveAToken)) =
            self.pool.decode("LiquidationCall", txn_input)
        {
            return Some(LiquidationCallArgs {
                collateral,
                debt,
                user,
                debtToCover,
                receiveAToken,
            });
        }
        None
    }

    pub async fn get_user_account_data(
        &self,
        address: &str,
    ) -> (
        ethers::prelude::U256,
        ethers::prelude::U256,
        ethers::prelude::U256,
        ethers::prelude::U256,
        ethers::prelude::U256,
        ethers::prelude::U256,
    ) {
        let address = parse_address(address);
        return self
            .pool
            .get_user_account_data(address)
            .call()
            .await
            .unwrap();
    }
}
