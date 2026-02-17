// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title MockChainlinkOracle
 * @dev Mock Chainlink price oracle for testing
 * Implements AggregatorV3Interface
 */
contract MockChainlinkOracle {
    int256 private _price;
    uint8 private _decimals;
    uint256 private _updatedAt;
    uint80 private _roundId;
    
    event AnswerUpdated(int256 indexed current, uint256 indexed roundId, uint256 updatedAt);
    
    constructor(int256 initialPrice) {
        _price = initialPrice;
        _decimals = 8;
        _updatedAt = block.timestamp;
        _roundId = 1;
    }
    
    function decimals() external view returns (uint8) {
        return _decimals;
    }
    
    function description() external pure returns (string memory) {
        return "Mock Chainlink Oracle";
    }
    
    function version() external pure returns (uint256) {
        return 1;
    }
    
    function latestRoundData()
        external
        view
        returns (
            uint80 roundId,
            int256 answer,
            uint256 startedAt,
            uint256 updatedAt,
            uint80 answeredInRound
        )
    {
        return (_roundId, _price, _updatedAt, _updatedAt, _roundId);
    }
    
    function getRoundData(uint80 _roundId)
        external
        view
        returns (
            uint80 roundId,
            int256 answer,
            uint256 startedAt,
            uint256 updatedAt,
            uint80 answeredInRound
        )
    {
        return (_roundId, _price, _updatedAt, _updatedAt, _roundId);
    }
    
    function latestAnswer() external view returns (int256) {
        return _price;
    }
    
    function latestTimestamp() external view returns (uint256) {
        return _updatedAt;
    }
    
    function latestRound() external view returns (uint256) {
        return _roundId;
    }
    
    function getAnswer(uint256 roundId) external view returns (int256) {
        return _price;
    }
    
    function getTimestamp(uint256 roundId) external view returns (uint256) {
        return _updatedAt;
    }
    
    // Admin functions for testing
    function setPrice(int256 newPrice) external {
        _price = newPrice;
        _updatedAt = block.timestamp;
        _roundId++;
        emit AnswerUpdated(newPrice, _roundId, _updatedAt);
    }
    
    function setDecimals(uint8 newDecimals) external {
        _decimals = newDecimals;
    }
    
    function setUpdatedAt(uint256 timestamp) external {
        _updatedAt = timestamp;
    }
}
