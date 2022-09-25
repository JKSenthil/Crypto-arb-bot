use ethers::types::{Address, U256};

pub fn parse_address(addr: &str) -> Address {
    let addr = addr.strip_prefix("0x").unwrap_or(addr);
    addr.parse().unwrap()
}

pub fn convert_to_U256(input: u32, decimal: usize) -> U256 {
    // TODO use safe multiply?
    U256::from(input) * U256::exp10(decimal)
}
