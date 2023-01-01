use std::sync::Arc;

use ethers::prelude::abigen;
use ethers::providers::{JsonRpcClient, Middleware, ProviderError};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{GethTrace, Transaction};
use ethers::utils::serialize;
use ethers::{providers::Provider, types::U256};
use tsuki::constants::{protocol::UniswapV2, token::ERC20Token};
use tsuki::uniswapV2::UniswapV2Client;

abigen!(
    ERC20,
    r#"[
        approve(address spender, uint256 amount) external returns (bool)
    ]"#,
);

async fn debug_trace_call<M: JsonRpcClient>(
    provider: Arc<Provider<M>>,
    typed_tx: &TypedTransaction,
) -> Result<GethTrace, ProviderError> {
    let tx = serialize(&typed_tx);
    let block = serialize(&provider.get_block_number().await.unwrap());
    provider.request("debug_traceCall", [tx, block]).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider_ipc = Provider::connect_ipc("/home/jsenthil/.bor/data/bor.ipc").await?;
    let provider_ipc = Arc::new(provider_ipc);

    let client = UniswapV2Client::new(provider_ipc.clone());
    let tx = client.get_swapExactTokensForTokens_txn(
        UniswapV2::QUICKSWAP,
        ERC20Token::USDC,
        ERC20Token::USDT,
        U256::from(1_000_000),
    );

    let token_contract = ERC20::new(ERC20Token::USDC.get_address(), provider_ipc.clone());
    let approve_tx = token_contract.approve(
        UniswapV2::SUSHISWAP.get_router_address(),
        U256::from(1_000_000),
    );

    let result = debug_trace_call(provider_ipc, &approve_tx.tx).await;
    println!("{:?}", result);

    // let mut multicall = Multicall::new(provider_ipc.clone());
    // multicall.add_call(approve_tx);
    // multicall.add_call(tx);

    // let data = multicall.call_raw().await;
    // println!("{:?}", data);

    Ok(())
}
