use std::num::NonZeroUsize;

use ethers::types::H256;
use lru::LruCache;

use super::{block::Block, transaction::TypedTransaction};

pub struct BlockOracle {
    prev_blocks: LruCache<H256, Block>,
}

impl BlockOracle {
    pub fn new(capacity: usize) -> Self {
        Self {
            prev_blocks: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    pub fn append_block(&mut self, block: Block) {
        self.prev_blocks.push(block.header.hash(), block);
    }

    pub fn predict_next_block(&self, mempool_txns: Vec<TypedTransaction>) -> Vec<TypedTransaction> {
        // TODO: come up with strategy to predict next block
        return mempool_txns;
    }

    pub fn compare_accuracy(
        &self,
        pred_txns: Vec<TypedTransaction>,
        actual_txns: Vec<TypedTransaction>,
    ) -> f32 {
        // TODO return accuracy, can be used to debug block prediction accuracy
        0.0
    }
}
