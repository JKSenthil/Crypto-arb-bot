use enum_map::{enum_map, Enum, EnumMap};
use ethers::types::Address;
use lazy_static::lazy_static;

#[derive(Debug, Enum, Clone, Copy)]
pub enum ERC20Token {
    USDC,
    USDT,
    DAI,
    WBTC,
    WMATIC,
    WETH,
}

struct ERC20TokenData {
    pub address: Address,
    pub name: &'static str,
    pub symbol: &'static str,
    pub decimals: u8,
}

lazy_static! {
    static ref ERC20_MAPPING: EnumMap<ERC20Token, ERC20TokenData> = enum_map! {
        ERC20Token::USDC => ERC20TokenData {
            address: "0x2791bca1f2de4661ed88a30c99a7a9449aa84174"
                .parse::<Address>()
                .unwrap(),
            name: "USD Coin",
            symbol: "USDC",
            decimals: 6,
        },
        ERC20Token::USDT => ERC20TokenData {
            address: "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
                .parse::<Address>()
                .unwrap(),
            name: "Tether USD",
            symbol: "USDT",
            decimals: 6,
        },
        ERC20Token::DAI => ERC20TokenData {
            address: "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
                .parse::<Address>()
                .unwrap(),
            name: "Dai Stablecoin",
            symbol: "DAI",
            decimals: 18,
        },
        ERC20Token::WBTC => ERC20TokenData {
            address: "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6"
                .parse::<Address>()
                .unwrap(),
            name: "Wrapped BTC",
            symbol: "WBTC",
            decimals: 8,
        },
        ERC20Token::WMATIC => ERC20TokenData {
            address: "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"
                .parse::<Address>()
                .unwrap(),
            name: "Wrapped Matic",
            symbol: "WMATIC",
            decimals: 18,
        },
        ERC20Token::WETH => ERC20TokenData {
            address: "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"
                .parse::<Address>()
                .unwrap(),
            name: "Wrapped Ether",
            symbol: "WETH",
            decimals: 18,
        }
    };
}

impl ERC20Token {
    pub fn get_address(self) -> Address {
        ERC20_MAPPING[self].address
    }

    pub fn get_name(self) -> &'static str {
        ERC20_MAPPING[self].name
    }

    pub fn get_symbol(self) -> &'static str {
        ERC20_MAPPING[self].symbol
    }

    pub fn get_decimals(self) -> u8 {
        ERC20_MAPPING[self].decimals
    }
}
