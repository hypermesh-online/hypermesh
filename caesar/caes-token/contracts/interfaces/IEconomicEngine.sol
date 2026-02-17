// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title IEconomicEngine
 * @dev Interface for the comprehensive economic model implementation
 * Defines all functions for demurrage mechanisms, anti-speculation systems,
 * and mathematical stability frameworks for Caesar Token
 */
interface IEconomicEngine {
    
    // ============ Structs ============
    
    struct EconomicParameters {
        uint256 baseDemurrageRate;      // Base demurrage rate (basis points per hour)
        uint256 maxDemurrageRate;       // Maximum demurrage rate (basis points per hour)
        uint256 stabilityThreshold;     // Price stability threshold (basis points)
        uint256 fiatDiscountFactor;     // Maximum fiat activity discount (basis points)
        uint256 gracePeriodsHours;      // Grace period for new users (hours)
        uint256 interventionThreshold;  // Price deviation threshold for intervention
        uint256 rebalanceFrequency;     // Frequency of reserve rebalancing (seconds)
        uint256 emergencyThreshold;     // Emergency intervention threshold
    }
    
    struct MarketMetrics {
        uint256 totalDemurrageCollected;    // Total demurrage collected
        uint256 totalPenaltiesCollected;    // Total speculation penalties
        uint256 averageHoldingPeriod;       // Average token holding period
        uint256 networkVelocity;            // Money velocity metric
        uint256 participationScore;         // Overall network participation
        uint256 lastMetricUpdate;           // Last metric update timestamp
    }
    
    struct SystemHealth {
        uint256 priceStability;         // Price stability score (0-1000)
        uint256 liquidityHealth;        // Liquidity health score (0-1000)  
        uint256 participationRate;      // Active participation rate (0-1000)
        uint256 reserveRatio;           // Reserve backing ratio (0-2000)
        uint256 lastUpdate;             // Last health update timestamp
    }
    
    struct AccountEconomicData {
        uint256 lastActivity;               // Last activity timestamp
        uint256 lastDemurrageApplication;   // Last demurrage application time
        uint256 totalDemurragePaid;         // Total demurrage paid
        uint256 totalPenaltiesPaid;         // Total penalties paid
        uint256 transactionCount;           // Number of transactions
        uint256 totalVolume;                // Total transaction volume
        uint256 lastTransactionTime;        // Last transaction timestamp
        uint256 riskScore;                  // Current risk score (0-1000)
        bool isNewUser;                     // Whether user is in grace period
        uint256 graceEndTime;               // When grace period ends
    }
    
    struct FiatActivityData {
        uint256 totalFiatVolume;        // Total fiat activity volume
        uint256 lastFiatActivity;       // Last fiat activity timestamp
        uint256 activityCount;          // Number of fiat activities
        bool discountEligible;          // Whether eligible for discount
        uint256 discountPercentage;     // Current discount percentage
    }
    
    struct ChainEconomicState {
        EconomicParameters parameters;   // Economic parameters for this chain
        uint256 totalSupply;            // Total supply on this chain
        uint256 reserveAmount;          // Reserve amount on this chain
        uint256 lastSync;               // Last synchronization time
        bytes32 syncHash;               // Hash of last sync data
    }
    
    struct ParameterProposal {
        EconomicParameters proposedParams; // Proposed new parameters
        address proposer;                   // Address of proposer
        uint256 proposalTime;              // Proposal timestamp
        uint256 votes;                     // Number of votes
        bool executed;                     // Whether proposal was executed
    }
    
    struct HealthMetrics {
        uint256 overallHealth;          // Overall system health (0-1000)
        uint256 priceStability;         // Price stability component
        uint256 liquidityHealth;        // Liquidity component
        uint256 participationRate;      // Participation component
        uint256 lastUpdate;             // Last update timestamp
    }
    
