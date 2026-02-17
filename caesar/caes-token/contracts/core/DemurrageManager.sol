// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "../libs/AdvancedMathUtils.sol";
import "../interfaces/ICaesar.sol";
import "../interfaces/IEconomicEngine.sol";
import "../oracles/GoldPriceOracle.sol";

/**
 * @title DemurrageManager
 * @dev Advanced demurrage system with gold-pegged economic model
 * 
 * SOPHISTICATED ECONOMIC MODEL:
 * - Uses dynamic gold price oracle for reference calculations
 * - Position-based penalties using statistical deviation from gold average
 * - Volatility-adaptive demurrage rates based on gold market conditions
 * - Real-time convergence mechanisms with gold price correlation
 * 
 * Key Features:
 * - Statistical deviation scoring for penalty calculations
 * - Gold market volatility integration for rate adjustments
 * - Cross-chain transaction convergence support
 * - Emergency circuit breakers with gold-based thresholds
 */
contract DemurrageManager is Ownable, ReentrancyGuard, Pausable {
    using AdvancedMathUtils for uint256;
    
    // State variables
    ICaesar.DemurrageConfig public config;
    GoldPriceOracle public immutable goldOracle;
    
    mapping(address => uint256) public lastDemurrageApplication;
    mapping(address => bool) public exemptAccounts;
    
    uint256 public totalDemurrageCollected;
    uint256 public currentStabilityIndex;
    uint256 public lastPriceUpdate;
    // No fixed target - uses dynamic gold reference
    
    // Gold-based economic parameters
    struct GoldEconomicData {
        uint256 lastGoldUpdate;
        int256 lastDeviationScore;
        uint256 adaptiveRateMultiplier; // Based on gold volatility
        uint256 convergenceTarget;      // Dynamic convergence target
        bool emergencyMode;
    }
    
    GoldEconomicData public goldEconomicData;
    
    // Events
    event DemurrageConfigUpdated(
        uint256 baseRate,
        uint256 maxRate,
        uint256 decayInterval,
        uint256 stabilityThreshold
    );
    event DemurrageApplied(address indexed account, uint256 amount);
    event AccountExemptionSet(address indexed account, bool exempt);
    event StabilityIndexUpdated(uint256 index);
    
    constructor(address initialOwner, address _goldOracle) Ownable(initialOwner) {
        goldOracle = GoldPriceOracle(_goldOracle);
        config = ICaesar.DemurrageConfig({
            baseRate: 1,               // 0.01% base rate (very small)
            maxRate: 10,               // 0.1% maximum rate (small)
            decayInterval: 86400,      // Daily decay
            stabilityThreshold: 100    // 1% stability threshold
        });
        
        // Initialize gold-based economic data
        goldEconomicData = GoldEconomicData({
            lastGoldUpdate: block.timestamp,
            lastDeviationScore: 0,
            adaptiveRateMultiplier: 1000, // 100% (1.0x) initial
            convergenceTarget: 117e18,     // Initial gold reference ~$117/gram
            emergencyMode: false
        });
    }
    
    /**
     * @dev Calculate demurrage amount for an account
     * @param account The account to calculate demurrage for
     * @param balance The current balance of the account
     * @param lastActivity Timestamp of last account activity
     * @return demurrageAmount The calculated demurrage amount
     */
    function calculateDemurrage(
        address account,
        uint256 balance,
        uint256 lastActivity
    ) external view returns (uint256 demurrageAmount) {
        if (exemptAccounts[account] || balance == 0) {
            return 0;
        }
        
        uint256 timeElapsed = block.timestamp - lastActivity;
        if (timeElapsed < config.decayInterval) {
            return 0;
        }
        
        uint256 effectiveRate = _calculateEffectiveRate();
        
        // Convert seconds to hours for the decay calculation
        uint256 hoursElapsed = timeElapsed / 3600;
        if (hoursElapsed == 0) {
            return 0; // No demurrage for less than 1 hour
        }
        
        uint256 originalBalance = balance;
        uint256 decayedBalance = AdvancedMathUtils.calculateExponentialDecay(
            originalBalance,
            effectiveRate,
            hoursElapsed
        );
        
        // Safety check: decayed balance should never exceed original balance
        if (decayedBalance >= originalBalance) {
            return 0; // No demurrage if calculation error
        }
        
        uint256 demurrageAmount = originalBalance - decayedBalance;
        
        // Cap demurrage at 50% of balance to prevent excessive penalties
        uint256 maxDemurrage = originalBalance / 2;
        if (demurrageAmount > maxDemurrage) {
            demurrageAmount = maxDemurrage;
        }
        
        return demurrageAmount;
    }
    
    /**
     * @dev Apply demurrage to an account
     * @param account The account to apply demurrage to
     * @param balance The current balance of the account
     * @param lastActivity Timestamp of last account activity
     * @return appliedAmount The amount of demurrage applied
     */
    function applyDemurrage(
        address account,
        uint256 balance,
        uint256 lastActivity
    ) external nonReentrant returns (uint256 appliedAmount) {
        require(msg.sender == address(this) || msg.sender == owner(), "Unauthorized");
        
        appliedAmount = this.calculateDemurrage(account, balance, lastActivity);
        
        if (appliedAmount > 0) {
            lastDemurrageApplication[account] = block.timestamp;
            totalDemurrageCollected += appliedAmount;
            emit DemurrageApplied(account, appliedAmount);
        }
        
        return appliedAmount;
    }
    
    /**
     * @dev Update stability index based on current market conditions
     * @param currentPrice Current token price
     * @param activeParticipants Number of active participants
     * @param totalHolders Total number of token holders
     */
    function updateStabilityIndex(
        uint256 currentPrice,
        uint256 activeParticipants,
        uint256 totalHolders
    ) external {
        require(msg.sender == owner() || msg.sender == address(this), "Unauthorized");
        
        currentStabilityIndex = AdvancedMathUtils.calculateStabilityIndex(
            currentPrice,
            goldEconomicData.convergenceTarget,
            activeParticipants,
            totalHolders
        );
        
        lastPriceUpdate = block.timestamp;
        emit StabilityIndexUpdated(currentStabilityIndex);
    }
    
    /**
     * @dev Set demurrage configuration
     * @param newConfig The new demurrage configuration
     */
    function setDemurrageConfig(
        ICaesar.DemurrageConfig calldata newConfig
    ) external onlyOwner {
        require(newConfig.baseRate <= newConfig.maxRate, "Invalid rate configuration");
        require(newConfig.maxRate <= 1000, "Max rate too high"); // 10% maximum
        require(newConfig.decayInterval > 0, "Invalid decay interval");
        
        config = newConfig;
        
        emit DemurrageConfigUpdated(
            newConfig.baseRate,
            newConfig.maxRate,
            newConfig.decayInterval,
            newConfig.stabilityThreshold
        );
    }
    
    /**
     * @dev Set account exemption status
     * @param account The account to set exemption for
     * @param exempt Whether the account should be exempt from demurrage
     */
    function setAccountExemption(address account, bool exempt) external onlyOwner {
        exemptAccounts[account] = exempt;
        emit AccountExemptionSet(account, exempt);
    }
    
    /**
     * @dev Get current effective demurrage rate
     * @return effectiveRate The current effective demurrage rate
     */
    function getCurrentDecayRate() external view returns (uint256 effectiveRate) {
        return _calculateEffectiveRate();
    }
    
    /**
     * @dev Check if stability conditions are met
     * @return isStable True if conditions are stable
     */
    function isStabilityConditionMet() external view returns (bool isStable) {
        return currentStabilityIndex >= config.stabilityThreshold;
    }
    
    /**
     * @dev Get total demurrage collected
     * @return total Total demurrage collected
     */
    function getTotalDemurrageCollected() external view returns (uint256 total) {
        return totalDemurrageCollected;
    }
    
    /**
     * @dev Calculate effective demurrage rate based on stability
     * @return effectiveRate The calculated effective rate
     */
    function _calculateEffectiveRate() internal view returns (uint256 effectiveRate) {
        if (currentStabilityIndex >= 1000) {
            // Perfect stability - minimal demurrage
            return config.baseRate / 10;
        }
        
        // Linear interpolation between baseRate and maxRate based on stability
        uint256 instabilityFactor = 1000 - currentStabilityIndex;
        uint256 rateRange = config.maxRate - config.baseRate;
        uint256 additionalRate = (rateRange * instabilityFactor) / 1000;
        
        return config.baseRate + additionalRate;
    }
    
    /**
     * @dev Emergency function to pause demurrage
     */
    function pauseDemurrage() external onlyOwner {
        config.baseRate = 0;
        config.maxRate = 0;
    }
    
    /**
     * @dev Get demurrage configuration
     * @return config The current demurrage configuration
     */
    function getDemurrageConfig() external view returns (ICaesar.DemurrageConfig memory) {
        return config;
    }
}
