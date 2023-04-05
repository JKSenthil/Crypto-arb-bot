// SPDX-License-Identifier: MIT
pragma solidity ^0.8.9;

import "@openzeppelin/contracts/access/Ownable.sol";

import "./interfaces/IDodo.sol";
import "./interfaces/IPool.sol";
import "./interfaces/IUniswapV2Router.sol";

contract Liquidations is Ownable {
    event Log(uint256 indexed profit);

    address public immutable POOL;

    // takes in aave pool address
    constructor(address pool) {
        POOL = pool;
    }

    function liquidation(
        address flashLoanPool,
        address uniswapRouter, //You will make a flashloan from this DODOV2 pool
        address collateralAsset,
        address debtAsset,
        address user,
        uint256 totalDebtBase
    ) external onlyOwner {
        //Note: The data can be structured with any variables required by your logic. The following code is just an example
        bytes memory data = abi.encode(flashLoanPool, uniswapRouter, collateralAsset, debtAsset, user, totalDebtBase);
        address flashLoanBase = IDODO(flashLoanPool)._BASE_TOKEN_();
        if(flashLoanBase == debtAsset) {
            IDODO(flashLoanPool).flashLoan(totalDebtBase, 0, address(this), data);
        } else {
            IDODO(flashLoanPool).flashLoan(0, totalDebtBase, address(this), data);
        }
    }

    //Note: CallBack function executed by DODOV2(DVM) flashLoan pool
    function DVMFlashLoanCall(address sender, uint256 baseAmount, uint256 quoteAmount,bytes calldata data) external {
        _flashLoanCallBack(sender,baseAmount,quoteAmount,data);
    }

    //Note: CallBack function executed by DODOV2(DPP) flashLoan pool
    function DPPFlashLoanCall(address sender, uint256 baseAmount, uint256 quoteAmount, bytes calldata data) external {
        _flashLoanCallBack(sender,baseAmount,quoteAmount,data);
    }

    //Note: CallBack function executed by DODOV2(DSP) flashLoan pool
    function DSPFlashLoanCall(address sender, uint256 baseAmount, uint256 quoteAmount, bytes calldata data) external {
        _flashLoanCallBack(sender,baseAmount,quoteAmount,data);
    }

    function _flashLoanCallBack(address sender, uint256, uint256, bytes calldata data) internal {
        (address flashLoanPool, address uniswapRouter, address collateralAsset, address debtAsset, address user, uint256 totalDebtBase) = abi.decode(data, (address, address, address, address, address, uint256));
        require(sender == address(this) && msg.sender == flashLoanPool, "HANDLE_FLASH_DENIED");

        // TODO: Realize your own logic using the token from flashLoan pool.

        // 1) call aave liquidation
        uint256 collateralAmountOut = _liquidation(collateralAsset, debtAsset, user, totalDebtBase);
        // 2) call any uniswap router to convert back to debtAsset
        _uniswapV2(uniswapRouter, collateralAmountOut, collateralAsset, debtAsset);
        // 3) send profit to wallet
        uint256 balance = IERC20(debtAsset).balanceOf(address(this));
        emit Log(balance - totalDebtBase);
        IERC20(debtAsset).transfer(owner(), balance - totalDebtBase);

        //Return funds
        IERC20(debtAsset).transfer(flashLoanPool, totalDebtBase);
    }

    function _liquidation(
        address collateralAsset,
        address debtAsset,
        address user,
        uint256 totalDebtBase // debtToCover
    ) internal returns (uint256) {
        require(IERC20(debtAsset).approve(POOL, totalDebtBase));
        IPool(POOL).liquidationCall(
            collateralAsset,
            debtAsset,
            user,
            totalDebtBase,
            false
        );
        uint256 amountOut = IERC20(collateralAsset).balanceOf(address(this));
        return amountOut;
    }

    function _uniswapV2(
        address router,
        uint256 amountIn,
        address collateralAsset,
        address debtAsset 
    ) internal returns (uint256 amountOut) {
        require(IERC20(collateralAsset).approve(router, amountIn));
        address[] memory path = new address[](2);
        path[0] = collateralAsset;
        path[1] = debtAsset;
        return
            IUniswapV2Router(router).swapExactTokensForTokens(
                amountIn,
                1,
                path,
                address(this),
                block.timestamp
            )[1];
    }
}