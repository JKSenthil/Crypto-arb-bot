use enum_map::Enum;
use ethers::types::Address;

#[derive(Debug, Enum)]
pub enum Protocol {
    UniswapV2 { address: Address },
    UniswapV3 { fee: u32 },
}

pub static SUSHISWAP: Protocol = Protocol::UniswapV2 {
    address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"
        .parse::<Address>()
        .unwrap(),
};

pub static QUICKSWAP: Protocol = Protocol::UniswapV2 {
    address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"
        .parse::<Address>()
        .unwrap(),
};

pub static JETSWAP: Protocol = Protocol::UniswapV2 {
    address: "0x5C6EC38fb0e2609672BDf628B1fD605A523E5923"
        .parse::<Address>()
        .unwrap(),
};

pub static POLYCAT: Protocol = Protocol::UniswapV2 {
    address: "0x94930a328162957FF1dd48900aF67B5439336cBD"
        .parse::<Address>()
        .unwrap(),
};

pub static APESWAP: Protocol = Protocol::UniswapV2 {
    address: "0xC0788A3aD43d79aa53B09c2EaCc313A787d1d607"
        .parse::<Address>()
        .unwrap(),
};

// Solidity does not support float values, so pool fees are multiplied by 10^4
// 0.05% -> 500
// 0.3% -> 3000
pub static UNISWAP_V3_100: Protocol = Protocol::UniswapV3 { fee: 100 };
pub static UNISWAP_V3_500: Protocol = Protocol::UniswapV3 { fee: 500 };
pub static UNISWAP_V3_3000: Protocol = Protocol::UniswapV3 { fee: 3000 };
pub static UNISWAP_V3_10000: Protocol = Protocol::UniswapV3 { fee: 10000 };

impl Protocol {
    pub const V3_ADDRESS: Address = "0xE592427A0AEce92De3Edee1F18E0157C05861564"
        .parse::<Address>()
        .unwrap();
}
