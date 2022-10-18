use std::{
    convert::TryFrom,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use dotenv::dotenv;
use ethers::{
    abi::{parse_abi, Token},
    prelude::{abigen, builders::ContractCall, BaseContract, SignerMiddleware},
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, Chain, TransactionReceipt},
    utils::Anvil,
};

const QUICKSWAP: &str = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff";

abigen!(Liquidations, "abis/Liquidations.json");

const WETH: &str = "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619";
const USDT: &str = "0xc2132d05d31c914a87c6611c10748aeb04b58e8f";
const DAI: &str = "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063";
const WBTC: &str = "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6";
const WMATIC: &str = "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270";
const USDC: &str = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174";
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

fn parse_args(contract: &BaseContract, input: &str) -> Vec<Token> {
    let bytes = Bytes::from_str(input).unwrap();
    let args = contract.decode_raw("liquidationCall", bytes).unwrap();
    return args;
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let base_contract = BaseContract::from(
        parse_abi(&[
            "function liquidationCall(address collateral, address debt, address user, uint256 debtToCover, bool receiveAToken)",
        ])?
    );

    let input = "0x00a718a90000000000000000000000002791bca1f2de4661ed88a30c99a7a9449aa84174000000000000000000000000c2132d05d31c914a87c6611c10748aeb04b58e8f00000000000000000000000007bcdbb839d9f64f9009d8c44cf2a2ec38116ab6000000000000000000000000000000000000000000000000000000000459cc990000000000000000000000000000000000000000000000000000000000000000";

    let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL")?;

    // let anvil = Anvil::new().fork(rpc_node_url).spawn();
    let wallet = std::env::var("PRIVATE_KEY")?
        .parse::<LocalWallet>()?
        .with_chain_id(Chain::AnvilHardhat);

    // anvil
    // let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    let provider = Provider::<Http>::try_from(rpc_node_url)?;

    // 4. instantiate the client with the wallet
    let client = Arc::new(SignerMiddleware::new(
        provider,
        wallet.with_chain_id(137_u64),
    ));

    let liquidations_contract = Liquidations::new(
        "0x5D03B3678c120F3EcC04eb96dAAb6e15B012022e".parse::<Address>()?,
        client,
    );

    let args = parse_args(&base_contract, input);
    let mut args = args.into_iter();

    println!("Size: {}", args.len());

    let collateral = args.next().unwrap().into_address().unwrap();
    let debt = args.next().unwrap().into_address().unwrap();
    let user = args.next().unwrap().into_address().unwrap();
    let debtToCover = args.next().unwrap().into_uint().unwrap();

    let dodoPool = get_dodo_pool(debt).unwrap();
    let uniswap_router = QUICKSWAP.parse::<Address>().unwrap();

    let contract_call = liquidations_contract.liquidation(
        dodoPool,
        uniswap_router,
        collateral,
        debt,
        user,
        debtToCover,
    );

    println!("oof");

    let now = Instant::now();
    let estimated_gas = contract_call.estimate_gas().await?;
    println!(
        "Estimated gas: {}, time taken: {}",
        estimated_gas,
        now.elapsed().as_millis(),
    );

    // print_type_of(&receipt);
    // .send()
    // .await?
    // .await
    // .unwrap();

    // match receipt {
    //     Ok(_) => println!("success!"),
    //     Err(e) => println!("{}", e),
    // }

    Ok(())
}