    struct EconomicHealthMetrics {
        uint256 overallHealth;                      // Overall health score
        uint256 priceStability;                     // Price stability score
        uint256 liquidityHealth;                    // Liquidity health score
        uint256 participationRate;                  // Participation rate score
        uint256 reserveRatio;                       // Reserve ratio score
        uint256 demurrageEfficiency;                // Demurrage system efficiency
        uint256 antiSpeculationEffectiveness;      // Anti-speculation effectiveness
        uint256 systemStress;                       // System stress level
        uint256 timestamp;                          // Timestamp of metrics
    }
    
    // ============ Enums ============
    
    enum StabilityAction {
        NO_ACTION,
        INCREASE_SUPPLY,
        DECREASE_SUPPLY,
        ADJUST_RATES,
        EMERGENCY_INTERVENTION
    }
    
    enum InterventionType {
        SUPPLY_INCREASE,
        SUPPLY_DECREASE,
        RATE_ADJUSTMENT,
        RESERVE_ADDITION,
        RESERVE_REMOVAL,
        EMERGENCY_PAUSE
    }
    
    enum ReserveOperationType {
        ADD_RESERVES,
        REMOVE_RESERVES,
        REBALANCE_CHAINS,
        EMERGENCY_WITHDRAWAL
    }
    
    struct ReserveOperation {
        ReserveOperationType operationType;
        uint256 amount;
        uint32 targetChain;
        uint256 timestamp;
        bool success;
    }
    
    // ============ Events ============
    
    event EconomicParametersUpdated(EconomicParameters params);
    event DemurrageApplied(address indexed account, uint256 amount, uint256 rate);
    event SpeculationPenaltyApplied(address indexed account, uint256 penalty, string reason);
    event StabilityIntervention(InterventionType interventionType, uint256 amount);
    event HealthMetricsUpdated(EconomicHealthMetrics metrics);
    event EmergencyModeActivated(address operator, string reason);
    event ParameterProposalCreated(bytes32 indexed proposalId, address proposer);
    event CrossChainSync(uint32 indexed chainId, bytes32 stateHash);
    event FiatActivityRecorded(address indexed account, uint256 amount, uint8 activityType);
    event ReserveOperationExecuted(ReserveOperationType operationType, uint256 amount, bool success);
    
    // ============ Core Economic Functions ============
    
    /**
     * @dev Calculate demurrage amount with stability adjustments and fiat discounts
     * @param account Account address
     * @param balance Current balance
     * @return demurrageAmount Calculated demurrage amount
     */
    function calculateDemurrage(address account, uint256 balance) external view returns (uint256 demurrageAmount);
    
    /**
     * @dev Apply demurrage to account with mathematical precision
     * @param account Account to apply demurrage to
     * @param balance Current account balance  
     * @return appliedAmount Amount of demurrage applied
     */
    function applyDemurrage(address account, uint256 balance) external returns (uint256 appliedAmount);
    
    /**
     * @dev Analyze transaction patterns for speculation detection
     * @param account Account making transaction
     * @param amount Transaction amount
     * @param transactionType Type of transaction (0=transfer, 1=buy, 2=sell)
     * @return riskScore Risk score from 0-1000
     * @return penalty Calculated penalty amount
     */
    function analyzeTransactionPattern(
        address account,
        uint256 amount,
        uint8 transactionType
    ) external returns (uint256 riskScore, uint256 penalty);
    
    /**
     * @dev Apply speculation penalty with progressive scaling
     * @param account Account to penalize
     * @param penalty Penalty amount
     * @return success Whether penalty was applied successfully
     */
    function applySpeculationPenalty(address account, uint256 penalty) external returns (bool success);
    
    /**
     * @dev Maintain 1:1 USD peg through automated interventions
     * @return action Type of stability action taken
     */
    function maintainPegStability() external returns (StabilityAction action);
    
    /**
     * @dev Rebalance reserves across chains and stability pools
     * @return operation Details of rebalancing operation performed
     */
    function rebalanceReserves() external returns (ReserveOperation memory operation);
    
    /**
     * @dev Update economic parameters through governance
     * @param params New economic parameters
     */
    function updateEconomicParameters(EconomicParameters calldata params) external;
    
