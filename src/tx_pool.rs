use std::{num::NonZeroUsize, sync::Arc};

use ethers::{
    providers::{Middleware, PubsubClient},
    types::{Block, Transaction, H256, U256, U64},
};
use futures_util::StreamExt;
use lru::LruCache;
use tokio::sync::RwLock;

pub struct TxPool<M> {
    provider: Arc<M>,
    lru_cache: RwLock<LruCache<H256, Transaction>>, // tx hash -> gas price
}

impl<M: Middleware + Clone> TxPool<M> {
    pub fn init(provider: Arc<M>, capacity: usize) -> Self {
        TxPool {
            provider: provider.clone(),
            lru_cache: RwLock::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
        }
    }

    pub async fn get_mempool(&self) -> Vec<Transaction> {
        let mut txns: Vec<Transaction> = Vec::new();
        let lru_cache = self.lru_cache.read().await;
        for (_, txn) in lru_cache.iter() {
            txns.push(txn.clone());
        }
        return txns;
    }

    async fn retrieve_all_gas_prices(&self) -> Vec<U256> {
        let lru_cache = self.lru_cache.read().await;
        let mut gas_prices = Vec::with_capacity(lru_cache.len());
        for (_, txn) in lru_cache.iter() {
            gas_prices.push(txn.gas_price.unwrap());
        }
        return gas_prices;
    }

    pub async fn get_90th_percentile_gas_price(&self) -> U256 {
        let mut gas_prices = self.retrieve_all_gas_prices().await;
        gas_prices.sort();
        let mut idx = gas_prices.len();
        idx *= 9000;
        idx /= 10000;
        if gas_prices.len() > 4 {
            idx = gas_prices.len() - 4;
        }

        return gas_prices[idx];
    }

    pub async fn stream_mempool(self: Arc<TxPool<M>>)
    where
        <M as Middleware>::Provider: PubsubClient,
    {
        let mut block_stream = self.provider.subscribe_blocks().await.unwrap().fuse();
        let mut pending_tx_stream = self
            .provider
            .subscribe_pending_txs()
            .await
            .unwrap()
            .transactions_unordered(16) // TODO: what n is ideal?
            .fuse();

        loop {
            futures_util::select! {
                block = block_stream.next() => {
                    let block: Block<H256> = block.unwrap();
                    let txns = self.provider.get_block(block.hash.unwrap()).await.unwrap().unwrap().transactions;
                    // let num_txns_in_block = txns.len();
                    let mut lru_cache = self.lru_cache.write().await;
                    // let mut count: usize = 0;
                    for tx_hash in txns {
                        lru_cache.pop(&tx_hash);
                    }
                    // println!("----------------------");
                    // println!("Mempool txn count: {:?}", lru_cache.len());
                    // println!("{}/{} hit rate in mempool", count, num_txns_in_block);
                },
                pending_tx = pending_tx_stream.next() => {
                    match pending_tx.unwrap() {
                        Ok(pending_tx) => {
                            // Move to seperate function to determine gas prices
                            // let gas_price = pending_tx.gas_price.unwrap_or(U256::zero());
                            // let max_fee_per_gas = pending_tx.max_fee_per_gas.unwrap_or(U256::zero());
                            // let fee = if gas_price > max_fee_per_gas {gas_price} else {max_fee_per_gas};
                            self.lru_cache.write().await.push(pending_tx.hash, pending_tx);
                        },
                        _ => {}
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ethers::providers::{Middleware, Provider, Ws};
    use futures_util::StreamExt;

    use super::TxPool;

    #[tokio::test]
    async fn test_mempool_stream_alchemy() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();
        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let txpool = TxPool::init(provider_ws.clone(), 1000);
        let txpool = Arc::new(txpool);
        tokio::spawn(txpool.clone().stream_mempool());

        let mut stream = provider_ws.subscribe_blocks().await.unwrap();
        while let Some(_) = stream.next().await {
            // println!("Pending txn count: {:?}", txpool.data.len());
        }
    }

    #[tokio::test]
    async fn test_mempool_stream_ipc() {
        let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc")
            .await
            .unwrap();
        let provider_ipc = Arc::new(provider_ipc);

        let txpool = TxPool::init(provider_ipc.clone(), 1000);
        let txpool = Arc::new(txpool);
        tokio::spawn(txpool.clone().stream_mempool());

        let mut stream = provider_ipc.subscribe_blocks().await.unwrap();
        while let Some(_) = stream.next().await {
            println!(
                "Pending txn count: {:?}",
                txpool.lru_cache.read().await.len()
            );
            println!(
                "90th percentile gas price: {:?}",
                txpool.get_90th_percentile_gas_price().await
            );
        }
    }
}
