use ethers::types::Address;

pub struct ERC20Token {
    pub address: Address,
    pub name: &'static str,
    pub symbol: &'static str,
    pub decimals: u8,
}

pub static USDC: ERC20Token = ERC20Token {
    address: "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
        .parse::<Address>()
        .unwrap(),
    name: "USD Coin",
    symbol: "USDC",
    decimals: 6,
};

pub static USDT: ERC20Token = ERC20Token {
    address: "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
        .parse::<Address>()
        .unwrap(),
    name: "Tether USD",
    symbol: "USDT",
    decimals: 6,
};

pub static DAI: ERC20Token = ERC20Token {
    address: "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
        .parse::<Address>()
        .unwrap(),
    name: "Dai Stablecoin",
    symbol: "DAI",
    decimals: 18,
};

pub static WBTC: ERC20Token = ERC20Token {
    address: "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6"
        .parse::<Address>()
        .unwrap(),
    name: "Wrapped BTC",
    symbol: "WBTC",
    decimals: 8,
};

pub static WMATIC: ERC20Token = ERC20Token {
    address: "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"
        .parse::<Address>()
        .unwrap(),
    name: "Wrapped Matic",
    symbol: "WMATIC",
    decimals: 18,
};

pub static WETH: ERC20Token = ERC20Token {
    address: "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
        .parse::<Address>()
        .unwrap(),
    name: "Wrapped Ether",
    symbol: "WETH",
    decimals: 18,
};
