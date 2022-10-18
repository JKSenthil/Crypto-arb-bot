use std::collections::HashSet;
use std::fs;
use std::ops::Add;
use std::process::exit;
use std::str::FromStr;
use std::time::Instant;

use cryptorocket::price::Price;
use cryptorocket::{consts::*, utils::convert_to_U256};
use dotenv::dotenv;
use ethers::abi::{parse_abi, ParamType};
use ethers::abi::{AbiDecode, Function, Param, StateMutability};
use ethers::contract::Contract;
use ethers::prelude::BaseContract;
use ethers::providers::{Http, Middleware, Provider, Ws};
use ethers::types::{Address, Bytes, GethDebugTracingOptions, ValueOrArray, H256};
use ethers::utils::__serde_json::json;
use ethers::utils::{hex, keccak256};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{Client, RequestBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL")?;

    let provider = Provider::<Http>::try_from(rpc_node_url)?;
    let tx_hash =
        "0xf6f39213771aaa9bc399805929121643f35f13e2c76bce41154334b26b7eb609".parse::<H256>()?;
    let now = Instant::now();
    let res = provider
        .debug_trace_transaction(
            tx_hash,
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
    match response.to_string().find("0x00a718a9") {
        Some(index) => {
            let str = &response[index..index + 330];
            println!("{}", str);
        }
        None => {}
    }
    println!("Time taken: {}ms", now.elapsed().as_millis());
    // let provider = Provider::<Ws>::connect(rpc_node_url).await?;

    // let contract = BaseContract::from(
    //     parse_abi(&[
    //         "function liquidationCall(address collateral, address debt, address user, uint256 debtToCover, bool receiveAToken)",
    //     ])?
    // );

    // let input = "0x00a718a90000000000000000000000007ceb23fd6bc0add59e62ac25578270cff1b9f6190000000000000000000000008f3cf7ad23cd3cadbd9735aff958023239c6a0630000000000000000000000008a49b27e7f268f1073ce77f246e3a41ca07f8a760000000000000000000000000000000000000000000000359f8ec93190b2112a0000000000000000000000000000000000000000000000000000000000000000";
    // let bytes = Bytes::from_str(input)?;
    // let args = contract.decode_raw("liquidationCall", bytes)?;
    // for i in 0..args.len() {
    //     println!("{:?}", args[i]);
    // }

    // let function_abi = "liquidationCall(address,address,address,uint256,bool)";
    // let hash = keccak256(function_abi);
    // let value = ValueOrArray::Value(H256::from(hash));
    // println!("{:?}", value);
    // let s = "0x00a718a9";
    // let tx_hash = "0xbd83d3289d5a14e9b09d963e1184f76520f44a9c91305cac48a9f58013ada4c4";

    // let client = DebugTxClient::new(rpc_node_url.as_str());
    // let now = Instant::now();
    // let h = client.debug_trace_transaction(tx_hash, s).await;
    // println!("time elapsed {}ms", now.elapsed().as_millis());
    // if let Some(val) = h {
    //     print!("{}", val);
    // }
    // let args = format!(
    //     "
    // {{
    //     \"id\": 1,
    //     \"jsonrpc\": \"2.0\",
    //     \"method\": \"debug_traceTransaction\",
    //     \"params\": [
    //         \"{}\",
    //         {{
    //             \"tracer\": \"callTracer\",
    //             \"timeout\": \"5s\"
    //         }}
    //     ]
    // }}
    // ",
    //     tx_hash
    // );

    // println!("{}", args);
    // // let json_args = json!(args);

    // let uri = "https://polygon-mainnet.g.alchemy.com/v2/demo";
    // let client = reqwest::Client::new();
    // let resp = client
    //     .post(uri)
    //     .header(ACCEPT, "application/json")
    //     .header(CONTENT_TYPE, "application/json")
    //     .body(args)
    //     .send()
    //     .await?
    //     .text()
    //     .await?;

    // match resp.find(s) {
    //     Some(index) => println!("{:?}", &resp[index..index + 330]),
    //     None => println!("Some issue occurred, {}", resp),
    // }

    // let str = "

    // println!("{}", str);
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
    ]
    .map(|x| x.parse::<Address>().unwrap());

    let known_liquidators = HashSet::from(known_liquidators);

    if known_liquidators
        .contains(&Address::from_str("0xEb7e2AeB58b55bc419BDAD48A8c49e2C6d7CEB84").unwrap())
    {
        println!("OH YEAH");
    }
    Ok(())
}