    /**
     * @dev Monitor and update real-time economic health metrics
     * @return metrics Current health metrics
     */
    function monitorEconomicHealth() external returns (HealthMetrics memory metrics);
    
    // ============ Fiat Integration Functions ============
    
    /**
     * @dev Record fiat activity for demurrage discount calculation
     * @param account Account with fiat activity
     * @param amount Amount of fiat activity
     * @param activityType Type of fiat activity (0=purchase, 1=redemption, 2=payment)
     */
    function recordFiatActivity(address account, uint256 amount, uint8 activityType) external;
    
    /**
     * @dev Validate fiat backing in real-time with Stripe integration
     * @param expectedBacking Expected backing amount
     * @return isValid Whether backing is valid
     * @return actualBacking Actual backing amount
     */
    function validateFiatBacking(uint256 expectedBacking) external view returns (bool isValid, uint256 actualBacking);
    
    // ============ Cross-Chain Functions ============
    
    /**
     * @dev Synchronize economic parameters across LayerZero V2 chains
     * @param chainId Target chain ID
     * @param parameters Parameters to synchronize
     */
    function synchronizeParameters(uint32 chainId, EconomicParameters calldata parameters) external;
    
    // ============ View Functions ============
    
    /**
     * @dev Get current economic parameters
     * @return Current economic parameters
     */
    function getEconomicParameters() external view returns (EconomicParameters memory);
    
    /**
     * @dev Get current system health metrics
     * @return Current system health
     */
    function getSystemHealth() external view returns (SystemHealth memory);
    
    /**
     * @dev Get comprehensive health metrics
     * @return Current health metrics
     */
    function getHealthMetrics() external view returns (EconomicHealthMetrics memory);
    
    /**
     * @dev Get account economic data
     * @param account Account address
     * @return Account economic data
     */
    function getAccountEconomicData(address account) external view returns (AccountEconomicData memory);
    
    /**
     * @dev Get fiat activity data for account
     * @param account Account address  
     * @return Fiat activity data
     */
    function getFiatActivityData(address account) external view returns (FiatActivityData memory);
    
    /**
     * @dev Get economic state for specific chain
     * @param chainId Chain ID
     * @return Chain economic state
     */
    function getChainEconomicState(uint32 chainId) external view returns (ChainEconomicState memory);
    
    /**
     * @dev Check if account is exempt from economic mechanisms
     * @param account Account address
     * @return Whether account is exempt
     */
    function isAccountExempt(address account) external view returns (bool);
    
    /**
     * @dev Get current demurrage rate for account
     * @param account Account address
     * @return Current demurrage rate (basis points per hour)
     */
    function getCurrentDemurrageRate(address account) external view returns (uint256);
    
    /**
     * @dev Get current speculation risk score for account
     * @param account Account address
     * @return Current risk score (0-1000)
     */
    function getSpeculationRiskScore(address account) external view returns (uint256);
    
    /**
     * @dev Get market metrics
     * @return Current market metrics
     */
    function getMarketMetrics() external view returns (MarketMetrics memory);
    
    // ============ Administrative Functions ============
    
    /**
     * @dev Set account exemption status
     * @param account Account address
     * @param exempt Whether to exempt account
     */
    function setAccountExemption(address account, bool exempt) external;
    
    /**
     * @dev Add emergency operator
     * @param operator Operator address
     */
    function addEmergencyOperator(address operator) external;
    
    /**
     * @dev Remove emergency operator
     * @param operator Operator address  
     */
    function removeEmergencyOperator(address operator) external;
    
    /**
     * @dev Activate emergency mode
     * @param reason Reason for activation
     */
    function activateEmergencyMode(string calldata reason) external;
    
    /**
     * @dev Deactivate emergency mode
     */
    function deactivateEmergencyMode() external;
    
    /**
     * @dev Pause contract operations
     */
    function pause() external;
    
    /**
     * @dev Unpause contract operations
     */
    function unpause() external;
}