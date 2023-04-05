# Polygon Arbitrage Bot

## Requirements

Must have Rust installed. Solidity compiler is optional.
Create a `.env` file to setup private keys and other variables, like this:

    #Note: If using metamask, you'll have to add a 0x to the start of your private key)

    PRIVATE_KEY="aaaaaaaaaaaa..."
    ALCHEMY_POLYGON_RPC_URL="https://polygon-mainnet.g.alchemy.com/v2/your_api_key"

## Build

To build release binaries, run `cargo build --release`

## arb.rs

Checks for arbitrage opportunities across DEXs (Sushiswap, Quickswap, Polycat, Apeswap, Uniswap V3, and others). If arb present, initiates a flashloan to profit off of opportunity. For best latency, must run your own polygon node and use ipc to communicate.

Must also deploy a version of the "Flashloan.sol" contract on chain and replace the address in the code with your deployed one. You can use the deploy.rs, or do another method of your choice.

Command to run:

    ./arb --help
    Usage: arb [OPTIONS]

    Options:
      -u, --use-ipc  use ipc (if running on node)
      -h, --help     Print help information
      -V, --version  Print version information


## arb_v2.rs (in progress)

The issue with arb (v1) is that when submitting a transaction at block n, your transaction will only go through at block n + 2 at the earliest. This mean that for popular tokens, the arbitrage opportunity may not exist by the time the arb transaction goes through.

To circumvent this, we can read from the mempool of a node and predict what the n+1 block will be, and submit our transaction with this in mind. Since block n+1 transactions have not gone through yet, it is no longer feasible to use flash loans, as validators will reject this transaction.
