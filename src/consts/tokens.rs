#[derive(Clone, Copy)]
pub enum ERC20Token {
    USDC,
    USDT,
    DAI,
    WBTC,
    WMATIC,
    WETH,
}

struct UniswapV3Fee {}

struct ERC20TokenData {
    symbol: &'static str,
    name: &'static str,
    decimals: usize,
    addr: &'static str,
}

impl ERC20Token {
    const TOKEN_INFO: &'static [&'static ERC20TokenData] = &[
        &ERC20TokenData {
            symbol: "USDC",
            name: "USD Coin",
            decimals: 6,
            addr: "0x2791bca1f2de4661ed88a30c99a7a9449aa84174",
        },
        &ERC20TokenData {
            symbol: "USDT",
            name: "Tether USD",
            decimals: 6,
            addr: "0xc2132d05d31c914a87c6611c10748aeb04b58e8f",
        },
        &ERC20TokenData {
            symbol: "DAI",
            name: "Dai Stablecoin",
            decimals: 18,
            addr: "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063",
        },
        &ERC20TokenData {
            symbol: "WBTC",
            name: "Wrapped BTC",
            decimals: 8,
            addr: "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6",
        },
        &ERC20TokenData {
            symbol: "WMATIC",
            name: "Wrapped Matic",
            decimals: 18,
            addr: "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270",
        },
        &ERC20TokenData {
            symbol: "WETH",
            name: "Wrapped Ether",
            decimals: 18,
            addr: "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619",
        },
    ];

    pub fn get_token_addr(self) -> &'static str {
        return ERC20Token::TOKEN_INFO[self as usize].addr;
    }

    pub fn get_token_decimal(self) -> usize {
        return ERC20Token::TOKEN_INFO[self as usize].decimals;
    }
}
