// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "../interfaces/IEconomicEngine.sol";

/**
 * @title SimpleEconomicEngine
 * @dev Complete mock implementation of EconomicEngine for testing
 * Implements all IEconomicEngine functions with simplified logic for testing
 */
contract SimpleEconomicEngine is Ownable, ReentrancyGuard, Pausable, IEconomicEngine {
    
    // Mock state variables
    EconomicParameters public economicParams;
    MarketMetrics public marketMetrics;
    SystemHealth public systemHealth;
    EconomicHealthMetrics public healthMetrics;
    
    mapping(address => AccountEconomicData) public accountData;
    mapping(address => FiatActivityData) public fiatActivity;
    mapping(address => bool) public exemptAccounts;
    mapping(address => bool) public emergencyOperators;
    mapping(uint32 => ChainEconomicState) public chainStates;
    
    bool public emergencyMode;
    uint256 public emergencyTimestamp;
    uint256 public lastHealthUpdate;
    
    // Mock addresses for dependencies
    address public immutable demurrageManager;
    address public immutable antiSpeculationEngine;
    address public immutable stabilityPool;
    
    constructor(
        address _owner,
        address _demurrageManager,
        address _antiSpeculationEngine,
        address _stabilityPool
    ) Ownable(_owner) {
        demurrageManager = _demurrageManager;
        antiSpeculationEngine = _antiSpeculationEngine;
        stabilityPool = _stabilityPool;
        
        // Initialize default parameters
        economicParams = EconomicParameters({
            baseDemurrageRate: 50,
            maxDemurrageRate: 200,
            stabilityThreshold: 100,
            fiatDiscountFactor: 5000,
            gracePeriodsHours: 48,
            interventionThreshold: 500,
            rebalanceFrequency: 3600,
            emergencyThreshold: 1000
        });
        
        systemHealth = SystemHealth({
            priceStability: 1000,
            liquidityHealth: 1000,
            participationRate: 500,
            reserveRatio: 1000,
            lastUpdate: block.timestamp
        });
        
        healthMetrics = EconomicHealthMetrics({
            overallHealth: 900,
            priceStability: 1000,
            liquidityHealth: 1000,
            participationRate: 500,
            reserveRatio: 1000,
            demurrageEfficiency: 750,
            antiSpeculationEffectiveness: 850,
            systemStress: 100,
            timestamp: block.timestamp
        });
        
        lastHealthUpdate = block.timestamp;
    }
    
    // ============ Core Economic Functions ============
    
    function calculateDemurrage(address account, uint256 balance) 
        external view override returns (uint256 demurrageAmount) {
        if (exemptAccounts[account] || balance == 0) {
            return 0;
        }
        
        AccountEconomicData memory data = accountData[account];
        uint256 activityTime = data.lastActivity;
        
        // For testing: check if grace period has ended based on contract deployment time
        if (activityTime == 0) {
            // If contract has been deployed for more than grace period (48h), assume account had activity at grace period end
            uint256 contractAge = block.timestamp - systemHealth.lastUpdate; // lastUpdate set in constructor
            if (contractAge > economicParams.gracePeriodsHours * 3600) {
                // Assume activity started after grace period ended
                activityTime = systemHealth.lastUpdate + (economicParams.gracePeriodsHours * 3600);
            } else {
                return 0; // Still in grace period or new account
            }
        }
        
        uint256 timeElapsed = block.timestamp - activityTime;
        if (timeElapsed < 3600) { // 1 hour
            return 0;
        }
        
        // Simple calculation: 0.5% per hour for testing
        uint256 hoursElapsed = timeElapsed / 3600;
        return (balance * economicParams.baseDemurrageRate * hoursElapsed) / 100000;
    }
    
    function applyDemurrage(address account, uint256 balance) 
        external override nonReentrant returns (uint256 appliedAmount) {
        require(msg.sender == owner() || msg.sender == address(this), "Unauthorized");
        
        appliedAmount = this.calculateDemurrage(account, balance);
        
        if (appliedAmount > 0) {
            accountData[account].lastDemurrageApplication = block.timestamp;
            accountData[account].totalDemurragePaid += appliedAmount;
            marketMetrics.totalDemurrageCollected += appliedAmount;
            
            emit DemurrageApplied(account, appliedAmount, economicParams.baseDemurrageRate);
        }
        
        return appliedAmount;
    }
    
    function analyzeTransactionPattern(
        address account,
        uint256 amount,
        uint8 transactionType
    ) external override returns (uint256 riskScore, uint256 penalty) {
        AccountEconomicData storage data = accountData[account];
        
        data.transactionCount++;
        data.totalVolume += amount;
        uint256 currentTime = block.timestamp;
        
        // Simple risk calculation for testing
        if (data.lastTransactionTime > 0 && currentTime - data.lastTransactionTime < 300) { // 5 minutes
            riskScore = 800; // High risk for rapid transactions
            penalty = (amount * 100) / 10000; // 1% penalty
        } else {
            riskScore = 100; // Low risk
            penalty = 0;
        }
        
        data.lastTransactionTime = currentTime;
        data.riskScore = riskScore;
        
        if (penalty > 0) {
            emit SpeculationPenaltyApplied(account, penalty, "High risk transaction pattern");
        }
        
        return (riskScore, penalty);
    }
    
    function applySpeculationPenalty(address account, uint256 penalty) 
        external override nonReentrant returns (bool success) {
        require(msg.sender == owner() || msg.sender == address(this), "Unauthorized");
        
        if (penalty > 0) {
            accountData[account].totalPenaltiesPaid += penalty;
            marketMetrics.totalPenaltiesCollected += penalty;
            
            emit SpeculationPenaltyApplied(account, penalty, "Applied speculation penalty");
            return true;
        }
        
        return false;
    }
    
    function maintainPegStability() external override returns (StabilityAction action) {
        // Mock stability logic
        uint256 currentPrice = 1e18; // $1 for testing
        uint256 deviation = 0; // No deviation in mock
        
        if (deviation > economicParams.emergencyThreshold) {
            return StabilityAction.EMERGENCY_INTERVENTION;
        } else if (deviation > economicParams.interventionThreshold) {
            return StabilityAction.ADJUST_RATES;
        }
        
        return StabilityAction.NO_ACTION;
    }
    
    function rebalanceReserves() external override returns (ReserveOperation memory operation) {
        operation = ReserveOperation({
            operationType: ReserveOperationType.REBALANCE_CHAINS,
            amount: 0,
            targetChain: 0,
            timestamp: block.timestamp,
            success: true
        });
        
        systemHealth.lastUpdate = block.timestamp;
        return operation;
    }
    
    function updateEconomicParameters(EconomicParameters calldata params) 
        external override onlyOwner {
        require(params.baseDemurrageRate <= params.maxDemurrageRate, "Invalid demurrage rates");
        require(params.maxDemurrageRate <= 300, "Demurrage rate too high");
        require(params.fiatDiscountFactor <= 10000, "Invalid discount factor");
        
        economicParams = params;
        emit EconomicParametersUpdated(params);
    }
    
    function monitorEconomicHealth() external override returns (HealthMetrics memory metrics) {
        healthMetrics.timestamp = block.timestamp;
        healthMetrics.overallHealth = 900; // Mock healthy system
        
        lastHealthUpdate = block.timestamp;
        systemHealth.lastUpdate = block.timestamp;
        
        emit HealthMetricsUpdated(healthMetrics);
        
        return HealthMetrics({
            overallHealth: healthMetrics.overallHealth,
            priceStability: healthMetrics.priceStability,
            liquidityHealth: healthMetrics.liquidityHealth,
            participationRate: healthMetrics.participationRate,
            lastUpdate: healthMetrics.timestamp
        });
    }
    
    // ============ Fiat Integration Functions ============
    
    function recordFiatActivity(address account, uint256 amount, uint8 activityType) 
        external override {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        FiatActivityData storage activity = fiatActivity[account];
        activity.totalFiatVolume += amount;
        activity.lastFiatActivity = block.timestamp;
        activity.activityCount++;
        activity.discountEligible = true;
        activity.discountPercentage = 2500; // 25% discount for testing
        
        // Also initialize account activity for demurrage calculation
        if (accountData[account].lastActivity == 0) {
            accountData[account].lastActivity = block.timestamp;
        }
        
        emit FiatActivityRecorded(account, amount, activityType);
    }
    
    function validateFiatBacking(uint256 expectedBacking) 
        external override view returns (bool isValid, uint256 actualBacking) {
        // Mock validation - always valid for testing
        actualBacking = expectedBacking;
        isValid = true;
        return (isValid, actualBacking);
    }
    
    // ============ Cross-Chain Functions ============
    
    function synchronizeParameters(uint32 chainId, EconomicParameters calldata parameters) 
        external override {
        require(msg.sender == owner(), "Unauthorized");
        
        ChainEconomicState storage state = chainStates[chainId];
        state.parameters = parameters;
        state.lastSync = block.timestamp;
        state.syncHash = keccak256(abi.encode(parameters));
        
        emit CrossChainSync(chainId, state.syncHash);
    }
    
    // ============ View Functions ============
    
    function getEconomicParameters() external view override returns (EconomicParameters memory) {
        return economicParams;
    }
    
    function getSystemHealth() external view override returns (SystemHealth memory) {
        return systemHealth;
    }
    
    function getHealthMetrics() external view override returns (EconomicHealthMetrics memory) {
        return healthMetrics;
    }
    
    function getAccountEconomicData(address account) external view override returns (AccountEconomicData memory) {
        return accountData[account];
    }
    
    function getFiatActivityData(address account) external view override returns (FiatActivityData memory) {
        return fiatActivity[account];
    }
    
    function getChainEconomicState(uint32 chainId) external view override returns (ChainEconomicState memory) {
        return chainStates[chainId];
    }
    
    function isAccountExempt(address account) external view override returns (bool) {
        return exemptAccounts[account];
    }
    
    function getCurrentDemurrageRate(address) external view override returns (uint256) {
        return economicParams.baseDemurrageRate;
    }
    
    function getSpeculationRiskScore(address account) external view override returns (uint256) {
        return accountData[account].riskScore;
    }
    
    function getMarketMetrics() external view override returns (MarketMetrics memory) {
        return marketMetrics;
    }
    
    // ============ Administrative Functions ============
    
    function setAccountExemption(address account, bool exempt) external override onlyOwner {
        exemptAccounts[account] = exempt;
    }
    
    function addEmergencyOperator(address operator) external override onlyOwner {
        emergencyOperators[operator] = true;
    }
    
    function removeEmergencyOperator(address operator) external override onlyOwner {
        emergencyOperators[operator] = false;
    }
    
    function activateEmergencyMode(string calldata reason) external override {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        
        emergencyMode = true;
        emergencyTimestamp = block.timestamp;
        
        economicParams.baseDemurrageRate = 0;
        economicParams.maxDemurrageRate = 0;
        
        emit EmergencyModeActivated(msg.sender, reason);
    }
    
    function deactivateEmergencyMode() external override onlyOwner {
        require(emergencyMode, "Not in emergency mode");
        
        emergencyMode = false;
        emergencyTimestamp = 0;
        
        economicParams.baseDemurrageRate = 50;
        economicParams.maxDemurrageRate = 200;
    }
    
    function pause() external override onlyOwner {
        _pause();
    }
    
    function unpause() external override onlyOwner {
        _unpause();
    }
    
    // ============ Legacy Functions for Compatibility ============
    
    function processTransaction(address, address, uint256, uint8) external pure returns (uint256) {
        return 0; // No fee for testing
    }
    
    function getCurrentPrice() external pure returns (uint256) {
        return 1e18; // Always return $1.00 for testing
    }
    
    function getNetworkHealthIndex() external pure returns (uint256) {
        return 1e18; // 100% health for testing
    }
    
    function adjustParameters() external {
        // Mock implementation - does nothing
    }
    
    // ============ Test Helper Functions ============
    
    /**
     * @dev Initialize account activity for testing (test helper function)
     */
    function initializeAccountActivity(address account) external {
        accountData[account].lastActivity = block.timestamp;
        accountData[account].isNewUser = false;
        accountData[account].graceEndTime = block.timestamp;
    }
    
    /**
     * @dev Set account activity timestamp for testing
     */
    function setAccountActivity(address account, uint256 timestamp) external {
        accountData[account].lastActivity = timestamp;
    }
    
    /**
     * @dev Simulate ending grace period for testing (test helper function)
     */
    function endGracePeriod(address account) external {
        // Set account activity to simulate grace period end
        accountData[account].lastActivity = block.timestamp;
        accountData[account].isNewUser = false;
        accountData[account].graceEndTime = block.timestamp;
    }
}
