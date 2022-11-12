use std::sync::Arc;

use ethers::{
    providers::{Middleware, Provider, PubsubClient, SubscriptionStream, Ws},
    types::{Address, Log, H256},
    utils::{self, keccak256},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EthSubscribeLogArgs {
    pub address: Vec<Address>,
    pub topics: Vec<H256>,
}

impl EthSubscribeLogArgs {
    pub fn new(addresses: Vec<Address>, topics: Vec<H256>) -> Self {
        Self {
            address: addresses,
            topics: topics,
        }
    }
}

// https://geth.ethereum.org/docs/rpc/pubsub
pub async fn get_pair_sync_stream<P: PubsubClient>(
    provider: &Provider<P>,
    pair_addresses: Vec<Address>,
) -> SubscriptionStream<P, Log> {
    let command = "logs";
    let command = utils::serialize(&command);

    let event_name = "Sync(uint112,uint112)";
    let topic = H256::from(keccak256(event_name.as_bytes()));
    let topics = vec![topic];

    let args = EthSubscribeLogArgs::new(pair_addresses, topics);
    let args = utils::serialize(&args);

    let stream = provider.subscribe::<_, Log>([command, args]).await.unwrap();
    return stream;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ethers::abi::parse_abi;
    use ethers::contract::EthLogDecode;
    use ethers::prelude::BaseContract;
    use ethers::types::H256;
    use ethers::{
        abi::AbiDecode,
        providers::{Provider, Ws},
        types::{Address, Bytes, U256},
    };
    use futures_util::StreamExt;

    use super::get_pair_sync_stream;

    #[test]
    fn test_pair_sync_parsing() {
        // let b = Bytes(0x00000000000000000000000000000000000000000000000000000115f9862b59000000000000000000000000000000000000000000000028f28f6b108a83ce90);
        let abi = BaseContract::from(
            parse_abi(&["event Sync(uint112 reserve0, uint112 reserve1)"]).unwrap(),
        );
        let topics = vec![
            "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"
                .parse::<H256>()
                .unwrap(),
        ];
        let data = Bytes::from_str("0x00000000000000000000000000000000000000000000000000000115f9862b59000000000000000000000000000000000000000000000028f28f6b108a83ce90").unwrap();
        let (reserve0, reserve1): (U256, U256) = abi.decode_event("Sync", topics, data).unwrap();
        println!("{:?}, {:?}", reserve0, reserve1);
    }

    #[tokio::test]
    async fn test_pair_sync_stream() {
        dotenv::dotenv().ok();
        let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL").unwrap();

        let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await.unwrap();

        let mut stream = get_pair_sync_stream(
            &provider_ws,
            vec!["0x34965ba0ac2451a34a0471f04cca3f990b8dea27"
                .parse::<Address>()
                .unwrap()],
        )
        .await;

        while let Some(log) = stream.next().await {
            println!(
                "block: {:?}, tx: {:?}, pair address: {:?}, log: {:?}",
                log.block_number,
                log.transaction_hash,
                log.address,
                2 // log.address,
                  // Address::from(log.topics[1]),
                  // Address::from(log.topics[2]),
                  // U256::decode(log.data)
            );
        }
    }
}
