#[derive(Clone, Copy)]
pub enum Protocol {
    SUSHISWAP,
    QUICKSWAP,
    JETSWAP,
    POLYCAT,
    APESWAP,
    UNISWAP_V3,
}

impl Protocol {
    const ROUTER_ADDRESSES: &'static [&'static str] = &[
        "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
        "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
        "0x5C6EC38fb0e2609672BDf628B1fD605A523E5923",
        "0x94930a328162957FF1dd48900aF67B5439336cBD",
        "0xC0788A3aD43d79aa53B09c2EaCc313A787d1d607",
        "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6",
    ];

    pub fn get_router_address(self) -> &'static str {
        return Protocol::ROUTER_ADDRESSES[self as usize];
    }

    pub fn uniswap_v2_protocols() -> [Protocol; 5] {
        return [
            Protocol::SUSHISWAP,
            Protocol::QUICKSWAP,
            Protocol::JETSWAP,
            Protocol::POLYCAT,
            Protocol::APESWAP,
        ];
    }

    pub fn is_uniswapV2_protocol(self) -> bool {
        return !(self as usize > 4);
    }
}

#[derive(Clone, Copy)]
pub enum ERC20Token {
    USDC,
    USDT,
    DAI,
    WBTC,
    WMATIC,
    WETH,
}

struct ERC20TokenData {
    symbol: &'static str,
    name: &'static str,
    decimals: u8,
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

    pub fn get_token_decimal(self) -> u8 {
        return ERC20Token::TOKEN_INFO[self as usize].decimals;
    }

    pub fn decimals_as_str(self) -> &'static str {
        match ERC20Token::TOKEN_INFO[self as usize].decimals {
            6 => "000000",
            8 => "00000000",
            _ => "000000000000000000",
        }
    }
}
