use enum_map::{enum_map, Enum, EnumMap};
use ethers::types::Address;
use lazy_static::lazy_static;

#[derive(Debug, Enum, Clone, Copy)]
pub enum UniswapV2 {
    SUSHISWAP,
    QUICKSWAP,
    // JETSWAP, # TODO investigate why pair contract has no getReserves function
    POLYCAT,
    APESWAP,
}

struct UniswapV2Data {
    pub name: &'static str,
    pub router_address: Address,
    pub factory_address: Address,
}

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
        // UniswapV2::JETSWAP => UniswapV2Data {
        //     name: "Jetswap",
        //     router_address: "0x5C6EC38fb0e2609672BDf628B1fD605A523E5923"
        //     .parse::<Address>()
        //     .unwrap(),
        //     factory_address: "0x668ad0ed2622C62E24f0d5ab6B6Ac1b9D2cD4AC7".parse::<Address>().unwrap(),
        // },
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
        }
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
            // UniswapV2::JETSWAP,
            UniswapV2::POLYCAT,
            UniswapV2::APESWAP,
        ]
    }
}
