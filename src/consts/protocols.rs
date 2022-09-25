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
