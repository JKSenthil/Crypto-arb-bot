// This is a file copied from https://github.com/DODOEX/dodo-example/blob/main/solidity/contracts/DODOFlashloan.sol
/*
    Copyright 2021 DODO ZOO.
    SPDX-License-Identifier: Apache-2.0
*/
pragma solidity ^0.8.9;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

interface IDODO {
    function flashLoan(
        uint256 baseAmount,
        uint256 quoteAmount,
        address assetTo,
        bytes calldata data
    ) external;

    function _BASE_TOKEN_() external view returns (address);
}