use std::{collections::HashSet, num::NonZeroUsize};

use ethers::types::H256;
use lru::LruCache;

use super::{block::Block, transaction::TypedTransaction};

pub struct BlockOracle {
    prev_blocks: LruCache<H256, Block>,
    prediction: Vec<TypedTransaction>,
    actual: HashSet<H256>,
}

impl BlockOracle {
    pub fn new(capacity: usize) -> Self {
        Self {
            prev_blocks: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            prediction: Vec::new(),
            actual: HashSet::new(),
        }
    }

    pub fn append_block(&mut self, block: Block) {
        self.prev_blocks.push(block.header.hash(), block.clone());
        self.actual.clear();
        for txn in block.transactions {
            self.actual.insert(txn.hash());
        }
    }

    pub fn predict_next_block(
        &mut self,
        mempool_txns: Vec<TypedTransaction>,
    ) -> Vec<TypedTransaction> {
        self.prediction.clear();
        let mut sum = 0 as usize;
        for (_, block) in &self.prev_blocks {
            sum += block.transactions.len();
        }
        let avg = sum / self.prev_blocks.len();
        self.prediction = mempool_txns[..avg].to_vec();
        return self.prediction.clone();
    }

    pub fn display_accuracy(&self) {
        let mut hits = 0 as usize;
        for txn in &self.prediction {
            if self.actual.contains(&txn.hash()) {
                hits += 1;
            }
        }
        println!(
            "{}/{} hits in actual block, prediction size: {}",
            hits,
            self.actual.len(),
            self.prediction.len()
        );
    }
}
