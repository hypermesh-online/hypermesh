// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title MockAMM
 * @dev Mock Automated Market Maker for testing purposes
 * Implements basic constant product AMM (x * y = k)
 */
contract MockAMM {
    using SafeERC20 for IERC20;
    
    IERC20 public tokenA;
    IERC20 public tokenB;
    
    uint256 public reserveA;
    uint256 public reserveB;
    uint256 public constant FEE_BASIS_POINTS = 30; // 0.3% fee
    
    event LiquidityAdded(uint256 amountA, uint256 amountB);
    event LiquidityRemoved(uint256 amountA, uint256 amountB);
    event Swap(address indexed user, address tokenIn, uint256 amountIn, uint256 amountOut);
    
    constructor(address _tokenA, address _tokenB) {
        tokenA = IERC20(_tokenA);
        tokenB = IERC20(_tokenB);
    }
    
    function addLiquidity(uint256 amountA, uint256 amountB) external {
        tokenA.safeTransferFrom(msg.sender, address(this), amountA);
        tokenB.safeTransferFrom(msg.sender, address(this), amountB);
        
        reserveA += amountA;
        reserveB += amountB;
        
        emit LiquidityAdded(amountA, amountB);
    }
    
    function removeLiquidity(uint256 amountA, uint256 amountB) external {
        require(reserveA >= amountA, "Insufficient reserve A");
        require(reserveB >= amountB, "Insufficient reserve B");
        
        reserveA -= amountA;
        reserveB -= amountB;
        
        tokenA.safeTransfer(msg.sender, amountA);
        tokenB.safeTransfer(msg.sender, amountB);
        
        emit LiquidityRemoved(amountA, amountB);
    }
    
    function swapAForB(uint256 amountIn) external returns (uint256 amountOut) {
        require(amountIn > 0, "Amount must be > 0");
        require(reserveA > 0 && reserveB > 0, "Insufficient liquidity");
        
        uint256 amountInWithFee = (amountIn * (10000 - FEE_BASIS_POINTS)) / 10000;
        amountOut = (amountInWithFee * reserveB) / (reserveA + amountInWithFee);
        
        require(amountOut < reserveB, "Insufficient output reserve");
        
        tokenA.safeTransferFrom(msg.sender, address(this), amountIn);
        tokenB.safeTransfer(msg.sender, amountOut);
        
        reserveA += amountIn;
        reserveB -= amountOut;
        
        emit Swap(msg.sender, address(tokenA), amountIn, amountOut);
    }
    
    function swapBForA(uint256 amountIn) external returns (uint256 amountOut) {
        require(amountIn > 0, "Amount must be > 0");
        require(reserveA > 0 && reserveB > 0, "Insufficient liquidity");
        
        uint256 amountInWithFee = (amountIn * (10000 - FEE_BASIS_POINTS)) / 10000;
        amountOut = (amountInWithFee * reserveA) / (reserveB + amountInWithFee);
        
        require(amountOut < reserveA, "Insufficient output reserve");
        
        tokenB.safeTransferFrom(msg.sender, address(this), amountIn);
        tokenA.safeTransfer(msg.sender, amountOut);
        
        reserveB += amountIn;
        reserveA -= amountOut;
        
        emit Swap(msg.sender, address(tokenB), amountIn, amountOut);
    }
    
    function getReserves() external view returns (uint256, uint256) {
        return (reserveA, reserveB);
    }
    
    function getAmountOut(uint256 amountIn, address tokenIn) external view returns (uint256) {
        require(amountIn > 0, "Amount must be > 0");
        require(reserveA > 0 && reserveB > 0, "Insufficient liquidity");
        
        uint256 amountInWithFee = (amountIn * (10000 - FEE_BASIS_POINTS)) / 10000;
        
        if (tokenIn == address(tokenA)) {
            return (amountInWithFee * reserveB) / (reserveA + amountInWithFee);
        } else if (tokenIn == address(tokenB)) {
            return (amountInWithFee * reserveA) / (reserveB + amountInWithFee);
        }
        
        revert("Invalid token");
    }
    
    function setReserves(uint256 _reserveA, uint256 _reserveB) external {
        reserveA = _reserveA;
        reserveB = _reserveB;
    }
}
