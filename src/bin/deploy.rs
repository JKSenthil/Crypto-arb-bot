use std::{convert::TryFrom, sync::Arc};

use dotenv::dotenv;
use ethers::{
    prelude::{abigen, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::Address,
};

abigen!(Liquidations, "abis/Liquidations.json");
abigen!(Flashloan, "abis/FlashloanV2.json");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let rpc_node_url = std::env::var("ALCHEMY_POLYGON_RPC_URL")?;

    let wallet = std::env::var("PRIVATE_KEY")?
        .parse::<LocalWallet>()?
        .with_chain_id(137_u64);
    let provider = Provider::<Http>::try_from(rpc_node_url)?;
    let provider = Arc::new(provider);

    // 4. instantiate the client with the wallet
    let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));

    let gas_price = provider.get_gas_price().await?;
    let deploy_txn = Flashloan::deploy(
        client,
        "0xBA12222222228d8Ba445958a75a0704d566BF2C8"
            .parse::<Address>()
            .unwrap(),
    )
    .unwrap();

    deploy_txn.gas_price(gas_price).send().await?;

    Ok(())
}
