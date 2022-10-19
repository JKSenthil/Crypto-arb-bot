use std::str::FromStr;
use std::sync::Arc;

use dotenv::dotenv;
use ethers::prelude::{abigen, SignerMiddleware};
use ethers::providers::{Http, SubscriptionStream};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Transaction, U256};
use ethers::utils;
use ethers::{
    abi::{parse_abi, Token},
    prelude::{BaseContract, Provider},
    providers::{Middleware, Ws},
    types::{Address, Bytes, GethDebugTracingOptions, H256},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

abigen!(Liquidations, "abis/Liquidations.json");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PendingTransactionOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_address: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_address: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashes_only: Option<bool>,
}

async fn get_args(
    provider: &Provider<Http>,
    txn_hash: H256,
    encoded_function_preface: &str,
) -> Option<String> {
    let res = provider
        .debug_trace_transaction(
            txn_hash,
            GethDebugTracingOptions {
                disable_storage: None,
                disable_stack: None,
                enable_memory: None,
                enable_return_data: None,
                tracer: Some("callTracer".to_string()),
                timeout: Some("5s".to_string()),
            },
        )
        .await
        .unwrap_err();
    let response = res.to_string();
    println!("{}", response);
    match response.find(encoded_function_preface) {
        Some(index) => {
            let str = &response[index..index + 330];
            Some(str.to_string())
        }
        None => None,
    }
}

fn parse_args(contract: &BaseContract, input: &str) -> Vec<Token> {
    let bytes = Bytes::from_str(input).unwrap();
    let args = contract.decode_raw("liquidationCall", bytes).unwrap();
    return args;
}

const WETH: &str = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619";
const USDT: &str = "0xc2132d05d31c914a87c6611c10748aeb04b58e8f";
const DAI: &str = "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063";
const WBTC: &str = "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6";
const WMATIC: &str = "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270";
const USDC: &str = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174";

const QUICKSWAP: &str = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff";

fn get_dodo_pool(token_address: Address) -> Option<Address> {
    match format!("{:?}", token_address).as_str() {
        WETH => Some(
            "0x5333Eb1E32522F1893B7C9feA3c263807A02d561"
                .parse::<Address>()
                .unwrap(),
        ),
        USDT => Some(
            "0x20B5F71DAF95c712E776Af8A3b7926fa8FDA5909"
                .parse::<Address>()
                .unwrap(),
        ),
        DAI => Some(
            "0x20B5F71DAF95c712E776Af8A3b7926fa8FDA5909"
                .parse::<Address>()
                .unwrap(),
        ),
        WBTC => Some(
            "0xe020008465cD72301A18b97d33D73bF44858A4b7"
                .parse::<Address>()
                .unwrap(),
        ),
        WMATIC => Some(
            "0xeB5CE2e035Dd9562a6d0a639A68D372eFb21D22e"
                .parse::<Address>()
                .unwrap(),
        ),
        USDC => Some(
            "0x5333Eb1E32522F1893B7C9feA3c263807A02d561"
                .parse::<Address>()
                .unwrap(),
        ),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let rpc_node_ws_url = std::env::var("ALCHEMY_POLYGON_RPC_WS_URL")?;
    let provider = Provider::<Http>::try_from(std::env::var("ALCHEMY_POLYGON_RPC_URL")?)?;
    // let provider = Arc::new(provider);
    let provider_ws = Provider::<Ws>::connect(&rpc_node_ws_url).await?;
    let provider_ws = Arc::new(provider_ws);

    let wallet = std::env::var("PRIVATE_KEY")?
        .parse::<LocalWallet>()?
        .with_chain_id(137u64);

    let client = SignerMiddleware::new(provider_ws.clone(), wallet);
    let client = Arc::new(client);

    let contract = BaseContract::from(
        parse_abi(&[
            "function liquidationCall(address collateral, address debt, address user, uint256 debtToCover, bool receiveAToken)",
        ])?
    );

    let liquidations_contract = Liquidations::new(
        "0x5D03B3678c120F3EcC04eb96dAAb6e15B012022e".parse::<Address>()?,
        client,
    );

    let encoded_prefix = "0x00a718a9";

    // TODO maybe change? this is quite a alot
    let max_gas = U256::from(20_650_000);

    // construct stream
    let known_liquidators = [
        "0x54999CBEA7ec48A373aCE8A5dDc1D6e6fF7F8202",
        "0x28d62d755D561e7468734Cd63c62ec960Cd4c1A7",
        "0x87C76A8A5d8D24250752F93BDC232B18997dDa15",
        "0x0000000eb7D8244007Da6CD63A512eC69494b231",
        "0xB8f013e063F59719D05b3F1F9076b4DC7e56FAe7",
        "0xEb7e2AeB58b55bc419BDAD48A8c39e2C6d7CEB84",
        "0x14770cD80fa8055c12BC092255496CA8D0fFCF5e",
        "0x88E2840bA66c7B618f37AEE2DD9c448997D41690",
        "0x774b407f518C91ae79250625291AA14440D5d8fB",
        "0x98648D396a35D1FF9ED354432B2C98C37931F69C",
        "0x794a61358D6845594F94dc1DB02A252b5b4814aD",
    ]
    .map(|x| x.to_string())
    .to_vec();
    let method = utils::serialize(&"alchemy_pendingTransactions");
    let method_params = utils::serialize(&PendingTransactionOptions {
        to_address: Some(known_liquidators),
        from_address: None,
        hashes_only: None,
    });
    let mut pending_txn_stream: SubscriptionStream<Ws, Box<RawValue>> =
        provider_ws.subscribe([method, method_params]).await?;

    println!("Listening to transactions");
    while let Some(item) = pending_txn_stream.next().await {
        if let Ok(txn) = serde_json::from_str::<Transaction>(item.get()) {
            println!(
                "Detected liquidation transaction with hash: {}",
                format!("{:?}", txn.hash)
            );

            if let Some(liquidation_call_args) = get_args(&provider, txn.hash, encoded_prefix).await
            {
                let args = parse_args(&contract, liquidation_call_args.as_str());
                let mut args = args.into_iter();

                let collateral = args.next().unwrap().into_address().unwrap();
                let debt = args.next().unwrap().into_address().unwrap();
                let user = args.next().unwrap().into_address().unwrap();
                let debtToCover = args.next().unwrap().into_uint().unwrap();

                let dodoPool = get_dodo_pool(debt);
                if let Some(dodoPool) = dodoPool {
                    let uniswap_router = QUICKSWAP.parse::<Address>().unwrap();

                    let gas_fee = txn.max_priority_fee_per_gas.unwrap();

                    // TODO pass args into smart contract and win $$$
                    // and don't forget to bid additional gas so that
                    // your txn is picked up before your opponents!
                    match liquidations_contract
                        .liquidation(
                            dodoPool,
                            uniswap_router,
                            collateral,
                            debt,
                            user,
                            debtToCover,
                        )
                        .gas(max_gas)
                        .gas_price(gas_fee + gas_fee) // double gas price for speedup
                        .send()
                        .await
                    {
                        Ok(pending_txn) => {
                            println!("Txn submitted: {}", pending_txn.tx_hash())
                        }
                        Err(e) => println!("Err received: {}", e),
                    }
                }
            }
        }
    }

    Ok(())
}
