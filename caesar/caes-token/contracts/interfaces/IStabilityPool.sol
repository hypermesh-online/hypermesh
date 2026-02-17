// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title IStabilityPool
 * @dev Interface for the automated stability pool with penalty collection,
 * reserve management, and emergency intervention mechanisms
 */
interface IStabilityPool {
    
    // ============ Structs ============
    
    struct ReserveState {
        uint256 balance;        // Reserve balance on this chain
        uint256 lastUpdate;     // Last update timestamp
        bool isActive;          // Whether chain is active
    }
    
    struct AMMIntervention {
        bool executed;          // Whether intervention was executed
        TradeType tradeType;    // Type of trade executed
        uint256 amountIn;       // Amount input to trade
        uint256 amountOut;      // Amount received from trade
        uint256 timestamp;      // Timestamp of intervention
    }
    
    struct RebalanceOperation {
        bool executed;          // Whether operation was executed
        uint32 sourceChain;     // Source chain ID
        uint32 targetChain;     // Target chain ID
        uint256 amount;         // Amount transferred
        uint256 timestamp;      // Timestamp of operation
    }
    
    struct StabilityMetrics {
        uint256 stabilityIndex;     // Current stability index (0-1000)
        uint256 reserveRatio;       // Current reserve ratio (basis points)
        uint256 interventionCount;  // Number of interventions executed
        uint256 lastUpdate;         // Last metrics update timestamp
        bool emergencyMode;         // Whether in emergency mode
    }
    
    struct PoolComposition {
        uint256 totalBalance;       // Total pool balance
        uint256 penaltyFunds;       // Funds from speculation penalties
        uint256 demurrageFunds;     // Funds from demurrage collection
        uint256 reserveFunds;       // Reserve backing funds
        uint256 emergencyFunds;     // Emergency intervention funds
    }
    
    // ============ Enums ============
    
    enum FundType {
        PENALTY,
        DEMURRAGE,
        RESERVE,
        EMERGENCY
    }
    
    enum TradeType {
        NO_TRADE,
        BUY,
        SELL
    }
    
    enum InterventionType {
        AMM_TRADE,
        RESERVE_INJECTION,
        SUPPLY_ADJUSTMENT,
        EMERGENCY_PAUSE
    }
    
    enum EmergencyType {
        PAUSE_TRADING,
        CIRCUIT_BREAKER,
        RESERVE_INJECTION,
        SUPPLY_ADJUSTMENT
    }
    
    // ============ Events ============
    
    event FundsReceived(FundType fundType, uint256 amount, address from);
    event StabilityIntervention(InterventionType intervention, uint256 amount, uint256 newStability);
    event ReserveRebalanced(uint32 chainId, uint256 amount, bool isAddition);
    event EmergencyIntervention(string reason, uint256 amount);
    event AMMTradeExecuted(bool isBuy, uint256 amountIn, uint256 amountOut);
    event StabilityIndexUpdated(uint256 oldIndex, uint256 newIndex);
    
    // ============ Fund Reception Functions ============
    
    /**
     * @dev Receive penalty funds from anti-speculation engine
     * @param amount Amount of penalty received
     * @param account Account that was penalized
     */
    function receivePenalty(uint256 amount, address account) external;
    
    /**
     * @dev Receive demurrage funds from demurrage system
     * @param amount Amount of demurrage received
     * @param account Account that paid demurrage
     */
    function receiveDemurrage(uint256 amount, address account) external;
    
    /**
     * @dev Contribute reserves to the pool
     * @param amount Amount to contribute
     */
    function contributeReserves(uint256 amount) external;
    
    // ============ Stability Functions ============
    
    /**
     * @dev Execute automated market maker intervention
     * @param currentPrice Current CAESAR token price
     * @param targetPrice Target price (1 USD)
     * @return intervention Details of intervention executed
     */
    function executeAMMIntervention(
        uint256 currentPrice,
        uint256 targetPrice
    ) external returns (AMMIntervention memory intervention);
    
    /**
     * @dev Rebalance reserves across supported chains
     * @param targetChain Chain to rebalance with
     * @return operation Details of rebalancing operation
     */
    function rebalanceChainReserves(uint32 targetChain) external returns (RebalanceOperation memory operation);
    
    /**
     * @dev Execute emergency intervention
     * @param reason Reason for emergency
     * @param interventionType Type of intervention
     * @return success Whether intervention succeeded
     */
    function executeEmergencyIntervention(
        string calldata reason,
        EmergencyType interventionType
    ) external returns (bool success);
    
    // ============ View Functions ============
    
    /**
     * @dev Calculate current reserve ratio
     * @return ratio Current reserve ratio (basis points)
     */
    function calculateReserveRatio() external view returns (uint256 ratio);
    
    /**
     * @dev Get stability metrics
     * @return metrics Current stability metrics
     */
    function getStabilityMetrics() external view returns (StabilityMetrics memory metrics);
    
    /**
     * @dev Get pool composition
     * @return composition Current pool fund composition
     */
    function getPoolComposition() external view returns (PoolComposition memory composition);
    
    /**
     * @dev Get chain reserve state
     * @param chainId Chain ID
     * @return state Reserve state for chain
     */
    function getChainReserveState(uint32 chainId) external view returns (ReserveState memory state);
    
    // ============ Administrative Functions ============
    
    /**
     * @dev Set AMM router address
     * @param ammRouter AMM router address
     */
    function setAMMRouter(address ammRouter) external;
    
    /**
     * @dev Add supported chain
     * @param chainId Chain ID to add
     * @param poolAddress Pool address on target chain
     */
    function addSupportedChain(uint32 chainId, address poolAddress) external;
    
    /**
     * @dev Remove supported chain
     * @param chainId Chain ID to remove
     */
    function removeSupportedChain(uint32 chainId) external;
    
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
     * @dev Pause pool operations
     */
    function pause() external;
    
    /**
     * @dev Unpause pool operations
     */
    function unpause() external;
}