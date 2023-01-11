use std::{num::NonZeroUsize, sync::Arc};

use ethers::{
    providers::{Middleware, PubsubClient},
    types::{Transaction, H256, U256},
};
use futures_util::StreamExt;
use lru::LruCache;
use tokio::sync::RwLock;

pub struct TxPool<M> {
    provider: Arc<M>,
    lru_cache: RwLock<LruCache<H256, Transaction>>, // tx hash -> gas price // TODO should not use lru cache
    num_txns_received: RwLock<usize>,
}

impl<M: Middleware + Clone> TxPool<M> {
    pub fn init(provider: Arc<M>, capacity: usize) -> Self {
        TxPool {
            provider: provider.clone(),
            lru_cache: RwLock::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
            num_txns_received: RwLock::new(0),
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

    pub async fn remove_transactions(&self, txn_hashes: Vec<H256>) -> usize {
        let mut num_removed: usize = 0;
        let mut lru_cache = self.lru_cache.write().await;
        for txn_hash in txn_hashes {
            match lru_cache.pop(&txn_hash) {
                Some(_) => {
                    num_removed += 1;
                }
                _ => {}
            };
        }
        return num_removed;
    }

    pub async fn reset_count(&self) {
        let mut val = self.num_txns_received.write().await;
        *val = 0;
    }

    pub async fn get_count(&self) -> usize {
        return *self.num_txns_received.read().await;
    }

    pub async fn stream_mempool(self: Arc<TxPool<M>>)
    where
        <M as Middleware>::Provider: PubsubClient,
    {
        let mut pending_tx_stream = self
            .provider
            .subscribe_pending_txs()
            .await
            .unwrap()
            .transactions_unordered(16); // TODO: what n is ideal?

        while let Some(Ok(pending_txn)) = pending_tx_stream.next().await {
            self.lru_cache
                .write()
                .await
                .push(pending_txn.hash, pending_txn);
            let mut val = self.num_txns_received.write().await;
            *val += 1;
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
