// use ethers::abi::Address;
// use std::collections::HashMap;

// use crate::{
//     constants::{protocol::UniswapV2, token::ERC20Token},
//     uniswapV2::UniswapV2Pair,
//     uniswapV3::UniswapV3Client,
//     utils::matrix::Matrix3D,
// };

// pub struct WorldState<M> {
//     uniswapV2_markets: Matrix3D<UniswapV2Pair>,
//     uniswapV2_pair_lookup: HashMap<Address, (UniswapV2, ERC20Token, ERC20Token)>,
//     uniswapV3_client: UniswapV3Client<M>,
// }

// impl WorldState {
//     pub fn new(
//         token_pairs: Vec<(ERC20Token, ERC20Token)>,
//         token_pair_addresses: Vec<Address>,
//     ) -> Self {
//         WorldState {
//             uniswapV2_markets: enum_map! {},
//         }
//     }
// }
