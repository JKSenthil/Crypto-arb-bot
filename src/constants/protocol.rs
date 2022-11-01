use enum_map::{enum_map, Enum, EnumMap};
use ethers::types::Address;
use lazy_static::lazy_static;

#[derive(Debug, Enum, Clone, Copy)]
pub enum UniswapV2 {
    SUSHISWAP,
    QUICKSWAP,
    POLYCAT,
    APESWAP,
    // MESHSWAP,
}

struct UniswapV2Data {
    pub name: &'static str,
    pub router_address: Address,
    pub factory_address: Address,
}

pub struct UniswapV3Data {
    pub name: &'static str,
    pub router_address: Address,
    pub factory_address: Address,
}

pub static UNISWAPV2_PROTOCOLS: [UniswapV2; 4] = [
    UniswapV2::SUSHISWAP,
    UniswapV2::QUICKSWAP,
    UniswapV2::POLYCAT,
    UniswapV2::APESWAP,
    // UniswapV2::MESHSWAP,
];

lazy_static! {
    static ref PROTOCOL_MAPPING: EnumMap<UniswapV2, UniswapV2Data> = enum_map! {
        UniswapV2::SUSHISWAP => UniswapV2Data {
            name: "Sushiswap",
            router_address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"
                .parse::<Address>()
                .unwrap(),
            factory_address: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse::<Address>().unwrap()
        },
        UniswapV2::QUICKSWAP => UniswapV2Data {
            name: "Quickswap",
            router_address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"
                .parse::<Address>()
                .unwrap(),
            factory_address: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".parse::<Address>().unwrap()
        },
        UniswapV2::POLYCAT => UniswapV2Data {
            name: "Polycat",
            router_address: "0x94930a328162957FF1dd48900aF67B5439336cBD"
                .parse::<Address>()
                .unwrap(),
            factory_address: "0x477Ce834Ae6b7aB003cCe4BC4d8697763FF456FA".parse::<Address>().unwrap(),
        },
        UniswapV2::APESWAP => UniswapV2Data {
            name: "Apeswap",
            router_address: "0xC0788A3aD43d79aa53B09c2EaCc313A787d1d607"
                .parse::<Address>()
                .unwrap(),
            factory_address: "0xCf083Be4164828f00cAE704EC15a36D711491284".parse::<Address>().unwrap(),
        },
        // UniswapV2::MESHSWAP => UniswapV2Data {
        //     name: "Meshwwap",
        //     router_address: "0x10f4a785f458bc144e3706575924889954946639"
        //         .parse::<Address>()
        //         .unwrap(),
        //     factory_address: "0x9f3044f7f9fc8bc9ed615d54845b4577b833282d".parse::<Address>().unwrap()
        // },
    };
    pub static ref UNISWAP_V3: UniswapV3Data = UniswapV3Data {
        name: "UniswapV3",
        router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564"
            .parse::<Address>()
            .unwrap(),
        factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984"
            .parse::<Address>()
            .unwrap()
    };
}

impl UniswapV2 {
    pub fn get_name(&self) -> &str {
        PROTOCOL_MAPPING[*self].name
    }

    pub fn get_router_address(&self) -> Address {
        PROTOCOL_MAPPING[*self].router_address
    }

    pub fn get_factory_address(&self) -> Address {
        PROTOCOL_MAPPING[*self].factory_address
    }

    pub fn get_all_protoccols() -> Vec<UniswapV2> {
        // keep in enum order!
        vec![
            UniswapV2::SUSHISWAP,
            UniswapV2::QUICKSWAP,
            UniswapV2::POLYCAT,
            UniswapV2::APESWAP,
            // UniswapV2::MESHSWAP,
        ]
    }
}
