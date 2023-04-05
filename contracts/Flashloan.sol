// SPDX-License-Identifier: MIT
pragma solidity ^0.8.12;

import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

import "./interfaces/uniswap/ISwapRouter.sol";
import "./interfaces/uniswap/IUniswapV2Router.sol";
import "./interfaces/balancer/IBalancerVault.sol";
import "./interfaces/balancer/IFlashLoanRecipient.sol";

import "hardhat/console.sol";

struct ArbParams {
    uint256 amountIn;
    address[] tokenPath;
    address[] protocolPath;
    uint8[] protocolTypes; // 0 is uniswapv2, 1 is uniswapv3
    uint24[] fees;
}

contract Flashloan is Ownable, IFlashLoanRecipientBalancer {
    using SafeERC20 for IERC20;

    IBalancerVault private immutable vault;

    constructor(address _vault) {
        vault = IBalancerVault(_vault);
	}

    function executeArbitrage(ArbParams memory params, uint blockNumber) external onlyOwner{
        require(block.number <= blockNumber, "b");
        bytes memory data = abi.encode(params);

        // create params to pass into vault flashloan call
        IERC20[] memory tokens = new IERC20[](1);
        uint256[] memory amounts = new uint256[](1);
        tokens[0] = IERC20(params.tokenPath[0]);
        amounts[0] = params.amountIn;

        vault.flashLoan(
            IFlashLoanRecipientBalancer(address(this)),
            tokens,
            amounts,
            data
        );
    }

    function receiveFlashLoan(
        IERC20[] memory tokens,
        uint256[] memory amounts, 
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external override {
        ArbParams memory decoded = abi.decode(userData, (ArbParams));

        // ARB LOGIC HERE
        uint256 currentAmount = decoded.amountIn;
        uint len = decoded.protocolPath.length;
        address[] memory path = new address[](2);
        for (uint i; i < len; ++i) {
            path[0] = decoded.tokenPath[i];
            path[1] = decoded.tokenPath[i + 1];

            uint8 protocolType = decoded.protocolTypes[i];
            if (protocolType == 0) {
                // uniswapv2 gang
                currentAmount = uniswapV2(currentAmount, decoded.protocolPath[i], path);
            } else if (protocolType == 1) {
                // uniswapv3 gang
                currentAmount = uniswapV3(currentAmount, decoded.protocolPath[i], decoded.fees[i], path);
            }
        }

        require(currentAmount > decoded.amountIn, "a");

        IERC20 loanToken = tokens[0];
        uint256 loanAmount = amounts[0];

        // Send profits to owner
        loanToken.transfer(owner(), currentAmount - loanAmount);

        // Return funds
        loanToken.transfer(address(vault), loanAmount);
    }

    function uniswapV2(
        uint256 amountIn,
        address router,
        address[] memory path
    ) internal returns (uint256 amountOut) {
        approveToken(path[0], router, amountIn);
        return IUniswapV2Router(router).swapExactTokensForTokens(
            amountIn,
            1,
            path,
            address(this),
            block.timestamp
        )[1];
    }

    function uniswapV3(
        uint256 amountIn,
        address router,
        uint24 fee,
        address[] memory path
    ) internal returns (uint256 amountOut) {
        ISwapRouter swapRouter = ISwapRouter(router);
        approveToken(path[0], address(swapRouter), amountIn);

        // single swaps
        amountOut = swapRouter.exactInputSingle(
            ISwapRouter.ExactInputSingleParams({
                tokenIn: path[0],
                tokenOut: path[1],
                fee: fee,
                recipient: address(this),
                deadline: block.timestamp,
                amountIn: amountIn,
                amountOutMinimum: 0,
                sqrtPriceLimitX96: 0
            })
        );
    }

    function approveToken(
        address token,
        address to,
        uint256 amountIn
    ) internal {
        require(IERC20(token).approve(to, amountIn), "c");
    }
}