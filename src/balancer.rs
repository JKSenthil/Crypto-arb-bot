use std::sync::Arc;

use ethers::{
    prelude::abigen,
    providers::Middleware,
    types::{Address, U256},
};

abigen!(Vault, "abis/balancer/Vault.json");

pub struct Balancer<M> {
    // provider: Arc<M>,
    vault_contract: Vault<M>,
}

impl<M: Middleware + Clone> Balancer<M> {
    pub fn new(provider: Arc<M>) -> Self {
        let vault_address = "0xBA12222222228d8Ba445958a75a0704d566BF2C8"
            .parse::<Address>()
            .unwrap();

        Self {
            vault_contract: Vault::new(vault_address, provider.clone()),
        }
    }

    pub fn query_batch_swap(self) -> U256 {
        // let a = BatchSwapStep { pool_id: todo!(), asset_in_index: todo!(), asset_out_index: todo!(), amount: todo!(), user_data: todo!() };
        // self.vault_contract.query_batch_swap(0, swaps, assets, funds)
        U256::zero()
    }
}
