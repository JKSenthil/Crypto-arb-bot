use std::collections::LinkedList;

use ethers::types::{Transaction, U256};

#[derive(PartialEq, Eq)]
pub struct TxLinkedList {
    pub linked_list: LinkedList<Transaction>,
}

impl TxLinkedList {
    pub fn new() -> Self {
        Self {
            linked_list: LinkedList::new(),
        }
    }
}

// Legacy, Dynamic, AccessList
fn get_gas_prices(item1: &TxLinkedList, item2: &TxLinkedList) -> (U256, U256) {
    let this_txn = item1.linked_list.front().unwrap();
    let other_txn = item2.linked_list.front().unwrap();
    let this_gas_price = match this_txn.max_fee_per_gas {
        Some(val) => val,
        None => this_txn.gas_price.unwrap(),
    };
    let other_gas_price = match other_txn.max_fee_per_gas {
        Some(val) => val,
        None => other_txn.gas_price.unwrap(),
    };
    return (this_gas_price, other_gas_price);
}

impl PartialOrd for TxLinkedList {
    fn lt(&self, other: &Self) -> bool {
        let (gp1, gp2) = get_gas_prices(self, other);
        gp1 < gp2
    }

    fn le(&self, other: &Self) -> bool {
        let (gp1, gp2) = get_gas_prices(self, other);
        gp1 <= gp2
    }

    fn gt(&self, other: &Self) -> bool {
        let (gp1, gp2) = get_gas_prices(self, other);
        gp1 > gp2
    }

    fn ge(&self, other: &Self) -> bool {
        let (gp1, gp2) = get_gas_prices(self, other);
        gp1 >= gp2
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (gp1, gp2) = get_gas_prices(self, other);
        Some(gp1.cmp(&gp2))
    }
}

impl Ord for TxLinkedList {
    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::max_by(self, other, Ord::cmp)
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::min_by(self, other, Ord::cmp)
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
    {
        return min;
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (gp1, gp2) = get_gas_prices(self, other);
        gp1.cmp(&gp2)
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{BinaryHeap, HashMap},
        sync::Arc,
    };

    use ethers::{
        providers::{Middleware, Provider, Ws},
        types::Address,
    };

    use super::TxLinkedList;

    #[tokio::test]
    async fn test_heap() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();
        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();
        let provider_ws = Arc::new(provider_ws);

        let block_number = provider_ws.get_block_number().await.unwrap();
        let block = provider_ws
            .get_block_with_txs(block_number)
            .await
            .unwrap()
            .unwrap();
        let txns = block.transactions;

        let mut mapping: HashMap<Address, TxLinkedList> = HashMap::new();
        for txn in txns {
            let sender_address = txn.from;
            if !mapping.contains_key(&sender_address) {
                mapping.insert(sender_address, TxLinkedList::new());
            }
            mapping
                .get_mut(&sender_address)
                .unwrap()
                .linked_list
                .push_back(txn);
        }
        let mut heap = BinaryHeap::<TxLinkedList>::new();

        for (_, lls) in mapping {
            heap.push(lls);
        }

        while heap.len() != 0 {
            let ll = heap.pop().unwrap().linked_list;
            print!("{:?}:", ll.front().unwrap().from);
            for ele in ll {
                print!(
                    "({},{},{}),",
                    ele.gas_price.unwrap_or_default(),
                    ele.max_fee_per_gas.unwrap_or_default(),
                    ele.nonce
                );
            }
            println!("\n-----------");
        }
    }
}
