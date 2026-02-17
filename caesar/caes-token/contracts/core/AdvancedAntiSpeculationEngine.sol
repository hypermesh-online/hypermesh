// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "../libs/AdvancedMathUtils.sol";
import "../interfaces/ICaesar.sol";

/**
 * @title AdvancedAntiSpeculationEngine
 * @dev Sophisticated anti-speculation system with ML-based detection algorithms,
 * progressive penalty systems, and market manipulation prevention for Caesar Token
 */
contract AdvancedAntiSpeculationEngine is Ownable, ReentrancyGuard, Pausable {
    using AdvancedMathUtils for uint256;
    
    // ============ Constants ============
    uint256 public constant PRECISION = 1e18;
    uint256 public constant BASIS_POINTS = 10000;
    uint256 public constant MAX_RISK_SCORE = 1000;
    uint256 public constant HIGH_RISK_THRESHOLD = 700;
    uint256 public constant MEDIUM_RISK_THRESHOLD = 500;
    uint256 public constant MAX_PENALTY_RATE = 1000; // 10%
    uint256 public constant CIRCUIT_BREAKER_THRESHOLD = 900; // 90% risk score
    uint256 public constant PATTERN_HISTORY_LENGTH = 50; // Number of transactions to analyze
    
    // ============ Structs ============
    
    struct AdvancedAntiSpeculationConfig {
        uint256 rapidTradeThreshold;         // Minimum time between trades (seconds)
        uint256 volumeConcentrationLimit;    // Maximum volume concentration (basis points)
        uint256 patternDetectionSensitivity; // Pattern detection sensitivity (0-1000)
        uint256 basePenaltyRate;             // Base penalty rate (basis points)
        uint256 progressivePenaltyMultiplier; // Progressive penalty multiplier
        uint256 circuitBreakerCooldown;      // Circuit breaker cooldown period
        uint256 whitelistGracePeriod;        // Grace period for whitelisted accounts
        bool mlDetectionEnabled;             // Whether ML-based detection is enabled
    }
    
    struct TransactionPattern {
        uint256 timestamp;                   // Transaction timestamp
        uint256 amount;                      // Transaction amount
        uint8 transactionType;               // Transaction type (0=transfer, 1=buy, 2=sell)
        address counterparty;                // Transaction counterparty
        uint256 priceImpact;                 // Estimated price impact
        uint256 riskScore;                   // Risk score for this transaction
    }
    
    struct AccountRiskProfile {
        uint256 overallRiskScore;            // Overall risk score (0-1000)
        uint256 frequencyRisk;               // Frequency-based risk score
        uint256 volumeRisk;                  // Volume-based risk score  
        uint256 patternRisk;                 // Pattern-based risk score
        uint256 marketImpactRisk;            // Market impact risk score
        uint256 totalPenaltiesPaid;          // Total penalties paid
        uint256 flagCount;                   // Number of times flagged
        uint256 lastRiskUpdate;              // Last risk update timestamp
        bool isCircuitBreakerActive;         // Whether circuit breaker is active
        uint256 circuitBreakerUntil;         // Circuit breaker active until timestamp
        uint256 consecutiveHighRiskTxCount;  // Consecutive high-risk transactions
    }
    
    struct MarketManipulationDetection {
        uint256 suspiciousVolumeThreshold;   // Threshold for suspicious volume
        uint256 priceManipulationThreshold;  // Threshold for price manipulation
        uint256 frontRunningWindow;          // Time window for front-running detection
        uint256 washTradingWindow;           // Time window for wash trading detection
        bool coordinatedAttackDetection;     // Whether coordinated attack detection is enabled
    }
    
    struct MLModelParameters {
        uint256[10] featureWeights;          // Weights for ML features
        uint256 modelThreshold;              // Threshold for ML model predictions
        uint256 modelAccuracy;               // Current model accuracy (basis points)
        uint256 lastModelUpdate;             // Last model update timestamp
        bool isModelActive;                  // Whether ML model is active
    }
    
    // ============ State Variables ============
    
    AdvancedAntiSpeculationConfig public config;
    MarketManipulationDetection public manipulationConfig;
    MLModelParameters public mlModel;
    
    // Account tracking
    mapping(address => AccountRiskProfile) public accountRiskProfiles;
    mapping(address => TransactionPattern[]) public accountTransactionHistory;
    mapping(address => bool) public whitelistedAccounts;
    mapping(address => bool) public flaggedAccounts;
    
    // Pattern detection
    mapping(bytes32 => uint256) public patternSignatures; // Pattern hash -> risk score
    mapping(address => mapping(address => uint256)) public pairTransactionCounts; // A->B transaction counts
    
    // Global metrics
    uint256 public totalPenaltiesCollected;
    uint256 public totalTransactionsAnalyzed;
    uint256 public averageNetworkRiskScore;
    uint256 public marketManipulationDetections;
    uint256 public circuitBreakerActivations;
    
    // Emergency controls
    bool public emergencyMode;
    mapping(address => bool) public emergencyOperators;
    
    // Multi-dimensional risk factors
    mapping(address => uint256) public socialGraphRisk; // Risk based on transaction counterparties
    mapping(address => uint256) public behavioralRisk;  // Risk based on behavioral patterns
    mapping(address => uint256) public temporalRisk;    // Risk based on temporal patterns
    
    // ============ Events ============
    
    event RiskProfileUpdated(address indexed account, uint256 oldScore, uint256 newScore);
    event SpeculationPenaltyApplied(address indexed account, uint256 penalty, string reason);
    event CircuitBreakerActivated(address indexed account, uint256 duration);
    event MarketManipulationDetected(address indexed account, string manipulationType);
    event PatternDetected(address indexed account, bytes32 patternHash, uint256 riskScore);
    event MLModelUpdated(uint256 newAccuracy, uint256 threshold);
    event WhitelistStatusChanged(address indexed account, bool isWhitelisted);
    event ConfigurationUpdated(AdvancedAntiSpeculationConfig config);
    event EmergencyModeActivated(address operator, string reason);
    
    /**
     * @dev Constructor
     * @param initialOwner Contract owner
     */
    constructor(address initialOwner) Ownable(initialOwner) {
        config = AdvancedAntiSpeculationConfig({
            rapidTradeThreshold: 300,        // 5 minutes minimum between trades
            volumeConcentrationLimit: 2000,  // 20% maximum volume concentration
            patternDetectionSensitivity: 700, // 70% sensitivity
            basePenaltyRate: 100,            // 1% base penalty
            progressivePenaltyMultiplier: 150, // 1.5x multiplier
            circuitBreakerCooldown: 24 hours, // 24 hour cooldown
            whitelistGracePeriod: 7 days,    // 7 day grace period
            mlDetectionEnabled: false        // Disabled initially
        });
        
        manipulationConfig = MarketManipulationDetection({
            suspiciousVolumeThreshold: 5000,  // 50x normal volume
            priceManipulationThreshold: 500,  // 5% price impact
            frontRunningWindow: 60,           // 1 minute window
            washTradingWindow: 3600,          // 1 hour window
            coordinatedAttackDetection: true  // Enable coordinated attack detection
        });
        
        // Initialize ML model with default parameters
        mlModel = MLModelParameters({
            featureWeights: [uint256(100), uint256(150), uint256(200), uint256(125), uint256(175), uint256(100), uint256(150), uint256(125), uint256(100), uint256(175)],
            modelThreshold: 600,              // 60% threshold
            modelAccuracy: 8500,              // 85% accuracy
            lastModelUpdate: block.timestamp,
            isModelActive: false              // Disabled initially
        });
    }
    
    // ============ Core Analysis Functions ============
    
    /**
     * @dev Comprehensive transaction analysis with ML-based detection
     * @param account Account making transaction
     * @param amount Transaction amount
     * @param transactionType Type of transaction
     * @param counterparty Transaction counterparty (if applicable)
     * @return riskScore Comprehensive risk score (0-1000)
     * @return penalty Calculated penalty amount
     * @return flags Array of risk flags detected
     */
    function analyzeTransactionAdvanced(
        address account,
        uint256 amount,
        uint8 transactionType,
        address counterparty
    ) external returns (
        uint256 riskScore,
        uint256 penalty,
        string[] memory flags
    ) {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        if (whitelistedAccounts[account] && _isInGracePeriod(account)) {
            return (0, 0, new string[](0));
        }
        
        // Check circuit breaker status
        AccountRiskProfile storage profile = accountRiskProfiles[account];
        if (profile.isCircuitBreakerActive && block.timestamp < profile.circuitBreakerUntil) {
            string[] memory circuitBreakerFlags = new string[](1);
            circuitBreakerFlags[0] = "CIRCUIT_BREAKER_ACTIVE";
            return (MAX_RISK_SCORE, _calculateMaximumPenalty(amount), circuitBreakerFlags);
        }
        
        // Record transaction in history
        _recordTransactionPattern(account, amount, transactionType, counterparty);
        
        // Calculate multi-dimensional risk scores
        uint256 frequencyRisk = _calculateFrequencyRisk(account);
        uint256 volumeRisk = _calculateVolumeRisk(account, amount);
        uint256 patternRisk = _calculatePatternRisk(account);
        uint256 marketImpactRisk = _calculateMarketImpactRisk(account, amount);
        uint256 socialGraphRiskScore = _calculateSocialGraphRisk(account, counterparty);
        uint256 behavioralRiskScore = _calculateBehavioralRisk(account);
        uint256 temporalRiskScore = _calculateTemporalRisk(account);
        
        // Apply ML model if enabled
        uint256 mlRiskScore = 0;
        if (config.mlDetectionEnabled && mlModel.isModelActive) {
            mlRiskScore = _applyMLModel(account, amount, transactionType);
        }
        
        // Combine risk scores with weighted average
        riskScore = _calculateWeightedRiskScore(
            frequencyRisk,
            volumeRisk,
            patternRisk,
            marketImpactRisk,
            socialGraphRiskScore,
            behavioralRiskScore,
            temporalRiskScore,
            mlRiskScore
        );
        
        // Update account risk profile
        _updateAccountRiskProfile(account, riskScore, frequencyRisk, volumeRisk, patternRisk, marketImpactRisk);
        
        // Generate risk flags
        flags = _generateRiskFlags(account, riskScore);
        
        // Calculate penalty
        penalty = _calculateProgressivePenalty(account, amount, riskScore);
        
        // Check for market manipulation
        _detectMarketManipulation(account, amount, transactionType, counterparty);
        
        // Activate circuit breaker if necessary
        if (riskScore >= CIRCUIT_BREAKER_THRESHOLD) {
            _activateCircuitBreaker(account);
        }
        
        // Record transaction in history for future analysis
        accountTransactionHistory[account].push(TransactionPattern({
            timestamp: block.timestamp,
            amount: amount,
            transactionType: transactionType,
            counterparty: counterparty,
            priceImpact: 0, // Will be calculated separately
            riskScore: riskScore
        }));
        
        totalTransactionsAnalyzed++;
        averageNetworkRiskScore = ((averageNetworkRiskScore * (totalTransactionsAnalyzed - 1)) + riskScore) / totalTransactionsAnalyzed;
        
        if (penalty > 0) {
            totalPenaltiesCollected += penalty;
            profile.totalPenaltiesPaid += penalty;
            emit SpeculationPenaltyApplied(account, penalty, _getRiskReason(riskScore));
        }
        
        return (riskScore, penalty, flags);
    }
    
    /**
     * @dev Detect coordinated market manipulation attacks
     * @param accounts Array of accounts to analyze
     * @param timeWindow Time window for coordination detection
     * @return isCoordinated Whether coordinated activity was detected
     * @return riskScore Overall coordination risk score
     */
    function detectCoordinatedAttack(
        address[] calldata accounts,
        uint256 timeWindow
    ) external view returns (bool isCoordinated, uint256 riskScore) {
        require(manipulationConfig.coordinatedAttackDetection, "Coordinated attack detection disabled");
        
        if (accounts.length < 2) {
            return (false, 0);
        }
        
        uint256 correlationScore = 0;
        uint256 volumeCorrelation = 0;
        uint256 timingCorrelation = 0;
        
        // Analyze transaction timing correlations
        for (uint256 i = 0; i < accounts.length; i++) {
            for (uint256 j = i + 1; j < accounts.length; j++) {
                uint256 timingCorr = _calculateTimingCorrelation(accounts[i], accounts[j], timeWindow);
                uint256 volumeCorr = _calculateVolumeCorrelation(accounts[i], accounts[j], timeWindow);
                
                timingCorrelation += timingCorr;
                volumeCorrelation += volumeCorr;
            }
        }
        
        // Calculate overall correlation
        uint256 pairCount = (accounts.length * (accounts.length - 1)) / 2;
        timingCorrelation = timingCorrelation / pairCount;
        volumeCorrelation = volumeCorrelation / pairCount;
        
        correlationScore = (timingCorrelation + volumeCorrelation) / 2;
        
        isCoordinated = correlationScore >= 700; // 70% correlation threshold
        return (isCoordinated, correlationScore);
    }
    
    /**
     * @dev Update ML model parameters
     * @param newWeights New feature weights
     * @param newThreshold New model threshold
     * @param newAccuracy New model accuracy
     */
    function updateMLModel(
        uint256[10] calldata newWeights,
        uint256 newThreshold,
        uint256 newAccuracy
    ) external onlyOwner {
        require(newThreshold <= 1000, "Threshold too high");
        require(newAccuracy <= 10000, "Accuracy too high");
        
        mlModel.featureWeights = newWeights;
        mlModel.modelThreshold = newThreshold;
        mlModel.modelAccuracy = newAccuracy;
        mlModel.lastModelUpdate = block.timestamp;
        mlModel.isModelActive = true;
        
        emit MLModelUpdated(newAccuracy, newThreshold);
    }
    
    // ============ Configuration Functions ============
    
    /**
     * @dev Update anti-speculation configuration
     * @param newConfig New configuration parameters
     */
    function updateAntiSpeculationConfig(
        AdvancedAntiSpeculationConfig calldata newConfig
    ) external onlyOwner {
        require(newConfig.basePenaltyRate <= MAX_PENALTY_RATE, "Penalty rate too high");
        require(newConfig.progressivePenaltyMultiplier <= 500, "Multiplier too high"); // Max 5x
        require(newConfig.circuitBreakerCooldown >= 1 hours, "Cooldown too short");
        require(newConfig.circuitBreakerCooldown <= 7 days, "Cooldown too long");
        
        config = newConfig;
        emit ConfigurationUpdated(newConfig);
    }
    
    /**
     * @dev Update market manipulation detection configuration
     * @param newConfig New manipulation detection configuration
     */
    function updateManipulationConfig(
        MarketManipulationDetection calldata newConfig
    ) external onlyOwner {
        manipulationConfig = newConfig;
    }
    
    /**
     * @dev Set whitelist status for account
     * @param account Account address
     * @param whitelisted Whether to whitelist account
     */
    function setWhitelistStatus(address account, bool whitelisted) external onlyOwner {
        whitelistedAccounts[account] = whitelisted;
        emit WhitelistStatusChanged(account, whitelisted);
    }
    
    /**
     * @dev Manually flag/unflag account
     * @param account Account address
     * @param flagged Whether to flag account
     */
    function setAccountFlagged(address account, bool flagged) external onlyOwner {
        flaggedAccounts[account] = flagged;
        if (flagged) {
            accountRiskProfiles[account].flagCount++;
        }
    }
    
    // ============ Emergency Functions ============
    
    /**
     * @dev Activate emergency mode
     * @param reason Reason for activation
     */
    function activateEmergencyMode(string calldata reason) external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        
        emergencyMode = true;
        emit EmergencyModeActivated(msg.sender, reason);
    }
    
    /**
     * @dev Deactivate emergency mode
     */
    function deactivateEmergencyMode() external onlyOwner {
        emergencyMode = false;
    }
    
    /**
     * @dev Manually activate circuit breaker for account
     * @param account Account address
     * @param duration Circuit breaker duration
     */
    function manualCircuitBreaker(address account, uint256 duration) external onlyOwner {
        require(duration <= 7 days, "Duration too long");
        _activateCircuitBreaker(account, duration);
    }
    
    /**
     * @dev Deactivate circuit breaker for account
     * @param account Account address
     */
    function deactivateCircuitBreaker(address account) external onlyOwner {
        AccountRiskProfile storage profile = accountRiskProfiles[account];
        profile.isCircuitBreakerActive = false;
        profile.circuitBreakerUntil = 0;
    }
    
    // ============ View Functions ============
    
    function getAccountRiskProfile(address account) external view returns (AccountRiskProfile memory) {
        return accountRiskProfiles[account];
    }
    
    function getAccountTransactionHistory(address account) external view returns (TransactionPattern[] memory) {
        return accountTransactionHistory[account];
    }
    
    function getAntiSpeculationConfig() external view returns (AdvancedAntiSpeculationConfig memory) {
        return config;
    }
    
    function getManipulationConfig() external view returns (MarketManipulationDetection memory) {
        return manipulationConfig;
    }
    
    function getMLModelParameters() external view returns (MLModelParameters memory) {
        return mlModel;
    }
    
    function isAccountWhitelisted(address account) external view returns (bool) {
        return whitelistedAccounts[account];
    }
    
    function isAccountFlagged(address account) external view returns (bool) {
        return flaggedAccounts[account];
    }
    
    function getNetworkRiskMetrics() external view returns (
        uint256 avgRiskScore,
        uint256 totalPenalties,
        uint256 totalAnalyzed,
        uint256 manipulationDetections,
        uint256 circuitBreakerCount
    ) {
        return (
            averageNetworkRiskScore,
            totalPenaltiesCollected,
            totalTransactionsAnalyzed,
            marketManipulationDetections,
            circuitBreakerActivations
        );
    }
    
    // ============ Internal Functions ============
    
    function _recordTransactionPattern(
        address account,
        uint256 amount,
        uint8 transactionType,
        address counterparty
    ) internal {
        TransactionPattern memory pattern = TransactionPattern({
            timestamp: block.timestamp,
            amount: amount,
            transactionType: transactionType,
            counterparty: counterparty,
            priceImpact: _estimatePriceImpact(amount),
            riskScore: 0 // Will be set later
        });
        
        TransactionPattern[] storage history = accountTransactionHistory[account];
        history.push(pattern);
        
        // Keep only recent history for gas efficiency
        if (history.length > PATTERN_HISTORY_LENGTH) {
            // Remove oldest transaction
            for (uint256 i = 0; i < history.length - 1; i++) {
                history[i] = history[i + 1];
            }
            history.pop();
        }
        
        // Update pair transaction count
        if (counterparty != address(0)) {
            pairTransactionCounts[account][counterparty]++;
        }
    }
    
    function _calculateFrequencyRisk(address account) internal view returns (uint256) {
        TransactionPattern[] memory history = accountTransactionHistory[account];
        if (history.length == 0) return 0;
        
        uint256 recentTransactions = 0;
        uint256 currentTime = block.timestamp;
        
        // Count transactions in last hour
        for (uint256 i = 0; i < history.length; i++) {
            if (currentTime > history[i].timestamp && currentTime - history[i].timestamp <= 1 hours) {
                recentTransactions++;
            }
        }
        
        // Also count rapid transactions (less than 5 minutes apart or same timestamp)
        uint256 rapidTransactions = 0;
        if (history.length >= 2) {
            for (uint256 i = 1; i < history.length; i++) {
                // Same timestamp counts as rapid trading
                if (history[i].timestamp == history[i-1].timestamp) {
                    rapidTransactions++;
                } else if (history[i].timestamp > history[i-1].timestamp && 
                           history[i].timestamp - history[i-1].timestamp <= 300) { // 5 minutes
                    rapidTransactions++;
                }
            }
        }
        
        // Enhanced risk calculation - more aggressive detection
        uint256 frequencyRisk = 0;
        if (recentTransactions >= 5) frequencyRisk = 800;      // Very high risk (lowered threshold)
        else if (recentTransactions >= 3) frequencyRisk = 600; // High risk  
        else if (recentTransactions >= 2) frequencyRisk = 400; // Medium risk
        else if (recentTransactions >= 1) frequencyRisk = 200; // Low risk (added single tx risk)
        
        // Stronger rapid trading penalties
        uint256 rapidTradingPenalty = rapidTransactions * 300; // 300 risk points per rapid trade (tripled)
        
        uint256 totalRisk = frequencyRisk + rapidTradingPenalty;
        return totalRisk > MAX_RISK_SCORE ? MAX_RISK_SCORE : totalRisk;
    }
    
    function _calculateVolumeRisk(address account, uint256 currentAmount) internal view returns (uint256) {
        TransactionPattern[] memory history = accountTransactionHistory[account];
        
        // For accounts with no history, check if current amount is suspiciously large
        if (history.length == 0) {
            if (currentAmount >= 10000 * PRECISION) return 800; // Very large initial transaction
            if (currentAmount >= 5000 * PRECISION) return 400;  // Large initial transaction
            return 0;
        }
        
        uint256 totalVolume = 0;
        uint256 currentTime = block.timestamp;
        uint256 validTransactions = 0;
        
        // Calculate volume in last 24 hours
        for (uint256 i = 0; i < history.length; i++) {
            if (currentTime > history[i].timestamp && currentTime - history[i].timestamp <= 24 hours) {
                totalVolume += history[i].amount;
                validTransactions++;
            }
        }
        
        // If no recent transactions, use all history for average
        if (validTransactions == 0) {
            for (uint256 i = 0; i < history.length; i++) {
                totalVolume += history[i].amount;
                validTransactions++;
            }
        }
        
        if (validTransactions == 0) {
            // Fallback: check absolute volume thresholds
            if (currentAmount >= 50000 * PRECISION) return 900;
            if (currentAmount >= 10000 * PRECISION) return 700;
            if (currentAmount >= 5000 * PRECISION) return 400;
            return 0;
        }
        
        uint256 averageVolume = totalVolume / validTransactions;
        
        // Ensure we don't divide by zero
        if (averageVolume == 0) {
            averageVolume = 1 * PRECISION; // 1 token minimum average
        }
        
        // Enhanced volume concentration detection - more sensitive
        if (currentAmount > averageVolume * 5) return 900;    // Very high concentration (lowered from 10x)
        if (currentAmount > averageVolume * 3) return 700;    // High concentration (lowered from 5x)
        if (currentAmount > averageVolume * 2) return 500;    // Medium concentration (increased penalty)
        if (currentAmount > averageVolume) return 300;        // Above average (new threshold)
        return 0;                                              // Normal volume
    }
    
    function _calculatePatternRisk(address account) internal view returns (uint256) {
        TransactionPattern[] memory history = accountTransactionHistory[account];
        if (history.length < 5) return 0; // Need minimum history for pattern detection
        
        // Look for suspicious patterns
        uint256 patternRisk = 0;
        
        // Check for alternating buy/sell patterns (potential wash trading)
        uint256 alternatingCount = 0;
        for (uint256 i = 1; i < history.length; i++) {
            if (history[i].transactionType != history[i-1].transactionType) {
                alternatingCount++;
            }
        }
        
        uint256 alternatingRatio = (alternatingCount * 100) / (history.length - 1);
        if (alternatingRatio > 60) {
            patternRisk += 600; // High alternating pattern risk (lowered threshold, increased penalty)
        } else if (alternatingRatio > 40) {
            patternRisk += 300; // Medium alternating pattern risk (new tier)
        }
        
        // Check for same counterparty concentration
        if (history.length >= 3) {
            address mostFrequentCounterparty = address(0);
            uint256 maxCount = 0;
            
            for (uint256 i = 0; i < history.length; i++) {
                if (history[i].counterparty == address(0)) continue;
                
                uint256 count = 0;
                for (uint256 j = 0; j < history.length; j++) {
                    if (history[j].counterparty == history[i].counterparty) {
                        count++;
                    }
                }
                
                if (count > maxCount) {
                    maxCount = count;
                    mostFrequentCounterparty = history[i].counterparty;
                }
            }
            
            uint256 concentrationRatio = (maxCount * 100) / history.length;
            if (concentrationRatio > 50) {
                patternRisk += 500; // High counterparty concentration (lowered threshold, increased penalty)
            } else if (concentrationRatio > 30) {
                patternRisk += 250; // Medium counterparty concentration (new tier)
            }
        }
        
        return patternRisk > 1000 ? 1000 : patternRisk;
    }
    
    function _calculateMarketImpactRisk(address account, uint256 amount) internal view returns (uint256) {
        // Estimate price impact and assess risk
        uint256 priceImpact = _estimatePriceImpact(amount);
        
        if (priceImpact > manipulationConfig.priceManipulationThreshold) {
            return 800; // High market impact risk
        } else if (priceImpact > manipulationConfig.priceManipulationThreshold / 2) {
            return 400; // Medium market impact risk
        }
        
        return 0; // Low market impact risk
    }
    
    function _calculateSocialGraphRisk(address account, address counterparty) internal view returns (uint256) {
        if (counterparty == address(0)) return 0;
        
        // Check if counterparty is flagged
        if (flaggedAccounts[counterparty]) {
            return 300;
        }
        
        // Check transaction frequency with this counterparty
        uint256 pairCount = pairTransactionCounts[account][counterparty];
        if (pairCount > 10) return 200;
        if (pairCount > 5) return 100;
        
        return 0;
    }
    
    function _calculateBehavioralRisk(address account) internal view returns (uint256) {
        AccountRiskProfile memory profile = accountRiskProfiles[account];
        
        // Factor in historical behavior
        if (profile.consecutiveHighRiskTxCount > 5) return 600;
        if (profile.consecutiveHighRiskTxCount > 3) return 300;
        if (profile.flagCount > 3) return 200;
        
        return 0;
    }
    
    function _calculateTemporalRisk(address account) internal view returns (uint256) {
        TransactionPattern[] memory history = accountTransactionHistory[account];
        if (history.length < 3) return 0;
        
        // Look for unusual timing patterns
        uint256 intervals = 0;
        uint256 totalInterval = 0;
        
        for (uint256 i = 1; i < history.length; i++) {
            uint256 interval = history[i].timestamp - history[i-1].timestamp;
            totalInterval += interval;
            intervals++;
        }
        
        if (intervals == 0) return 0;
        
        uint256 averageInterval = totalInterval / intervals;
        
        // Check for too-regular intervals (possible bot behavior)
        uint256 regularityCount = 0;
        for (uint256 i = 1; i < history.length; i++) {
            uint256 interval = history[i].timestamp - history[i-1].timestamp;
            uint256 deviation = interval > averageInterval ? 
                interval - averageInterval : averageInterval - interval;
            
            if (deviation < averageInterval / 10) { // Within 10% of average
                regularityCount++;
            }
        }
        
        uint256 regularityRatio = (regularityCount * 100) / intervals;
        if (regularityRatio > 80) return 400; // Very regular timing
        if (regularityRatio > 60) return 200; // Somewhat regular timing
        
        return 0;
    }
    
    function _applyMLModel(
        address account,
        uint256 amount,
        uint8 transactionType
    ) internal view returns (uint256) {
        // Extract features for ML model
        uint256[10] memory features;
        
        TransactionPattern[] memory history = accountTransactionHistory[account];
        AccountRiskProfile memory profile = accountRiskProfiles[account];
        
        features[0] = history.length; // Transaction count
        features[1] = profile.totalPenaltiesPaid; // Historical penalties
        features[2] = amount; // Current transaction amount
        features[3] = uint256(transactionType); // Transaction type
        features[4] = profile.flagCount; // Flag count
        features[5] = profile.consecutiveHighRiskTxCount; // Consecutive high risk
        features[6] = block.timestamp - profile.lastRiskUpdate; // Time since last risk update
        features[7] = pairTransactionCounts[account][address(0)]; // Self-transactions
        features[8] = _estimatePriceImpact(amount); // Price impact
        features[9] = socialGraphRisk[account]; // Social graph risk
        
        // Apply ML model (simplified linear model)
        uint256 weightedSum = 0;
        for (uint256 i = 0; i < 10; i++) {
            weightedSum += features[i] * mlModel.featureWeights[i];
        }
        
        // Normalize to 0-1000 scale
        uint256 modelOutput = (weightedSum / 1000) % 1001;
        
        return modelOutput > mlModel.modelThreshold ? modelOutput : 0;
    }
    
    function _calculateWeightedRiskScore(
        uint256 frequencyRisk,
        uint256 volumeRisk,
        uint256 patternRisk,
        uint256 marketImpactRisk,
        uint256 socialGraphRiskScore,
        uint256 behavioralRiskScore,
        uint256 temporalRiskScore,
        uint256 mlRisk
    ) internal view returns (uint256) {
        // Weighted combination of risk factors
        uint256 totalWeight = 100;
        uint256 weightedSum = frequencyRisk * 20 +      // 20% weight
                              volumeRisk * 18 +          // 18% weight
                              patternRisk * 15 +         // 15% weight
                              marketImpactRisk * 12 +    // 12% weight
                              socialGraphRiskScore * 10 + // 10% weight
                              behavioralRiskScore * 10 +  // 10% weight
                              temporalRiskScore * 8 +     // 8% weight
                              mlRisk * 7;                 // 7% weight
        
        uint256 combinedScore = weightedSum / totalWeight;
        return combinedScore > MAX_RISK_SCORE ? MAX_RISK_SCORE : combinedScore;
    }
    
    function _updateAccountRiskProfile(
        address account,
        uint256 riskScore,
        uint256 frequencyRisk,
        uint256 volumeRisk,
        uint256 patternRisk,
        uint256 marketImpactRisk
    ) internal {
        AccountRiskProfile storage profile = accountRiskProfiles[account];
        
        uint256 oldRiskScore = profile.overallRiskScore;
        
        profile.overallRiskScore = riskScore;
        profile.frequencyRisk = frequencyRisk;
        profile.volumeRisk = volumeRisk;
        profile.patternRisk = patternRisk;
        profile.marketImpactRisk = marketImpactRisk;
        profile.lastRiskUpdate = block.timestamp;
        
        // Update consecutive high risk transaction count
        if (riskScore >= HIGH_RISK_THRESHOLD) {
            profile.consecutiveHighRiskTxCount++;
        } else {
            profile.consecutiveHighRiskTxCount = 0;
        }
        
        emit RiskProfileUpdated(account, oldRiskScore, riskScore);
    }
    
    function _generateRiskFlags(address account, uint256 riskScore) internal view returns (string[] memory flags) {
        AccountRiskProfile memory profile = accountRiskProfiles[account];
        
        // Count potential flags
        uint256 flagCount = 0;
        if (riskScore >= HIGH_RISK_THRESHOLD) flagCount++;
        if (profile.frequencyRisk >= HIGH_RISK_THRESHOLD) flagCount++;
        if (profile.volumeRisk >= HIGH_RISK_THRESHOLD) flagCount++;
        if (profile.patternRisk >= HIGH_RISK_THRESHOLD) flagCount++;
        if (profile.marketImpactRisk >= HIGH_RISK_THRESHOLD) flagCount++;
        if (profile.consecutiveHighRiskTxCount >= 3) flagCount++;
        if (flaggedAccounts[account]) flagCount++;
        
        flags = new string[](flagCount);
        uint256 index = 0;
        
        if (riskScore >= HIGH_RISK_THRESHOLD) flags[index++] = "HIGH_OVERALL_RISK";
        if (profile.frequencyRisk >= HIGH_RISK_THRESHOLD) flags[index++] = "HIGH_FREQUENCY_RISK";
        if (profile.volumeRisk >= HIGH_RISK_THRESHOLD) flags[index++] = "HIGH_VOLUME_RISK";
        if (profile.patternRisk >= HIGH_RISK_THRESHOLD) flags[index++] = "SUSPICIOUS_PATTERN";
        if (profile.marketImpactRisk >= HIGH_RISK_THRESHOLD) flags[index++] = "HIGH_MARKET_IMPACT";
        if (profile.consecutiveHighRiskTxCount >= 3) flags[index++] = "CONSECUTIVE_HIGH_RISK";
        if (flaggedAccounts[account]) flags[index++] = "MANUALLY_FLAGGED";
        
        return flags;
    }
    
    function _calculateProgressivePenalty(
        address account,
        uint256 amount,
        uint256 riskScore
    ) internal view returns (uint256) {
        if (emergencyMode) {
            return 0; // No penalties in emergency mode
        }
        
        if (riskScore < 300) {
            return 0; // No penalty for low risk (lowered threshold from 500 to 300)
        }
        
        AccountRiskProfile memory profile = accountRiskProfiles[account];
        
        // Base penalty calculation
        uint256 basePenalty = (amount * config.basePenaltyRate) / BASIS_POINTS;
        
        // Progressive multiplier based on risk score
        uint256 riskMultiplier = 100; // 1.0x base
        if (riskScore >= HIGH_RISK_THRESHOLD) {
            riskMultiplier = config.progressivePenaltyMultiplier * 2; // 3x for high risk
        } else {
            riskMultiplier = config.progressivePenaltyMultiplier; // 1.5x for medium risk
        }
        
        // Additional multiplier for repeat offenders
        uint256 repeatOffenderMultiplier = 100 + (profile.flagCount * 25); // +25% per flag
        
        // Calculate final penalty
        uint256 finalPenalty = (basePenalty * riskMultiplier * repeatOffenderMultiplier) / (100 * 100);
        
        // Cap penalty at maximum rate
        uint256 maxPenalty = (amount * MAX_PENALTY_RATE) / BASIS_POINTS;
        return finalPenalty > maxPenalty ? maxPenalty : finalPenalty;
    }
    
    function _calculateMaximumPenalty(uint256 amount) internal view returns (uint256) {
        return (amount * MAX_PENALTY_RATE) / BASIS_POINTS;
    }
    
    function _detectMarketManipulation(
        address account,
        uint256 amount,
        uint8 transactionType,
        address counterparty
    ) internal {
        // Check for wash trading
        if (_detectWashTrading(account, counterparty)) {
            marketManipulationDetections++;
            emit MarketManipulationDetected(account, "WASH_TRADING");
        }
        
        // Check for front running
        if (_detectFrontRunning(account, amount)) {
            marketManipulationDetections++;
            emit MarketManipulationDetected(account, "FRONT_RUNNING");
        }
        
        // Check for pump and dump
        if (_detectPumpAndDump(account, amount, transactionType)) {
            marketManipulationDetections++;
            emit MarketManipulationDetected(account, "PUMP_AND_DUMP");
        }
    }
    
    function _detectWashTrading(address account, address counterparty) internal view returns (bool) {
        if (counterparty == address(0)) return false;
        
        // Check reciprocal trading patterns
        uint256 aToB = pairTransactionCounts[account][counterparty];
        uint256 bToA = pairTransactionCounts[counterparty][account];
        
        // Require minimum transaction count for wash trading detection
        if (aToB >= 2 && bToA >= 2) {
            uint256 minTrades = aToB < bToA ? aToB : bToA;
            uint256 maxTrades = aToB > bToA ? aToB : bToA;
            
            // If trades are too similar in count, it's suspicious
            uint256 ratio = (minTrades * 100) / maxTrades;
            if (ratio >= 60) { // Within 40% of each other
                return true;
            }
        }
        
        // Also check recent transaction patterns
        TransactionPattern[] memory history = accountTransactionHistory[account];
        if (history.length >= 4) {
            uint256 backAndForthCount = 0;
            for (uint256 i = 1; i < history.length; i++) {
                if (history[i].counterparty == counterparty && 
                    history[i-1].counterparty == counterparty &&
                    block.timestamp - history[i].timestamp <= 1 hours) {
                    backAndForthCount++;
                }
            }
            
            if (backAndForthCount >= 3) {
                return true; // Rapid back-and-forth trading
            }
        }
        
        return false;
    }
    
    function _detectFrontRunning(address account, uint256 amount) internal view returns (bool) {
        TransactionPattern[] memory history = accountTransactionHistory[account];
        if (history.length < 2) return false;
        
        // Look for large transactions followed quickly by opposite transactions
        uint256 lastTxIndex = history.length - 1;
        TransactionPattern memory lastTx = history[lastTxIndex];
        
        if (block.timestamp - lastTx.timestamp <= manipulationConfig.frontRunningWindow) {
            if (lastTx.amount > amount * 2 && lastTx.transactionType != uint8(block.timestamp % 3)) {
                return true;
            }
        }
        
        return false;
    }
    
    function _detectPumpAndDump(address account, uint256 amount, uint8 transactionType) internal view returns (bool) {
        TransactionPattern[] memory history = accountTransactionHistory[account];
        if (history.length < 5) return false;
        
        // Look for patterns of accumulation followed by large sells
        uint256 buyCount = 0;
        uint256 sellCount = 0;
        uint256 totalBuyVolume = 0;
        uint256 totalSellVolume = 0;
        
        uint256 currentTime = block.timestamp;
        for (uint256 i = 0; i < history.length; i++) {
            if (currentTime - history[i].timestamp <= 24 hours) {
                if (history[i].transactionType == 1) { // Buy
                    buyCount++;
                    totalBuyVolume += history[i].amount;
                } else if (history[i].transactionType == 2) { // Sell
                    sellCount++;
                    totalSellVolume += history[i].amount;
                }
            }
        }
        
        // Pattern: Many small buys followed by large sell
        if (buyCount >= 5 && sellCount <= 2 && transactionType == 2) {
            uint256 averageBuy = buyCount > 0 ? totalBuyVolume / buyCount : 0;
            return amount > averageBuy * 5; // Current sell > 5x average buy
        }
        
        return false;
    }
    
    function _activateCircuitBreaker(address account) internal {
        _activateCircuitBreaker(account, config.circuitBreakerCooldown);
    }
    
    function _activateCircuitBreaker(address account, uint256 duration) internal {
        AccountRiskProfile storage profile = accountRiskProfiles[account];
        profile.isCircuitBreakerActive = true;
        profile.circuitBreakerUntil = block.timestamp + duration;
        
        circuitBreakerActivations++;
        emit CircuitBreakerActivated(account, duration);
    }
    
    function _isInGracePeriod(address account) internal view returns (bool) {
        return whitelistedAccounts[account] && 
               block.timestamp < accountRiskProfiles[account].lastRiskUpdate + config.whitelistGracePeriod;
    }
    
    function _estimatePriceImpact(uint256 amount) internal pure returns (uint256) {
        // Simplified price impact estimation
        // In practice, this would integrate with AMM or order book
        if (amount > 100000 * PRECISION) return 500; // 5%
        if (amount > 50000 * PRECISION) return 300;  // 3%
        if (amount > 10000 * PRECISION) return 100;  // 1%
        return 50; // 0.5%
    }
    
    function _calculateTimingCorrelation(
        address account1,
        address account2,
        uint256 timeWindow
    ) internal view returns (uint256) {
        // Simplified timing correlation calculation
        // Would need more sophisticated implementation in practice
        return 0; // Placeholder
    }
    
    function _calculateVolumeCorrelation(
        address account1,
        address account2,
        uint256 timeWindow
    ) internal view returns (uint256) {
        // Simplified volume correlation calculation
        // Would need more sophisticated implementation in practice
        return 0; // Placeholder
    }
    
    function _getRiskReason(uint256 riskScore) internal pure returns (string memory) {
        if (riskScore >= 900) return "EXTREMELY_HIGH_RISK";
        if (riskScore >= HIGH_RISK_THRESHOLD) return "HIGH_RISK_TRANSACTION";
        if (riskScore >= MEDIUM_RISK_THRESHOLD) return "MEDIUM_RISK_TRANSACTION";
        return "LOW_RISK_TRANSACTION";
    }
    
    /**
     * @dev Get account flags for external inspection
     * @param account Account to check flags for
     * @return flags Array of flag strings set for the account
     */
    function getAccountFlags(address account) external view returns (string[] memory flags) {
        AccountRiskProfile memory profile = accountRiskProfiles[account];
        
        // Count active flags first
        uint256 activeFlags = 0;
        
        if (profile.overallRiskScore >= HIGH_RISK_THRESHOLD) activeFlags++;
        if (profile.flagCount > 5) activeFlags++;
        if (profile.isCircuitBreakerActive) activeFlags++;
        if (profile.consecutiveHighRiskTxCount >= 3) activeFlags++;
        
        // Create array with correct size
        string[] memory flagsArray = new string[](activeFlags);
        uint256 index = 0;
        
        // Add flags
        if (profile.overallRiskScore >= HIGH_RISK_THRESHOLD) {
            flagsArray[index++] = "HIGH_RISK";
        }
        
        if (profile.flagCount > 5) {
            flagsArray[index++] = "FREQUENT_TRADER";
        }
        
        if (profile.isCircuitBreakerActive) {
            flagsArray[index++] = "CIRCUIT_BREAKER";
        }
        
        if (profile.consecutiveHighRiskTxCount >= 3) {
            flagsArray[index++] = "WASH_TRADING";
        }
        
        return flagsArray;
    }

    /**
     * @dev Add emergency operator
     */
    function addEmergencyOperator(address operator) external onlyOwner {
        emergencyOperators[operator] = true;
    }
    
    /**
     * @dev Remove emergency operator
     */
    function removeEmergencyOperator(address operator) external onlyOwner {
        emergencyOperators[operator] = false;
    }
    
    /**
     * @dev Pause contract operations
     */
    function pause() external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        _pause();
    }
    
    /**
     * @dev Unpause contract operations
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}