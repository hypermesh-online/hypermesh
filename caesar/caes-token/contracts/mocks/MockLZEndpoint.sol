// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title MockLZEndpoint
 * @dev Mock LayerZero endpoint for testing
 */
contract MockLZEndpoint {
    // Minimal mock implementation for testing
    address public owner;
    
    constructor() {
        owner = msg.sender;
    }
    
    // Mock functions to satisfy interface requirements
    function send() external payable {
        // Empty implementation for testing
    }
    
    function quote() external view returns (uint256) {
        return 0.001 ether;
    }
    
    function isValidReceiveLibrary() external pure returns (bool) {
        return true;
    }
    
    function defaultSendLibrary() external pure returns (address) {
        return address(0);
    }
    
    function defaultReceiveLibrary() external pure returns (address) {
        return address(0);
    }
    
    function setConfig() external {
        // Empty implementation
    }
    
    function setSendLibrary() external {
        // Empty implementation
    }
    
    function setReceiveLibrary() external {
        // Empty implementation
    }
    
    function setDelegate() external {
        // Empty implementation
    }
}