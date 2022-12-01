use std::{sync::Arc, time::Instant};

use dashmap::DashMap;
use ethers::{
    providers::{Middleware, PubsubClient},
    types::{Block, Transaction, H256, U256},
};
use futures_util::StreamExt;

pub struct TxPool<M> {
    provider: Arc<M>,
    data: DashMap<H256, U256>, // tx hash -> gas price
}

impl<M: Middleware + Clone> TxPool<M> {
    pub fn init(provider: Arc<M>) -> Self {
        TxPool {
            provider: provider.clone(),
            data: DashMap::new(),
        }
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
            .transactions_unordered(4) // what n is ideal?
            .fuse();

        let mut last_txn_hash: H256 = H256::zero();
        loop {
            futures_util::select! {
                block = block_stream.next() => {
                    let block: Block<H256> = block.unwrap();
                    let now = Instant::now();
                    let txns = self.provider.get_block_with_txs(block.hash.unwrap()).await.unwrap().unwrap().transactions;

                    println!("txn count: {:?}, time elapsed: {:?}ms", txns.len(), now.elapsed().as_millis());
                    for tx_hash in txns {
                        if let Some(_) = self.data.remove(&tx_hash.hash) {
                            println!("TXN removed!");
                        }

                    }
                    println!("Random txn hash: {:?}", last_txn_hash);
                },
                pending_tx = pending_tx_stream.next() => {
                    let pending_tx: Transaction = pending_tx.unwrap().unwrap();
                    let gas_price = pending_tx.gas_price.unwrap_or(U256::zero());
                    let max_fee_per_gas = pending_tx.max_fee_per_gas.unwrap_or(U256::zero());
                    let fee = if gas_price > max_fee_per_gas {gas_price} else {max_fee_per_gas};
                    self.data.insert(pending_tx.hash, fee);
                    last_txn_hash = pending_tx.hash;
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

        let txpool = TxPool::init(provider_ws.clone());
        let txpool = Arc::new(txpool);
        tokio::spawn(txpool.clone().stream_mempool());

        let mut stream = provider_ws.subscribe_blocks().await.unwrap();
        while let Some(_) = stream.next().await {
            println!("Pending txn count: {:?}", txpool.data.len());
        }
    }

    #[tokio::test]
    async fn test_mempool_stream_ipc() {
        let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc")
            .await
            .unwrap();
        let provider_ipc = Arc::new(provider_ipc);

        let txpool = TxPool::init(provider_ipc.clone());
        let txpool = Arc::new(txpool);
        tokio::spawn(txpool.clone().stream_mempool());

        let mut stream = provider_ipc.subscribe_blocks().await.unwrap();
        while let Some(_) = stream.next().await {
            println!("Pending txn count: {:?}", txpool.data.len());
        }
    }
}
