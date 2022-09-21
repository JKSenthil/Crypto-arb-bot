use ethers::types::Address;

pub fn parse_address(addr: &str) -> Address {
    let addr = addr.strip_prefix("0x").unwrap_or(addr);
    return addr.parse().unwrap();
}
