// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title ICaesarCoinDEXFactory
 * @dev Interface for the Caesar Token DEX Factory
 */
interface ICaesarCoinDEXFactory {
    // Events
    event PairCreated(address indexed token0, address indexed token1, address pair, uint256 allPairsLength);
    event FeeToSet(address indexed feeTo);
    event FeeToSetterSet(address indexed feeToSetter);
    event TradingFeeSet(uint256 fee);

    // Read functions
    function feeTo() external view returns (address);
    function feeToSetter() external view returns (address);
    function tradingFee() external view returns (uint256);
    function getPair(address tokenA, address tokenB) external view returns (address pair);
    function allPairs(uint256 index) external view returns (address pair);
    function allPairsLength() external view returns (uint256);
    function getPairTradingFee(address pair) external view returns (uint256);
    function getInitializingTokens() external view returns (address token0, address token1);

    // Write functions
    function createPair(address tokenA, address tokenB) external returns (address pair);
    function setFeeTo(address _feeTo) external;
    function setFeeToSetter(address _feeToSetter) external;
    function setTradingFee(uint256 _fee) external;
    function setProtocolFee(uint256 _fee) external;
    function updatePairVolume(address pair, uint256 volumeToAdd) external;
    function collectProtocolFees(address pair) external;
}