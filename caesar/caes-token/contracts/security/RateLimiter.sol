// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title RateLimiter
 * @dev Simple rate limiting implementation to prevent DoS attacks
 * Limits number of transactions per time window for each address
 */
contract RateLimiter {
    
    struct RateLimitInfo {
        uint256 transactionCount;
        uint256 windowStart;
        uint256 lastTransaction;
    }
    
    // Default rate limits
    uint256 public constant DEFAULT_MAX_TRANSACTIONS = 20;
    uint256 public constant DEFAULT_TIME_WINDOW = 1 hours;
    uint256 public constant DEFAULT_MIN_INTERVAL = 1 seconds;
    
    // Per-address rate limit info
    mapping(address => RateLimitInfo) public rateLimits;
    
    // Custom limits per address (0 means use default)
    mapping(address => uint256) public customMaxTransactions;
    mapping(address => uint256) public customTimeWindow;
    mapping(address => uint256) public customMinInterval;
    
    // Emergency rate limits for high-risk addresses
    mapping(address => bool) public emergencyRateLimit;
    uint256 public constant EMERGENCY_MAX_TRANSACTIONS = 5;
    uint256 public constant EMERGENCY_TIME_WINDOW = 1 hours;
    uint256 public constant EMERGENCY_MIN_INTERVAL = 10 seconds;
    
    // Events
    event RateLimitExceeded(address indexed account, uint256 transactionCount, uint256 limit);
    event EmergencyRateLimitActivated(address indexed account);
    event CustomRateLimitSet(address indexed account, uint256 maxTx, uint256 timeWindow, uint256 minInterval);
    
    modifier rateLimited(address account) {
        require(isWithinRateLimit(account), "RateLimiter: RATE_LIMIT_EXCEEDED");
        updateRateLimit(account);
        _;
    }
    
    /**
     * @dev Check if account is within rate limits
     */
    function isWithinRateLimit(address account) public view returns (bool) {
        RateLimitInfo memory info = rateLimits[account];
        uint256 currentTime = block.timestamp;
        
        // Get rate limits for this account
        (uint256 maxTx, uint256 timeWindow, uint256 minInterval) = getRateLimits(account);
        
        // Check minimum interval between transactions
        if (info.lastTransaction > 0 && currentTime - info.lastTransaction < minInterval) {
            return false;
        }
        
        // If outside time window, reset counter
        if (currentTime - info.windowStart >= timeWindow) {
            return true; // Will reset in updateRateLimit
        }
        
        // Check if within transaction limit
        return info.transactionCount < maxTx;
    }
    
    /**
     * @dev Update rate limit counters
     */
    function updateRateLimit(address account) internal {
        RateLimitInfo storage info = rateLimits[account];
        uint256 currentTime = block.timestamp;
        
        // Get rate limits for this account
        (uint256 maxTx, uint256 timeWindow,) = getRateLimits(account);
        
        // If outside time window, reset
        if (currentTime - info.windowStart >= timeWindow) {
            info.transactionCount = 0;
            info.windowStart = currentTime;
        }
        
        // Increment counter
        info.transactionCount++;
        info.lastTransaction = currentTime;
        
        // Emit event if limit was exceeded
        if (info.transactionCount > maxTx) {
            emit RateLimitExceeded(account, info.transactionCount, maxTx);
        }
    }
    
    /**
     * @dev Get effective rate limits for an account
     */
    function getRateLimits(address account) public view returns (
        uint256 maxTransactions,
        uint256 timeWindow,
        uint256 minInterval
    ) {
        if (emergencyRateLimit[account]) {
            return (EMERGENCY_MAX_TRANSACTIONS, EMERGENCY_TIME_WINDOW, EMERGENCY_MIN_INTERVAL);
        }
        
        maxTransactions = customMaxTransactions[account] > 0 ? 
            customMaxTransactions[account] : DEFAULT_MAX_TRANSACTIONS;
        timeWindow = customTimeWindow[account] > 0 ? 
            customTimeWindow[account] : DEFAULT_TIME_WINDOW;
        minInterval = customMinInterval[account] > 0 ? 
            customMinInterval[account] : DEFAULT_MIN_INTERVAL;
    }
    
    /**
     * @dev Set custom rate limits for an account (internal)
     */
    function _setCustomRateLimit(
        address account,
        uint256 maxTransactions,
        uint256 timeWindow,
        uint256 minInterval
    ) internal {
        require(maxTransactions > 0 && maxTransactions <= 1000, "Invalid max transactions");
        require(timeWindow >= 1 minutes && timeWindow <= 24 hours, "Invalid time window");
        require(minInterval <= 1 hours, "Invalid min interval");
        
        customMaxTransactions[account] = maxTransactions;
        customTimeWindow[account] = timeWindow;
        customMinInterval[account] = minInterval;
        
        emit CustomRateLimitSet(account, maxTransactions, timeWindow, minInterval);
    }
    
    /**
     * @dev Activate emergency rate limiting for high-risk account
     */
    function _activateEmergencyRateLimit(address account) internal {
        emergencyRateLimit[account] = true;
        emit EmergencyRateLimitActivated(account);
    }
    
    /**
     * @dev Deactivate emergency rate limiting
     */
    function _deactivateEmergencyRateLimit(address account) internal {
        emergencyRateLimit[account] = false;
    }
    
    /**
     * @dev Get current rate limit status for account
     */
    function getRateLimitStatus(address account) external view returns (
        uint256 currentCount,
        uint256 maxTransactions,
        uint256 windowStart,
        uint256 lastTransaction,
        bool isEmergency
    ) {
        RateLimitInfo memory info = rateLimits[account];
        (maxTransactions,,) = getRateLimits(account);
        
        return (
            info.transactionCount,
            maxTransactions,
            info.windowStart,
            info.lastTransaction,
            emergencyRateLimit[account]
        );
    }
    
    /**
     * @dev Check how much time until next transaction is allowed
     */
    function getTimeUntilNextTransaction(address account) external view returns (uint256) {
        RateLimitInfo memory info = rateLimits[account];
        (,, uint256 minInterval) = getRateLimits(account);
        
        if (info.lastTransaction == 0) {
            return 0;
        }
        
        uint256 timeSinceLastTx = block.timestamp - info.lastTransaction;
        if (timeSinceLastTx >= minInterval) {
            return 0;
        }
        
        return minInterval - timeSinceLastTx;
    }
    
    /**
     * @dev Reset rate limit for account (emergency function)
     */
    function _resetRateLimit(address account) internal {
        delete rateLimits[account];
    }
}