// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title MockPriceOracle
 * @dev Mock price oracle for testing purposes
 */
contract MockPriceOracle {
    uint256 private _price;
    uint8 private _decimals;
    bool private _validPrice;
    
    event PriceUpdated(uint256 newPrice);
    
    constructor(uint256 initialPrice) {
        _price = initialPrice;
        _decimals = 18;
        _validPrice = true;
    }
    
    function getPrice() external view returns (uint256) {
        require(_validPrice, "MockPriceOracle: Invalid price");
        return _price;
    }
    
    function setPrice(uint256 newPrice) external {
        _price = newPrice;
        emit PriceUpdated(newPrice);
    }
    
    function decimals() external view returns (uint8) {
        return _decimals;
    }
    
    function setDecimals(uint8 newDecimals) external {
        _decimals = newDecimals;
    }
    
    function setValidPrice(bool valid) external {
        _validPrice = valid;
    }
    
    function isValidPrice() external view returns (bool) {
        return _validPrice;
    }
    
    // Chainlink-like interface
    function latestRoundData() external view returns (
        uint80 roundId,
        int256 answer,
        uint256 startedAt,
        uint256 updatedAt,
        uint80 answeredInRound
    ) {
        return (
            1,
            int256(_price),
            block.timestamp,
            block.timestamp,
            1
        );
    }
    
    function getLatestPrice() external view returns (uint256) {
        require(_validPrice, "MockPriceOracle: Invalid price");
        return _price;
    }
}
