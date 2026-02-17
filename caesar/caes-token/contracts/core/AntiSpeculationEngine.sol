// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "../libs/MathUtils.sol";
import "../interfaces/ICaesar.sol";

/**
 * @title AntiSpeculationEngine
 * @dev Implements transaction analysis and speculation prevention mechanisms
 */
contract AntiSpeculationEngine is Ownable, ReentrancyGuard {
    using MathUtils for uint256;
    
    // Configuration
    ICaesar.AntiSpeculationConfig public config;
    
    // Account tracking
    struct TransactionHistory {
        uint256 lastTransactionTime;
        uint256 transactionCount;
        uint256 totalVolume;
        uint256 averageHoldingPeriod;
        uint256 rapidTradeCount;
        bool flaggedForSpeculation;
    }
    
    mapping(address => TransactionHistory) public accountHistory;
    mapping(address => uint256[]) public holdingPeriods; // Track holding periods
    mapping(address => bool) public whitelistedAddresses;
    
    // Global metrics
    uint256 public totalPenaltiesCollected;
    uint256 public averageNetworkHoldingPeriod;
    uint256 public speculationThreshold = 3; // Number of rapid trades to flag
    
    // Events
    event SpeculationPenaltyApplied(address indexed account, uint256 penalty, string reason);
    event AccountFlagged(address indexed account, string reason);
    event AccountUnflagged(address indexed account);
    event ConfigurationUpdated();
    event WhitelistUpdated(address indexed account, bool whitelisted);
    
    constructor(address initialOwner) Ownable(initialOwner) {
        config = ICaesar.AntiSpeculationConfig({
            maxHoldingPeriod: 30 days,     // 30 days optimal holding
            penaltyRate: 200,              // 2% penalty for speculation
            rapidTradePenalty: 100,        // 1% penalty for rapid trading
            minTransactionGap: 1 hours     // Minimum 1 hour between transactions
        });
    }
    
    /**
     * @dev Analyze transaction and update account metrics
     * @param account The account making the transaction
     * @param amount The transaction amount
     * @param transactionType Type of transaction (0=transfer, 1=buy, 2=sell)
     * @return penalty Calculated penalty amount
     */
    function analyzeTransaction(
        address account,
        uint256 amount,
        uint8 transactionType
    ) external returns (uint256 penalty) {
        require(msg.sender == owner() || msg.sender == address(this), "Unauthorized");
        
        if (whitelistedAddresses[account]) {
            return 0;
        }
        
        TransactionHistory storage history = accountHistory[account];
        uint256 currentTime = block.timestamp;
        
        // Update transaction metrics
        uint256 timeSinceLastTx = currentTime - history.lastTransactionTime;
        history.transactionCount++;
        history.totalVolume += amount;
        
        // Check for rapid trading
        if (timeSinceLastTx < config.minTransactionGap && history.lastTransactionTime != 0) {
            history.rapidTradeCount++;
            penalty += _calculateRapidTradePenalty(amount);
            emit SpeculationPenaltyApplied(account, penalty, "Rapid trading detected");
        }
        
        // Check for excessive holding (for sells)
        if (transactionType == 2 && history.lastTransactionTime != 0) {
            uint256 holdingPeriod = timeSinceLastTx;
            holdingPeriods[account].push(holdingPeriod);
            
            if (holdingPeriod > config.maxHoldingPeriod) {
                uint256 excessivePenalty = _calculateExcessiveHoldingPenalty(amount, holdingPeriod);
                penalty += excessivePenalty;
                emit SpeculationPenaltyApplied(account, excessivePenalty, "Excessive holding period");
            }
        }
        
        // Update speculation flag
        if (history.rapidTradeCount >= speculationThreshold) {
            if (!history.flaggedForSpeculation) {
                history.flaggedForSpeculation = true;
                emit AccountFlagged(account, "Excessive rapid trading");
            }
        }
        
        // Calculate volume-based penalty for flagged accounts
        if (history.flaggedForSpeculation) {
            uint256 volumePenalty = _calculateVolumePenalty(amount);
            penalty += volumePenalty;
            emit SpeculationPenaltyApplied(account, volumePenalty, "Flagged account penalty");
        }
        
        history.lastTransactionTime = currentTime;
        totalPenaltiesCollected += penalty;
        
        return penalty;
    }
    
    /**
     * @dev Calculate speculation penalty for an account
     * @param account The account to calculate penalty for
     * @param amount The transaction amount
     * @return penalty The calculated penalty amount
     */
    function calculateSpeculationPenalty(
        address account,
        uint256 amount
    ) external view returns (uint256 penalty) {
        if (whitelistedAddresses[account]) {
            return 0;
        }
        
        TransactionHistory memory history = accountHistory[account];
        
        // Rapid trade penalty
        if (block.timestamp - history.lastTransactionTime < config.minTransactionGap) {
            penalty += _calculateRapidTradePenalty(amount);
        }
        
        // Flagged account penalty
        if (history.flaggedForSpeculation) {
            penalty += _calculateVolumePenalty(amount);
        }
        
        return penalty;
    }
    
    /**
     * @dev Update participation score based on transaction patterns
     * @param account The account to update
     * @param transactionVolume Recent transaction volume
     * @return newScore The updated participation score
     */
    function updateParticipationScore(
        address account,
        uint256 transactionVolume
    ) external returns (uint256 newScore) {
        require(msg.sender == owner() || msg.sender == address(this), "Unauthorized");
        
        if (whitelistedAddresses[account]) {
            return 1000; // Max score for whitelisted accounts
        }
        
        TransactionHistory memory history = accountHistory[account];
        
        // Base score calculation
        newScore = 500; // Starting score
        
        // Penalty for rapid trading
        if (history.rapidTradeCount > 0) {
            uint256 rapidTradePenalty = (history.rapidTradeCount * 100);
            newScore = newScore > rapidTradePenalty ? newScore - rapidTradePenalty : 0;
        }
        
        // Bonus for consistent activity
        if (history.transactionCount >= 10) {
            uint256 averageInterval = _calculateAverageInterval(account);
            if (averageInterval >= 1 days && averageInterval <= 7 days) {
                newScore += 200; // Bonus for regular activity
            }
        }
        
        // Penalty for excessive volume concentration
        if (transactionVolume > history.totalVolume / 2) {
            newScore = newScore > 150 ? newScore - 150 : 0;
        }
        
        // Cap score at maximum
        if (newScore > 1000) {
            newScore = 1000;
        }
        
        return newScore;
    }
    
    /**
     * @dev Set anti-speculation configuration
     * @param newConfig New configuration parameters
     */
    function setAntiSpeculationConfig(
        ICaesar.AntiSpeculationConfig calldata newConfig
    ) external onlyOwner {
        require(newConfig.penaltyRate <= 1000, "Penalty rate too high"); // Max 10%
        require(newConfig.rapidTradePenalty <= 500, "Rapid trade penalty too high"); // Max 5%
        require(newConfig.minTransactionGap >= 300, "Transaction gap too short"); // Min 5 minutes
        require(newConfig.maxHoldingPeriod >= 1 days, "Holding period too short");
        
        config = newConfig;
        emit ConfigurationUpdated();
    }
    
    /**
     * @dev Set whitelist status for an account
     * @param account The account to whitelist/unwhitelist
     * @param whitelisted Whether the account should be whitelisted
     */
    function setWhitelistStatus(address account, bool whitelisted) external onlyOwner {
        whitelistedAddresses[account] = whitelisted;
        emit WhitelistUpdated(account, whitelisted);
    }
    
    /**
     * @dev Unflag an account manually
     * @param account The account to unflag
     */
    function unflagAccount(address account) external onlyOwner {
        accountHistory[account].flaggedForSpeculation = false;
        accountHistory[account].rapidTradeCount = 0;
        emit AccountUnflagged(account);
    }
    
    /**
     * @dev Get account transaction history
     * @param account The account to query
     * @return history The transaction history
     */
    function getAccountHistory(
        address account
    ) external view returns (TransactionHistory memory history) {
        return accountHistory[account];
    }
    
    /**
     * @dev Check if account is flagged for speculation
     * @param account The account to check
     * @return flagged Whether the account is flagged
     */
    function isAccountFlagged(address account) external view returns (bool flagged) {
        return accountHistory[account].flaggedForSpeculation;
    }
    
    /**
     * @dev Get total penalties collected
     * @return total Total penalties collected
     */
    function getTotalPenaltiesCollected() external view returns (uint256 total) {
        return totalPenaltiesCollected;
    }
    
    // Internal functions
    
    /**
     * @dev Calculate rapid trade penalty
     * @param amount Transaction amount
     * @return penalty Penalty amount
     */
    function _calculateRapidTradePenalty(uint256 amount) internal view returns (uint256 penalty) {
        return (amount * config.rapidTradePenalty) / 10000;
    }
    
    /**
     * @dev Calculate excessive holding penalty
     * @param amount Transaction amount
     * @param holdingPeriod Holding period in seconds
     * @return penalty Penalty amount
     */
    function _calculateExcessiveHoldingPenalty(
        uint256 amount,
        uint256 holdingPeriod
    ) internal view returns (uint256 penalty) {
        if (holdingPeriod <= config.maxHoldingPeriod) {
            return 0;
        }
        
        uint256 excessDays = (holdingPeriod - config.maxHoldingPeriod) / 1 days;
        uint256 penaltyRate = config.penaltyRate + (excessDays * 10); // Escalating penalty
        
        // Cap penalty at 10%
        if (penaltyRate > 1000) {
            penaltyRate = 1000;
        }
        
        return (amount * penaltyRate) / 10000;
    }
    
    /**
     * @dev Calculate volume-based penalty for flagged accounts
     * @param amount Transaction amount
     * @return penalty Penalty amount
     */
    function _calculateVolumePenalty(uint256 amount) internal view returns (uint256 penalty) {
        return (amount * config.penaltyRate) / 10000;
    }
    
    /**
     * @dev Calculate average transaction interval for an account
     * @param account The account to analyze
     * @return averageInterval Average interval in seconds
     */
    function _calculateAverageInterval(address account) internal view returns (uint256 averageInterval) {
        uint256[] memory periods = holdingPeriods[account];
        if (periods.length == 0) {
            return 0;
        }
        
        uint256 totalPeriod = 0;
        for (uint256 i = 0; i < periods.length; i++) {
            totalPeriod += periods[i];
        }
        
        return totalPeriod / periods.length;
    }
    
    /**
     * @dev Get anti-speculation configuration
     * @return config Current configuration
     */
    function getAntiSpeculationConfig() external view returns (ICaesar.AntiSpeculationConfig memory) {
        return config;
    }
    
    /**
     * @dev Emergency pause function
     */
    function pauseAntiSpeculation() external onlyOwner {
        config.penaltyRate = 0;
        config.rapidTradePenalty = 0;
        config.minTransactionGap = 0;
    }
}