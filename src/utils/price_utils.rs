use ethers::types::U256;

use crate::constants::token::ERC20Token;

pub fn amount_to_U256(amount: f64, decimal: i32, token: ERC20Token) -> U256 {
    let amount: u64 = (amount * 10.0_f64.powi(decimal)) as u64;
    U256::from(amount) * U256::exp10((token.get_decimals() - (decimal as u8)) as usize)
}

#[cfg(test)]
mod tests {
    use ethers::types::U256;

    use super::amount_to_U256;
    use crate::constants::token::ERC20Token::*;

    #[test]
    fn test_basic() {
        let amount = amount_to_U256(0.01, 2, USDC);
        assert_eq!(U256::from(10000), amount);

        let amount = amount_to_U256(0.001, 3, USDC);
        assert_eq!(U256::from(1000), amount);
    }
}
