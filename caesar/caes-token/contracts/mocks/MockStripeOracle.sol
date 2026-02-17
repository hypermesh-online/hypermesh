// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title MockStripeOracle
 * @dev Mock Stripe payment oracle for testing fiat integration
 */
contract MockStripeOracle {
    struct PaymentData {
        bytes32 paymentId;
        address user;
        uint256 amount;
        string currency;
        bool confirmed;
        uint256 timestamp;
    }
    
    mapping(bytes32 => PaymentData) public payments;
    mapping(address => uint256) public userTotalDeposits;
    
    event PaymentConfirmed(bytes32 indexed paymentId, address indexed user, uint256 amount);
    event PaymentFailed(bytes32 indexed paymentId, string reason);
    
    function confirmPayment(
        bytes32 paymentId,
        address user,
        uint256 amount,
        string calldata currency
    ) external {
        payments[paymentId] = PaymentData({
            paymentId: paymentId,
            user: user,
            amount: amount,
            currency: currency,
            confirmed: true,
            timestamp: block.timestamp
        });
        
        userTotalDeposits[user] += amount;
        emit PaymentConfirmed(paymentId, user, amount);
    }
    
    function failPayment(bytes32 paymentId, string calldata reason) external {
        payments[paymentId].confirmed = false;
        emit PaymentFailed(paymentId, reason);
    }
    
    function getPayment(bytes32 paymentId) external view returns (PaymentData memory) {
        return payments[paymentId];
    }
    
    function isPaymentConfirmed(bytes32 paymentId) external view returns (bool) {
        return payments[paymentId].confirmed;
    }
    
    function getConversionRate(string calldata currency) external pure returns (uint256) {
        // Mock conversion rates (in basis points, 10000 = 1.0)
        if (keccak256(bytes(currency)) == keccak256(bytes("USD"))) {
            return 10000; // 1:1 for USD
        }
        if (keccak256(bytes(currency)) == keccak256(bytes("EUR"))) {
            return 10800; // 1 EUR = 1.08 USD
        }
        return 10000; // Default 1:1
    }
}
