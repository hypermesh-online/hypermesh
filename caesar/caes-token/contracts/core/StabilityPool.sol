// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "../libs/AdvancedMathUtils.sol";
import "../interfaces/IStabilityPool.sol";
import "../oracles/GoldPriceOracle.sol";

/**
 * @title StabilityPool  
 * @dev Automated stability pool with penalty collection, reserve management,
 * and emergency intervention mechanisms for Caesar Token economic stability
 */
contract StabilityPool is Ownable, ReentrancyGuard, Pausable, IStabilityPool {
    using SafeERC20 for IERC20;
    using AdvancedMathUtils for uint256;
    
    // ============ Constants ============
    uint256 public constant PRECISION = 1e18;
    uint256 public constant BASIS_POINTS = 10000;
    uint256 public constant MAX_RESERVE_RATIO = 2000; // 200% maximum
    uint256 public constant MIN_RESERVE_RATIO = 800;  // 80% minimum  
    uint256 public constant TARGET_RESERVE_RATIO = 1000; // 100% target
    uint256 public constant REBALANCE_THRESHOLD = 50; // 5% deviation threshold
    
    // ============ State Variables ============
    IERC20 public immutable gateToken;
    IERC20 public immutable usdcToken;
    
    // Pool composition
    uint256 public totalPoolBalance;
    uint256 public penaltyFunds;
    uint256 public demurrageFunds;
    uint256 public reserveFunds;
    uint256 public emergencyFunds;
    
    // Reserve management
    mapping(uint32 => ReserveState) public chainReserves;
    uint256 public totalReserves;
    uint256 public lastRebalance;
    uint256 public rebalanceFrequency = 1 hours;
    
    // Stability metrics
    uint256 public stabilityIndex;
    uint256 public lastStabilityUpdate;
    uint256 public interventionCount;
    
    // Emergency controls
    bool public emergencyMode;
    uint256 public emergencyThreshold = 1000; // 10% deviation
    mapping(address => bool) public emergencyOperators;
    
    // Multi-chain support
    mapping(uint32 => bool) public supportedChains;
    mapping(uint32 => address) public chainPoolAddresses;
    
    // AMM integration
    address public ammRouter;
    uint256 public maxSlippage = 300; // 3%
    
    // Events are defined in IStabilityPool interface
    
    /**
     * @dev Constructor
     * @param initialOwner Contract owner
     * @param _gateToken CAESAR token address
     * @param _usdcToken USDC token address
     */
    constructor(
        address initialOwner,
        address _gateToken,
        address _usdcToken
    ) Ownable(initialOwner) {
        gateToken = IERC20(_gateToken);
        usdcToken = IERC20(_usdcToken);
        
        stabilityIndex = 1000; // Perfect stability initially
        lastStabilityUpdate = block.timestamp;
        lastRebalance = block.timestamp;
    }
    
    // ============ Fund Reception Functions ============
    
    /**
     * @dev Receive penalty funds from anti-speculation engine
     * @param amount Amount of penalty received
     * @param account Account that was penalized
     */
    function receivePenalty(uint256 amount, address account) external override nonReentrant {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        penaltyFunds += amount;
        totalPoolBalance += amount;
        
        // Update stability based on penalty collection
        _updateStabilityIndex();
        
        emit FundsReceived(FundType.PENALTY, amount, account);
    }
    
    /**
     * @dev Receive demurrage funds from demurrage system
     * @param amount Amount of demurrage received
     * @param account Account that paid demurrage
     */
    function receiveDemurrage(uint256 amount, address account) external override nonReentrant {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        demurrageFunds += amount;
        totalPoolBalance += amount;
        
        // Update stability metrics
        _updateStabilityIndex();
        
        emit FundsReceived(FundType.DEMURRAGE, amount, account);
    }
    
    /**
     * @dev Contribute reserves to the pool
     * @param amount Amount to contribute
     */
    function contributeReserves(uint256 amount) external override nonReentrant whenNotPaused {
        require(amount > 0, "Amount must be positive");
        
        usdcToken.safeTransferFrom(msg.sender, address(this), amount);
        
        reserveFunds += amount;
        totalReserves += amount;
        totalPoolBalance += amount;
        
        emit FundsReceived(FundType.RESERVE, amount, msg.sender);
    }
    
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
    ) external override returns (AMMIntervention memory intervention) {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        require(ammRouter != address(0), "AMM router not set");
        
        uint256 priceDeviation = _calculatePriceDeviation(currentPrice, targetPrice);
        
        if (priceDeviation < REBALANCE_THRESHOLD) {
            return AMMIntervention({
                executed: false,
                tradeType: TradeType.NO_TRADE,
                amountIn: 0,
                amountOut: 0,
                timestamp: block.timestamp
            });
        }
        
        if (currentPrice > targetPrice) {
            // Price too high, sell GATE for USDC
            intervention = _executeSellPressure(currentPrice, targetPrice, priceDeviation);
        } else {
            // Price too low, buy GATE with USDC
            intervention = _executeBuyPressure(currentPrice, targetPrice, priceDeviation);
        }
        
        interventionCount++;
        _updateStabilityIndex();
        
        return intervention;
    }
    
    /**
     * @dev Rebalance reserves across supported chains
     * @param targetChain Chain to rebalance with
     * @return operation Details of rebalancing operation
     */
    function rebalanceChainReserves(uint32 targetChain) external override returns (RebalanceOperation memory operation) {
        require(supportedChains[targetChain], "Chain not supported");
        require(block.timestamp >= lastRebalance + rebalanceFrequency, "Rebalancing too frequent");
        
        ReserveState storage localState = chainReserves[0]; // Local chain
        ReserveState storage targetState = chainReserves[targetChain];
        
        // Calculate optimal distribution
        uint256 totalChainReserves = localState.balance + targetState.balance;
        uint256 optimalLocalAmount = totalChainReserves / 2;
        
        if (localState.balance > optimalLocalAmount + (optimalLocalAmount * REBALANCE_THRESHOLD / BASIS_POINTS)) {
            // Send excess to target chain
            uint256 excessAmount = localState.balance - optimalLocalAmount;
            operation = _transferToChain(targetChain, excessAmount);
        } else if (localState.balance < optimalLocalAmount - (optimalLocalAmount * REBALANCE_THRESHOLD / BASIS_POINTS)) {
            // Request from target chain
            uint256 deficitAmount = optimalLocalAmount - localState.balance;
            operation = _requestFromChain(targetChain, deficitAmount);
        }
        
        lastRebalance = block.timestamp;
        return operation;
    }
    
    /**
     * @dev Execute emergency intervention
     * @param reason Reason for emergency
     * @param interventionType Type of intervention
     * @return success Whether intervention succeeded
     */
    function executeEmergencyIntervention(
        string calldata reason,
        EmergencyType interventionType
    ) external override returns (bool success) {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        
        if (interventionType == EmergencyType.PAUSE_TRADING) {
            _pause();
            success = true;
        } else if (interventionType == EmergencyType.CIRCUIT_BREAKER) {
            emergencyMode = true;
            success = true;
        } else if (interventionType == EmergencyType.RESERVE_INJECTION) {
            success = _executeReserveInjection();
        } else if (interventionType == EmergencyType.SUPPLY_ADJUSTMENT) {
            success = _executeSupplyAdjustment();
        }
        
        if (success) {
            emit EmergencyIntervention(reason, 0);
        }
        
        return success;
    }
    
    // ============ Reserve Management ============
    
    /**
     * @dev Calculate current reserve ratio
     * @return ratio Current reserve ratio (basis points)
     */
    function calculateReserveRatio() external view override returns (uint256 ratio) {
        uint256 totalSupply = _getTotalSupply();
        return totalSupply > 0 ? (totalReserves * BASIS_POINTS) / totalSupply : BASIS_POINTS;
    }
    
    /**
     * @dev Get stability metrics
     * @return metrics Current stability metrics
     */
    function getStabilityMetrics() external view override returns (StabilityMetrics memory metrics) {
        return StabilityMetrics({
            stabilityIndex: stabilityIndex,
            reserveRatio: this.calculateReserveRatio(),
            interventionCount: interventionCount,
            lastUpdate: lastStabilityUpdate,
            emergencyMode: emergencyMode
        });
    }
    
    /**
     * @dev Get pool composition
     * @return composition Current pool fund composition
     */
    function getPoolComposition() external view override returns (PoolComposition memory composition) {
        return PoolComposition({
            totalBalance: totalPoolBalance,
            penaltyFunds: penaltyFunds,
            demurrageFunds: demurrageFunds,
            reserveFunds: reserveFunds,
            emergencyFunds: emergencyFunds
        });
    }
    
    /**
     * @dev Get chain reserve state
     * @param chainId Chain ID
     * @return state Reserve state for chain
     */
    function getChainReserveState(uint32 chainId) external view returns (ReserveState memory state) {
        return chainReserves[chainId];
    }
    
    // ============ Administrative Functions ============
    
    /**
     * @dev Set AMM router address
     * @param _ammRouter AMM router address
     */
    function setAMMRouter(address _ammRouter) external onlyOwner {
        require(_ammRouter != address(0), "Invalid router address");
        ammRouter = _ammRouter;
    }
    
    /**
     * @dev Add supported chain
     * @param chainId Chain ID to add
     * @param poolAddress Pool address on target chain
     */
    function addSupportedChain(uint32 chainId, address poolAddress) external onlyOwner {
        supportedChains[chainId] = true;
        chainPoolAddresses[chainId] = poolAddress;
        
        // Initialize chain state
        chainReserves[chainId] = ReserveState({
            balance: 0,
            lastUpdate: block.timestamp,
            isActive: true
        });
    }
    
    /**
     * @dev Remove supported chain
     * @param chainId Chain ID to remove
     */
    function removeSupportedChain(uint32 chainId) external onlyOwner {
        supportedChains[chainId] = false;
        chainReserves[chainId].isActive = false;
    }
    
    /**
     * @dev Add emergency operator
     * @param operator Operator address
     */
    function addEmergencyOperator(address operator) external onlyOwner {
        emergencyOperators[operator] = true;
    }
    
    /**
     * @dev Remove emergency operator
     * @param operator Operator address
     */
    function removeEmergencyOperator(address operator) external onlyOwner {
        emergencyOperators[operator] = false;
    }
    
    /**
     * @dev Set rebalance frequency
     * @param frequency New frequency in seconds
     */
    function setRebalanceFrequency(uint256 frequency) external onlyOwner {
        require(frequency >= 10 minutes, "Frequency too low");
        require(frequency <= 24 hours, "Frequency too high");
        rebalanceFrequency = frequency;
    }
    
    /**
     * @dev Set emergency threshold
     * @param threshold New threshold in basis points
     */
    function setEmergencyThreshold(uint256 threshold) external onlyOwner {
        require(threshold <= 2000, "Threshold too high"); // Max 20%
        emergencyThreshold = threshold;
    }
    
    /**
     * @dev Withdraw emergency funds (owner only)
     * @param amount Amount to withdraw
     * @param recipient Recipient address
     */
    function withdrawEmergencyFunds(uint256 amount, address recipient) external onlyOwner {
        require(amount <= emergencyFunds, "Insufficient emergency funds");
        require(recipient != address(0), "Invalid recipient");
        
        emergencyFunds -= amount;
        totalPoolBalance -= amount;
        
        usdcToken.safeTransfer(recipient, amount);
    }
    
    // ============ Internal Functions ============
    
    function _updateStabilityIndex() internal {
        uint256 reserveRatio = this.calculateReserveRatio();
        uint256 currentPrice = _getCurrentPrice();
        uint256 targetPrice = PRECISION; // 1 USD
        
        uint256 priceStability = _calculatePriceStability(currentPrice, targetPrice);
        uint256 reserveStability = _calculateReserveStability(reserveRatio);
        
        // Weighted average: 60% price stability, 40% reserve stability
        stabilityIndex = (priceStability * 60 + reserveStability * 40) / 100;
        lastStabilityUpdate = block.timestamp;
        
        emit StabilityIndexUpdated(stabilityIndex, stabilityIndex);
    }
    
    function _calculatePriceDeviation(uint256 currentPrice, uint256 targetPrice) internal pure returns (uint256) {
        if (currentPrice >= targetPrice) {
            return ((currentPrice - targetPrice) * BASIS_POINTS) / targetPrice;
        } else {
            return ((targetPrice - currentPrice) * BASIS_POINTS) / targetPrice;
        }
    }
    
    function _executeSellPressure(
        uint256 currentPrice,
        uint256 targetPrice,
        uint256 deviation
    ) internal returns (AMMIntervention memory intervention) {
        uint256 sellAmount = _calculateInterventionAmount(deviation);
        
        if (sellAmount > gateToken.balanceOf(address(this))) {
            sellAmount = gateToken.balanceOf(address(this));
        }
        
        if (sellAmount > 0) {
            // Execute sell through AMM (simplified)
            uint256 usdcReceived = _simulateAMMTrade(sellAmount, true);
            
            intervention = AMMIntervention({
                executed: true,
                tradeType: TradeType.SELL,
                amountIn: sellAmount,
                amountOut: usdcReceived,
                timestamp: block.timestamp
            });
            
            emit AMMTradeExecuted(false, sellAmount, usdcReceived);
        }
        
        return intervention;
    }
    
    function _executeBuyPressure(
        uint256 currentPrice,
        uint256 targetPrice,
        uint256 deviation
    ) internal returns (AMMIntervention memory intervention) {
        uint256 buyAmount = _calculateInterventionAmount(deviation);
        
        if (buyAmount > usdcToken.balanceOf(address(this))) {
            buyAmount = usdcToken.balanceOf(address(this));
        }
        
        if (buyAmount > 0) {
            // Execute buy through AMM (simplified)
            uint256 gateReceived = _simulateAMMTrade(buyAmount, false);
            
            intervention = AMMIntervention({
                executed: true,
                tradeType: TradeType.BUY,
                amountIn: buyAmount,
                amountOut: gateReceived,
                timestamp: block.timestamp
            });
            
            emit AMMTradeExecuted(true, buyAmount, gateReceived);
        }
        
        return intervention;
    }
    
    function _calculateInterventionAmount(uint256 deviation) internal view returns (uint256) {
        // Calculate intervention size based on deviation and available funds
        uint256 maxIntervention = totalPoolBalance / 20; // Max 5% of pool
        uint256 deviationFactor = deviation * PRECISION / BASIS_POINTS;
        
        return (maxIntervention * deviationFactor) / PRECISION;
    }
    
    function _simulateAMMTrade(uint256 amountIn, bool isSell) internal pure returns (uint256 amountOut) {
        // Simplified AMM simulation - in production this would call actual AMM
        if (isSell) {
            amountOut = (amountIn * 99) / 100; // 1% slippage
        } else {
            amountOut = (amountIn * 101) / 100; // 1% bonus
        }
        return amountOut;
    }
    
    function _transferToChain(uint32 chainId, uint256 amount) internal returns (RebalanceOperation memory operation) {
        chainReserves[0].balance -= amount;
        chainReserves[chainId].balance += amount;
        
        emit ReserveRebalanced(chainId, amount, true);
        
        return RebalanceOperation({
            executed: true,
            sourceChain: 0,
            targetChain: chainId,
            amount: amount,
            timestamp: block.timestamp
        });
    }
    
    function _requestFromChain(uint32 chainId, uint256 amount) internal returns (RebalanceOperation memory operation) {
        chainReserves[chainId].balance -= amount;
        chainReserves[0].balance += amount;
        
        emit ReserveRebalanced(chainId, amount, false);
        
        return RebalanceOperation({
            executed: true,
            sourceChain: chainId,
            targetChain: 0,
            amount: amount,
            timestamp: block.timestamp
        });
    }
    
    function _executeReserveInjection() internal returns (bool) {
        uint256 injectionAmount = emergencyFunds / 2; // Use half of emergency funds
        if (injectionAmount > 0) {
            reserveFunds += injectionAmount;
            emergencyFunds -= injectionAmount;
            totalReserves += injectionAmount;
            return true;
        }
        return false;
    }
    
    function _executeSupplyAdjustment() internal returns (bool) {
        // This would interact with the main token contract to adjust supply
        // For now, return success as placeholder
        return true;
    }
    
    function _getCurrentPrice() internal view returns (uint256) {
        // This would integrate with price oracles
        return PRECISION; // Placeholder: 1 USD
    }
    
    function _getTotalSupply() internal view returns (uint256) {
        return gateToken.totalSupply();
    }
    
    function _calculatePriceStability(uint256 currentPrice, uint256 targetPrice) internal pure returns (uint256) {
        uint256 deviation = _calculatePriceDeviation(currentPrice, targetPrice);
        return deviation > 1000 ? 0 : 1000 - deviation;
    }
    
    function _calculateReserveStability(uint256 reserveRatio) internal pure returns (uint256) {
        if (reserveRatio >= MIN_RESERVE_RATIO && reserveRatio <= MAX_RESERVE_RATIO) {
            // Calculate distance from target
            uint256 deviation = reserveRatio > TARGET_RESERVE_RATIO ?
                reserveRatio - TARGET_RESERVE_RATIO :
                TARGET_RESERVE_RATIO - reserveRatio;
            
            return deviation > 200 ? 0 : 1000 - (deviation * 5); // Scale deviation
        }
        return 0; // Outside acceptable range
    }
    
    /**
     * @dev Emergency pause function
     */
    function pause() external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        _pause();
    }
    
    /**
     * @dev Emergency unpause function
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}