// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";

/**
 * @title DynamicEconomicsOracle
 * @dev Real-time gas fee and reward calculation based on network-matrix liquidity/volatility
 * 
 * Implements dynamic economic adjustments to achieve self-stabilization through:
 * - Liquidity health analysis across network matrix
 * - Volatility-based fee scaling
 * - Merit-based reward distribution
 * - Cross-chain economic coordination
 */
contract DynamicEconomicsOracle is Ownable, ReentrancyGuard, Pausable {
    
    // Economic configuration parameters
    struct EconomicConfig {
        uint256 baseFee;                    // Base transaction fee (in wei)
        uint256 maxFeeMultiplier;           // Maximum fee multiplier (default: 10x)
        uint256 hostRewardPercentage;       // Percentage of fees for routing hosts (default: 70%)
        uint256 liquidityPoolPercentage;    // Percentage for liquidity pool (default: 20%)
        uint256 reservePercentage;          // Percentage for reserve fund (default: 10%)
        uint256 crossChainBonusRate;        // Cross-chain routing bonus rate
        uint256 performanceDecayRate;       // Rate at which performance scores decay
        bool emergencyMode;                 // Emergency fee adjustment mode
    }
    
    // Network liquidity health metrics
    struct LiquidityMetrics {
        uint256 liquidityHealthIndex;       // L(t) - Current liquidity health (0-1000)
        uint256 activeParticipants;         // Number of active network participants
        uint256 targetParticipants;         // Target participant count
        uint256 dailyVolume;                // 24h transaction volume
        uint256 targetVolume;               // Target daily volume
        uint256 stabilityReserve;           // Current stability reserve
        uint256 requiredReserve;            // Required reserve amount
        uint256 lastUpdate;                 // Last metrics update timestamp
    }
    
    // Market pressure and volatility indicators
    struct VolatilityMetrics {
        uint256 marketPressure;             // |current_price - target_price| / target_price * 1000
        uint256 priceDeviation;             // Price deviation from target
        uint256 transactionVolume;          // Current transaction volume
        uint256 targetTransactionVolume;    // Target transaction volume
        uint256 crossChainTransfers;        // Cross-chain transfer count
        uint256 targetCrossChainTransfers;  // Target cross-chain transfers
        uint256 volatilityScore;            // Overall volatility score (0-1000)
        uint256 lastUpdate;                 // Last update timestamp
    }
    
    // Network health assessment per chain
    struct NetworkHealth {
        address networkAddress;             // Network/chain identifier
        uint256 settlementRate;             // Success rate (0-1000, where 1000 = 100%)
        uint256 averageLatency;             // Average transaction latency (ms)
        uint256 capacityUtilization;       // Network capacity usage (0-1000)
        uint256 qualityScore;               // Overall network quality (0-1000)
        bool isActive;                      // Network active status
        uint256 lastHealthCheck;            // Last health assessment
    }
    
    // Economic calculation results
    struct EconomicParameters {
        uint256 dynamicGasFee;              // Calculated gas fee
        uint256 hostReward;                 // Reward per routing host
        uint256 performanceMultiplier;      // Performance-based multiplier
        uint256 crossChainBonus;            // Cross-chain routing bonus
        uint256 rateLimitThreshold;         // Current rate limiting threshold
        uint256 rearouteThreshold;          // Threshold for cross-chain rerouting
    }
    
    // State variables
    EconomicConfig public economicConfig;
    LiquidityMetrics public liquidityMetrics;
    VolatilityMetrics public volatilityMetrics;
    
    mapping(address => NetworkHealth) public networkHealth;
    mapping(address => uint256) public hostPerformanceScores;
    mapping(address => uint256) public hostTotalRewards;
    
    address[] public monitoredNetworks;
    address[] public activeNodes;
    bytes32[] public pendingValidations;
    // Define a NetworkNode struct for nodes mapping
    struct NetworkNode {
        bool isActive;
        uint256 successRate;
        uint256 avgLatency;
    }
    mapping(address => NetworkNode) public nodes;
    uint256 public totalNetworks;
    
    // Economic formulas constants (from formulas.py)
    uint256 constant SCALE_FACTOR = 1000;           // 1000 = 100% for calculations
    uint256 constant MIN_LIQUIDITY_HEALTH = 100;    // 10% minimum liquidity health
    uint256 constant EMERGENCY_THRESHOLD = 200;     // 20% emergency threshold
    uint256 constant HALT_THRESHOLD = 100;          // 10% halt threshold
    uint256 constant MAX_MARKET_PRESSURE = 500;     // 50% maximum market pressure
    
    // Events
    event EconomicParametersUpdated(uint256 newGasFee, uint256 newHostReward, uint256 timestamp);
    event LiquidityMetricsUpdated(uint256 healthIndex, uint256 marketPressure, uint256 timestamp);
    event NetworkHealthUpdated(address indexed network, uint256 qualityScore, uint256 timestamp);
    event EmergencyModeActivated(string reason, uint256 timestamp);
    event RewardDistributed(address indexed host, uint256 amount, uint256 performanceScore);
    event RateLimitThresholdUpdated(uint256 newThreshold, string reason);
    event CrossChainRerouteTriggered(address fromNetwork, address toNetwork, string reason);
    
    constructor(address owner) Ownable(owner) {
        // Initialize default economic configuration
        economicConfig = EconomicConfig({
            baseFee: 0.001 ether,           // 0.001 ETH base fee
            maxFeeMultiplier: 10000,        // 10x maximum multiplier (10000 = 10.0)
            hostRewardPercentage: 700,      // 70% to routing hosts
            liquidityPoolPercentage: 200,   // 20% to liquidity pool
            reservePercentage: 100,         // 10% to reserve
            crossChainBonusRate: 50,        // 5% cross-chain bonus
            performanceDecayRate: 50,       // 5% daily performance decay
            emergencyMode: false
        });
        
        // Initialize liquidity metrics with neutral values
        liquidityMetrics = LiquidityMetrics({
            liquidityHealthIndex: 700,      // Start with 70% health
            activeParticipants: 0,
            targetParticipants: 100,        // Target 100 participants
            dailyVolume: 0,
            targetVolume: 10 ether,         // Target 10 ETH daily volume
            stabilityReserve: 0,
            requiredReserve: 1 ether,       // Require 1 ETH reserve
            lastUpdate: block.timestamp
        });
        
        // Initialize volatility metrics
        volatilityMetrics = VolatilityMetrics({
            marketPressure: 0,
            priceDeviation: 0,
            transactionVolume: 0,
            targetTransactionVolume: 100,   // Target 100 transactions
            crossChainTransfers: 0,
            targetCrossChainTransfers: 10,  // Target 10 cross-chain transfers
            volatilityScore: 0,
            lastUpdate: block.timestamp
        });
    }
    
    /**
     * @dev Calculate dynamic gas fee based on current network conditions
     * Formula: Dynamic_Gas_Fee = base_fee * (1 + Market_Pressure) * sqrt(Transaction_Volume / Target_Volume) * (1/Liquidity_Health_Index)
     */
    function calculateDynamicGasFee() external view returns (uint256) {
        uint256 baseFee = economicConfig.baseFee;
        
        // Market pressure adjustment (1 + Market_Pressure)
        uint256 pressureMultiplier = SCALE_FACTOR + volatilityMetrics.marketPressure;
        
        // Volume scaling factor: sqrt(Transaction_Volume / Target_Volume)
        uint256 volumeRatio = (volatilityMetrics.transactionVolume * SCALE_FACTOR) / volatilityMetrics.targetTransactionVolume;
        uint256 volumeMultiplier = _sqrt(volumeRatio);
        
        // Liquidity health adjustment: (1 / Liquidity_Health_Index)
        uint256 liquidityDivisor = liquidityMetrics.liquidityHealthIndex > 0 ? 
            liquidityMetrics.liquidityHealthIndex : MIN_LIQUIDITY_HEALTH;
        uint256 liquidityMultiplier = (SCALE_FACTOR * SCALE_FACTOR) / liquidityDivisor;
        
        // Calculate final fee
        uint256 dynamicFee = (baseFee * pressureMultiplier * volumeMultiplier * liquidityMultiplier) / 
                            (SCALE_FACTOR * SCALE_FACTOR * SCALE_FACTOR);
        
        // Apply emergency mode caps
        if (economicConfig.emergencyMode || liquidityMetrics.liquidityHealthIndex < EMERGENCY_THRESHOLD) {
            uint256 maxFee = (baseFee * economicConfig.maxFeeMultiplier) / SCALE_FACTOR;
            if (dynamicFee > maxFee) {
                dynamicFee = maxFee;
            }
        }
        
        return dynamicFee;
    }
    
    /**
     * @dev Calculate host reward based on performance and contribution
     * Formula: Host_Reward = (Transaction_Fee * Host_Percentage) * Performance_Multiplier + Cross_Chain_Bonus
     */
    function calculateHostReward(
        address host, 
        uint256 transactionFee, 
        bool isCrossChain
    ) external view returns (uint256) {
        // Base reward calculation
        uint256 baseReward = (transactionFee * economicConfig.hostRewardPercentage) / SCALE_FACTOR;
        
        // Performance multiplier based on host's historical performance
        uint256 performanceScore = hostPerformanceScores[host];
        if (performanceScore == 0) {
            performanceScore = 500; // Default neutral performance (50%)
        }
        
        uint256 performanceMultiplier = _calculatePerformanceMultiplier(performanceScore);
        uint256 adjustedReward = (baseReward * performanceMultiplier) / SCALE_FACTOR;
        
        // Add cross-chain bonus if applicable
        if (isCrossChain) {
            uint256 crossChainBonus = (baseReward * economicConfig.crossChainBonusRate) / SCALE_FACTOR;
            adjustedReward += crossChainBonus;
        }
        
        return adjustedReward;
    }
    
    /**
     * @dev Update liquidity health metrics
     * Formula: Liquidity_Health_Index = min(active_participants/target_participants, daily_volume/target_volume, stability_reserve/required_reserve)
     */
    function updateLiquidityMetrics(
        uint256 activeParticipants,
        uint256 dailyVolume,
        uint256 stabilityReserve
    ) external onlyOwner {
        // Calculate individual health ratios
        uint256 participantRatio = activeParticipants > liquidityMetrics.targetParticipants ? 
            SCALE_FACTOR : (activeParticipants * SCALE_FACTOR) / liquidityMetrics.targetParticipants;
            
        uint256 volumeRatio = dailyVolume > liquidityMetrics.targetVolume ?
            SCALE_FACTOR : (dailyVolume * SCALE_FACTOR) / liquidityMetrics.targetVolume;
            
        uint256 reserveRatio = stabilityReserve > liquidityMetrics.requiredReserve ?
            SCALE_FACTOR : (stabilityReserve * SCALE_FACTOR) / liquidityMetrics.requiredReserve;
        
        // Take minimum as overall health index
        uint256 healthIndex = participantRatio;
        if (volumeRatio < healthIndex) healthIndex = volumeRatio;
        if (reserveRatio < healthIndex) healthIndex = reserveRatio;
        
        // Update metrics
        liquidityMetrics.liquidityHealthIndex = healthIndex;
        liquidityMetrics.activeParticipants = activeParticipants;
        liquidityMetrics.dailyVolume = dailyVolume;
        liquidityMetrics.stabilityReserve = stabilityReserve;
        liquidityMetrics.lastUpdate = block.timestamp;
        
        // Check for emergency conditions
        if (healthIndex < HALT_THRESHOLD) {
            economicConfig.emergencyMode = true;
            emit EmergencyModeActivated("Liquidity health below halt threshold", block.timestamp);
        } else if (healthIndex < EMERGENCY_THRESHOLD && !economicConfig.emergencyMode) {
            economicConfig.emergencyMode = true;
            emit EmergencyModeActivated("Liquidity health in emergency zone", block.timestamp);
        } else if (healthIndex > EMERGENCY_THRESHOLD * 2 && economicConfig.emergencyMode) {
            economicConfig.emergencyMode = false;
        }
        
        emit LiquidityMetricsUpdated(healthIndex, volatilityMetrics.marketPressure, block.timestamp);
    }
    
    /**
     * @dev Update market volatility metrics
     * Formula: Market_Pressure = |current_price - target_price| / target_price
     */
    function updateVolatilityMetrics(
        uint256 currentPrice,
        uint256 targetPrice,
        uint256 transactionVolume,
        uint256 crossChainTransfers
    ) external onlyOwner {
        // Calculate market pressure
        uint256 priceDeviation = currentPrice > targetPrice ? 
            currentPrice - targetPrice : targetPrice - currentPrice;
        uint256 marketPressure = (priceDeviation * SCALE_FACTOR) / targetPrice;
        
        // Cap market pressure at maximum
        if (marketPressure > MAX_MARKET_PRESSURE) {
            marketPressure = MAX_MARKET_PRESSURE;
        }
        
        // Calculate network utility score
        uint256 transactionRatio = (transactionVolume * SCALE_FACTOR) / volatilityMetrics.targetTransactionVolume;
        uint256 crossChainRatio = (crossChainTransfers * SCALE_FACTOR) / volatilityMetrics.targetCrossChainTransfers;
        uint256 networkUtilityScore = (transactionRatio + crossChainRatio) / 2;
        
        // Update volatility metrics
        volatilityMetrics.marketPressure = marketPressure;
        volatilityMetrics.priceDeviation = priceDeviation;
        volatilityMetrics.transactionVolume = transactionVolume;
        volatilityMetrics.crossChainTransfers = crossChainTransfers;
        volatilityMetrics.volatilityScore = (marketPressure + (SCALE_FACTOR - networkUtilityScore)) / 2;
        volatilityMetrics.lastUpdate = block.timestamp;
        
        emit LiquidityMetricsUpdated(liquidityMetrics.liquidityHealthIndex, marketPressure, block.timestamp);
    }
    
    /**
     * @dev Update network health for a specific chain
     */
    function updateNetworkHealth(
        address network,
        uint256 settlementRate,
        uint256 averageLatency,
        uint256 capacityUtilization
    ) external onlyOwner {
        // Calculate quality score based on metrics
        uint256 qualityScore = _calculateNetworkQualityScore(
            settlementRate,
            averageLatency,
            capacityUtilization
        );
        
        // Update network health
        NetworkHealth storage health = networkHealth[network];
        if (health.networkAddress == address(0)) {
            // New network
            monitoredNetworks.push(network);
            totalNetworks++;
            health.networkAddress = network;
            health.isActive = true;
        }
        
        health.settlementRate = settlementRate;
        health.averageLatency = averageLatency;
        health.capacityUtilization = capacityUtilization;
        health.qualityScore = qualityScore;
        health.lastHealthCheck = block.timestamp;
        
        emit NetworkHealthUpdated(network, qualityScore, block.timestamp);
    }
    
    /**
     * @dev Calculate rate limiting threshold based on current conditions
     * Formula: Rate_Limit = base_limit * Liquidity_Health_Index * (1 / sqrt(Market_Pressure + 1))
     */
    function calculateRateLimitThreshold() external view returns (uint256) {
        uint256 baseLimit = 100; // Base rate limit (100 transactions per unit time)
        
        // Liquidity health factor
        uint256 liquidityFactor = liquidityMetrics.liquidityHealthIndex;
        
        // Market pressure adjustment
        uint256 pressureAdjustment = _sqrt(volatilityMetrics.marketPressure + SCALE_FACTOR);
        uint256 pressureFactor = (SCALE_FACTOR * SCALE_FACTOR) / pressureAdjustment;
        
        // Calculate final threshold
        uint256 threshold = (baseLimit * liquidityFactor * pressureFactor) / (SCALE_FACTOR * SCALE_FACTOR);
        
        return threshold;
    }
    
    /**
     * @dev Determine if cross-chain rerouting should be triggered
     */
    function shouldTriggerReroute(address network) external view returns (bool, address) {
        NetworkHealth memory health = networkHealth[network];
        
        // Check if network quality is below threshold
        if (health.qualityScore < 300 || health.capacityUtilization > 900) { // 30% quality or 90% capacity
            // Find alternative network
            address bestAlternative = _findBestAlternativeNetwork(network);
            if (bestAlternative != address(0)) {
                return (true, bestAlternative);
            }
        }
        
        return (false, address(0));
    }
    
    /**
     * @dev Update host performance score
     */
    function updateHostPerformance(
        address host,
        bool success,
        uint256 latency,
        uint256 expectedLatency
    ) external onlyOwner {
        uint256 currentScore = hostPerformanceScores[host];
        if (currentScore == 0) {
            currentScore = 500; // Start with neutral score
        }
        
        // Performance adjustment based on success and latency
        int256 adjustment = 0;
        
        if (success) {
            adjustment += 10; // +10 for successful routing
            if (latency < expectedLatency) {
                adjustment += int256((expectedLatency - latency) * 5 / expectedLatency); // Bonus for better latency
            }
        } else {
            adjustment -= 50; // -50 for failed routing
        }
        
        // Apply adjustment with bounds checking
        int256 newScore = int256(currentScore) + adjustment;
        if (newScore < 0) newScore = 0;
        if (newScore > 1000) newScore = 1000;
        
        hostPerformanceScores[host] = uint256(newScore);
    }
    
    /**
     * @dev Get network health metrics
     */
    function getNetworkHealth() external view returns (
        uint256 activeNodeCount,
        uint256 avgSuccessRate,
        uint256 avgLatency,
        uint256 totalValidations,
        uint256 emergencyModeStatus
    ) {
        activeNodeCount = activeNodes.length;
        
        uint256 totalSuccessRate = 0;
        uint256 totalLatency = 0;
        uint256 activeCount = 0;
        
        for (uint256 i = 0; i < activeNodes.length; i++) {
            if (nodes[activeNodes[i]].isActive) {
                totalSuccessRate += nodes[activeNodes[i]].successRate;
                totalLatency += nodes[activeNodes[i]].avgLatency;
                activeCount++;
            }
        }
        
        avgSuccessRate = activeCount > 0 ? totalSuccessRate / activeCount : 0;
        avgLatency = activeCount > 0 ? totalLatency / activeCount : 0;
        totalValidations = pendingValidations.length;
        emergencyModeStatus = economicConfig.emergencyMode ? 1 : 0;
    }

    /**
     * @dev Get comprehensive economic parameters for current conditions
     */
    function getEconomicParameters() external view returns (EconomicParameters memory) {
        return EconomicParameters({
            dynamicGasFee: this.calculateDynamicGasFee(),
            hostReward: 0, // Calculated per transaction
            performanceMultiplier: 0, // Calculated per host
            crossChainBonus: economicConfig.crossChainBonusRate,
            rateLimitThreshold: this.calculateRateLimitThreshold(),
            rearouteThreshold: 300 // 30% network quality threshold
        });
    }
    
    // Internal helper functions
    
    /**
     * @dev Calculate performance multiplier based on score
     * Range: 50% to 200% (0.5x to 2.0x)
     */
    function _calculatePerformanceMultiplier(uint256 performanceScore) internal pure returns (uint256) {
        // Linear scaling: 0 score = 0.5x, 500 score = 1.0x, 1000 score = 2.0x
        return 500 + (performanceScore * 1500) / 1000; // Range: 500-2000 (0.5x-2.0x)
    }
    
    /**
     * @dev Calculate network quality score
     */
    function _calculateNetworkQualityScore(
        uint256 settlementRate,
        uint256 averageLatency,
        uint256 capacityUtilization
    ) internal pure returns (uint256) {
        // Settlement rate weight: 50%
        uint256 settlementScore = (settlementRate * 500) / SCALE_FACTOR;
        
        // Latency score weight: 30% (lower latency = higher score)
        uint256 maxLatency = 10000; // 10 seconds max
        uint256 latencyScore = averageLatency > maxLatency ? 0 : 
            ((maxLatency - averageLatency) * 300) / maxLatency;
        
        // Capacity utilization weight: 20% (optimal at 70%)
        uint256 optimalCapacity = 700;
        uint256 capacityDiff = capacityUtilization > optimalCapacity ?
            capacityUtilization - optimalCapacity : optimalCapacity - capacityUtilization;
        uint256 capacityScore = capacityDiff > optimalCapacity ? 0 :
            ((optimalCapacity - capacityDiff) * 200) / optimalCapacity;
        
        return settlementScore + latencyScore + capacityScore;
    }
    
    /**
     * @dev Find best alternative network for rerouting
     */
    function _findBestAlternativeNetwork(address excludeNetwork) internal view returns (address) {
        address bestNetwork = address(0);
        uint256 bestScore = 0;
        
        for (uint256 i = 0; i < monitoredNetworks.length; i++) {
            address network = monitoredNetworks[i];
            if (network != excludeNetwork && networkHealth[network].isActive) {
                uint256 score = networkHealth[network].qualityScore;
                if (score > bestScore) {
                    bestScore = score;
                    bestNetwork = network;
                }
            }
        }
        
        return bestNetwork;
    }
    
    /**
     * @dev Square root approximation using Babylonian method
     */
    function _sqrt(uint256 x) internal pure returns (uint256) {
        if (x == 0) return 0;
        
        uint256 z = (x + 1) / 2;
        uint256 y = x;
        
        while (z < y) {
            y = z;
            z = (x / z + z) / 2;
        }
        
        return y;
    }
    
    /**
     * @dev Emergency functions (owner only)
     */
    function activateEmergencyMode(string calldata reason) external onlyOwner {
        economicConfig.emergencyMode = true;
        emit EmergencyModeActivated(reason, block.timestamp);
    }
    
    function deactivateEmergencyMode() external onlyOwner {
        economicConfig.emergencyMode = false;
    }
    
    function updateEconomicConfig(EconomicConfig calldata newConfig) external onlyOwner {
        economicConfig = newConfig;
    }
    
    /**
     * @dev Receive function for fee collection
     */
    receive() external payable {
        // Fees collected for economic calculations
    }
}