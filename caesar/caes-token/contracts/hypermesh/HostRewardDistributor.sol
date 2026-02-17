// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./DynamicEconomicsOracle.sol";
import "./HypermeshNetworkManager.sol";

/**
 * @title HostRewardDistributor
 * @dev Merit-based reward distribution system for transaction routing hosts
 * 
 * Implements comprehensive reward sharing based on:
 * - Transaction routing performance and success rates
 * - Cross-chain bridging and network utility contributions
 * - Stake-neutral reward distribution ensuring equal opportunities
 * - Anti-gaming mechanisms and Sybil protection
 */
contract HostRewardDistributor is Ownable, ReentrancyGuard, Pausable {
    using SafeERC20 for IERC20;
    
    // Reward distribution configuration
    struct RewardConfig {
        uint256 baseRewardPercentage;       // Base reward percentage for routing hosts (default: 70%)
        uint256 performanceBonusMax;        // Maximum performance bonus multiplier (default: 2x)
        uint256 crossChainBonusRate;        // Additional bonus for cross-chain routing (default: 5%)
        uint256 networkUtilityBonusRate;    // Bonus for network utility contributions (default: 3%)
        uint256 minimumRoutingThreshold;    // Minimum transactions before earning rewards
        uint256 rewardDecayRate;            // Daily reward decay rate for inactive hosts
        uint256 maxRewardAccumulation;      // Maximum reward accumulation period (days)
        bool stakingRequirementEnabled;     // Enable/disable staking requirements (should be false)
    }
    
    // Host reward tracking
    struct HostRewards {
        address hostAddress;                // Host wallet address
        uint256 totalEarned;                // Total rewards earned lifetime
        uint256 totalRouted;                // Total transactions successfully routed
        uint256 successfulTransactions;     // Number of successful transactions
        uint256 failedTransactions;         // Number of failed transactions
        uint256 totalLatencySeconds;        // Cumulative latency for averaging
        uint256 crossChainTransactions;     // Cross-chain transactions routed
        uint256 lastRewardClaim;            // Last reward claim timestamp
        uint256 pendingRewards;             // Unclaimed rewards balance
        uint256 reputationScore;            // Long-term reputation (0-1000)
        bool isActive;                      // Active status
    }
    
    // Reward distribution event tracking
    struct RewardDistribution {
        bytes32 transactionId;              // Associated transaction ID
        address host;                       // Reward recipient
        uint256 baseReward;                 // Base routing reward
        uint256 performanceBonus;           // Performance-based bonus
        uint256 crossChainBonus;            // Cross-chain routing bonus
        uint256 networkUtilityBonus;        // Network utility bonus
        uint256 totalReward;                // Total reward distributed
        uint256 timestamp;                  // Distribution timestamp
        string reason;                      // Reward distribution reason
    }
    
    // Network utility contribution tracking
    struct NetworkContribution {
        uint256 liquidityProvided;         // Liquidity provided to network
        uint256 uptimeSeconds;              // Host uptime contribution
        uint256 networkStabilizationActions; // Actions contributing to network stability
        uint256 crossChainBridgeVolume;     // Volume facilitated in cross-chain bridging
        uint256 lastContributionUpdate;    // Last contribution update timestamp
    }
    
    // Reward pool management
    struct RewardPool {
        uint256 totalPoolSize;              // Total rewards available for distribution
        uint256 distributedRewards;         // Total rewards distributed
        uint256 pendingDistributions;       // Pending reward distributions
        uint256 reserveAmount;              // Reserve fund amount
        uint256 lastPoolUpdate;             // Last pool size update
        bool emergencyWithdrawEnabled;      // Emergency withdrawal status
    }
    
    // State variables
    RewardConfig public rewardConfig;
    RewardPool public rewardPool;
    DynamicEconomicsOracle public economicsOracle;
    HypermeshNetworkManager public networkManager;
    IERC20 public rewardToken;
    
    mapping(address => HostRewards) public hostRewards;
    mapping(bytes32 => RewardDistribution) public rewardDistributions;
    mapping(address => NetworkContribution) public networkContributions;
    mapping(address => bool) public authorizedDistributors;
    
    address[] public activeHosts;
    bytes32[] public recentDistributions;
    
    uint256 public totalHosts;
    uint256 public totalRewardsDistributed;
    uint256 public totalTransactionsProcessed;
    
    // Anti-gaming and security
    mapping(address => uint256) public lastActivityTimestamp;
    mapping(address => uint256) public dailyRewardCap;
    mapping(bytes32 => bool) public processedTransactions;
    
    // Events
    event RewardDistributed(
        address indexed host,
        bytes32 indexed transactionId,
        uint256 baseReward,
        uint256 totalReward,
        string reason
    );
    event HostRegistered(address indexed host, uint256 timestamp);
    event HostDeactivated(address indexed host, string reason);
    event RewardClaimed(address indexed host, uint256 amount, uint256 timestamp);
    event NetworkContributionUpdated(address indexed host, uint256 contributionType, uint256 amount);
    event RewardPoolUpdated(uint256 newPoolSize, uint256 reserveAmount);
    event EmergencyWithdrawal(address indexed host, uint256 amount, string reason);
    event PerformanceMetricsUpdated(address indexed host, uint256 successRate, uint256 avgLatency);
    
    constructor(
        address owner,
        address payable _economicsOracle,
        address payable _networkManager,
        address _rewardToken
    ) Ownable(owner) {
        economicsOracle = DynamicEconomicsOracle(_economicsOracle);
        networkManager = HypermeshNetworkManager(_networkManager);
        rewardToken = IERC20(_rewardToken);
        
        // Initialize reward configuration (stake-neutral design)
        rewardConfig = RewardConfig({
            baseRewardPercentage: 700,          // 70% of transaction fees to hosts
            performanceBonusMax: 2000,          // 2x maximum performance bonus
            crossChainBonusRate: 50,            // 5% cross-chain bonus
            networkUtilityBonusRate: 30,        // 3% network utility bonus
            minimumRoutingThreshold: 10,        // Minimum 10 transactions before rewards
            rewardDecayRate: 50,                // 5% daily decay for inactive hosts
            maxRewardAccumulation: 7,           // Maximum 7 days reward accumulation
            stakingRequirementEnabled: false    // NO staking requirements (stake-neutral)
        });
        
        // Initialize reward pool
        rewardPool = RewardPool({
            totalPoolSize: 0,
            distributedRewards: 0,
            pendingDistributions: 0,
            reserveAmount: 0,
            lastPoolUpdate: block.timestamp,
            emergencyWithdrawEnabled: false
        });
        
        // Authorize this contract and economics oracle as distributors
        authorizedDistributors[address(this)] = true;
        authorizedDistributors[address(_economicsOracle)] = true;
    }
    
    /**
     * @dev Register host for reward distribution (stake-neutral)
     */
    function registerHost() external {
        require(hostRewards[msg.sender].hostAddress == address(0), "Host already registered");
        
        // No staking requirements - purely stake-neutral registration
        hostRewards[msg.sender] = HostRewards({
            hostAddress: msg.sender,
            totalEarned: 0,
            totalRouted: 0,
            successfulTransactions: 0,
            failedTransactions: 0,
            totalLatencySeconds: 0,
            crossChainTransactions: 0,
            lastRewardClaim: block.timestamp,
            pendingRewards: 0,
            reputationScore: 500,               // Start with neutral reputation
            isActive: true
        });
        
        activeHosts.push(msg.sender);
        totalHosts++;
        lastActivityTimestamp[msg.sender] = block.timestamp;
        
        emit HostRegistered(msg.sender, block.timestamp);
    }
    
    /**
     * @dev Distribute rewards for successful transaction routing
     * Called by authorized contracts (HypermeshNetworkManager, DynamicEconomicsOracle)
     */
    function distributeTransactionReward(
        bytes32 transactionId,
        address host,
        uint256 transactionFee,
        bool isCrossChain,
        uint256 actualLatency,
        bool success
    ) external nonReentrant {
        require(authorizedDistributors[msg.sender], "Unauthorized distributor");
        require(hostRewards[host].hostAddress != address(0), "Host not registered");
        require(!processedTransactions[transactionId], "Transaction already processed");
        
        processedTransactions[transactionId] = true;
        
        // Update host performance metrics
        _updateHostMetrics(host, success, actualLatency, isCrossChain);
        
        if (!success) {
            // No reward for failed transactions, but track metrics
            return;
        }
        
        // Calculate rewards based on current configuration and performance
        (uint256 baseReward, uint256 performanceBonus, uint256 crossChainBonus, uint256 utilityBonus) = 
            _calculateHostReward(host, transactionFee, isCrossChain);
        
        uint256 totalReward = baseReward + performanceBonus + crossChainBonus + utilityBonus;
        
        // Apply daily reward cap protection
        if (_checkDailyRewardCap(host, totalReward)) {
            // Update pending rewards
            hostRewards[host].pendingRewards += totalReward;
            rewardPool.pendingDistributions += totalReward;
            
            // Record distribution
            rewardDistributions[transactionId] = RewardDistribution({
                transactionId: transactionId,
                host: host,
                baseReward: baseReward,
                performanceBonus: performanceBonus,
                crossChainBonus: crossChainBonus,
                networkUtilityBonus: utilityBonus,
                totalReward: totalReward,
                timestamp: block.timestamp,
                reason: "Transaction routing reward"
            });
            
            recentDistributions.push(transactionId);
            totalRewardsDistributed += totalReward;
            totalTransactionsProcessed++;
            
            emit RewardDistributed(host, transactionId, baseReward, totalReward, "Transaction routing");
        }
        
        lastActivityTimestamp[host] = block.timestamp;
    }
    
    /**
     * @dev Calculate comprehensive reward for host based on performance and contribution
     * Implements formulas from Factor 1, 2, and 3 specifications
     */
    function _calculateHostReward(
        address host,
        uint256 transactionFee,
        bool isCrossChain
    ) internal view returns (uint256 baseReward, uint256 performanceBonus, uint256 crossChainBonus, uint256 utilityBonus) {
        HostRewards memory hostData = hostRewards[host];
        
        // Base reward calculation (70% of transaction fee)
        baseReward = (transactionFee * rewardConfig.baseRewardPercentage) / 1000;
        
        // Performance bonus calculation based on success rate and latency
        uint256 successRate = hostData.totalRouted > 0 ? 
            (hostData.successfulTransactions * 1000) / hostData.totalRouted : 1000;
        uint256 avgLatency = hostData.successfulTransactions > 0 ? 
            hostData.totalLatencySeconds / hostData.successfulTransactions : 5000;
        
        // Performance multiplier: combines success rate (40%) + latency performance (30%) + reputation (30%)
        uint256 performanceMultiplier = _calculatePerformanceMultiplier(successRate, avgLatency, hostData.reputationScore);
        performanceBonus = (baseReward * (performanceMultiplier - 1000)) / 1000; // Subtract base multiplier
        
        // Cross-chain bonus (5% additional for cross-chain transactions)
        if (isCrossChain) {
            crossChainBonus = (baseReward * rewardConfig.crossChainBonusRate) / 1000;
        }
        
        // Network utility bonus based on overall contribution
        NetworkContribution memory contribution = networkContributions[host];
        uint256 utilityMultiplier = _calculateNetworkUtilityMultiplier(contribution);
        utilityBonus = (baseReward * utilityMultiplier) / 1000;
        
        return (baseReward, performanceBonus, crossChainBonus, utilityBonus);
    }
    
    /**
     * @dev Calculate performance multiplier based on host metrics
     * Formula: Performance_Multiplier = (Success_Rate * 0.4) + (Avg_Latency_Score * 0.3) + (Reputation * 0.3)
     */
    function _calculatePerformanceMultiplier(
        uint256 successRate,
        uint256 avgLatency,
        uint256 reputationScore
    ) internal view returns (uint256) {
        // Success rate component (40% weight) - range 0-1000
        uint256 successComponent = (successRate * 400) / 1000;
        
        // Latency performance component (30% weight) - better latency = higher score
        uint256 targetLatency = 5000; // 5 seconds target
        uint256 latencyScore = avgLatency > targetLatency ? 
            (targetLatency * 1000) / avgLatency : 1000;
        uint256 latencyComponent = (latencyScore * 300) / 1000;
        
        // Reputation component (30% weight)
        uint256 reputationComponent = (reputationScore * 300) / 1000;
        
        // Total multiplier (base 1000 = 1x, max 2000 = 2x)
        uint256 totalMultiplier = 1000 + successComponent + latencyComponent + reputationComponent;
        
        // Cap at maximum bonus
        if (totalMultiplier > rewardConfig.performanceBonusMax) {
            totalMultiplier = rewardConfig.performanceBonusMax;
        }
        
        return totalMultiplier;
    }
    
    /**
     * @dev Calculate network utility multiplier for additional bonuses
     */
    function _calculateNetworkUtilityMultiplier(NetworkContribution memory contribution) internal view returns (uint256) {
        uint256 utilityScore = 0;
        
        // Liquidity provision bonus
        if (contribution.liquidityProvided > 1 ether) {
            utilityScore += 10; // 1% bonus for significant liquidity provision
        }
        
        // Uptime bonus (for hosts with > 90% uptime)
        uint256 daysSinceUpdate = (block.timestamp - contribution.lastContributionUpdate) / 86400;
        if (daysSinceUpdate > 0) {
            uint256 uptimePercentage = (contribution.uptimeSeconds * 100) / (daysSinceUpdate * 86400);
            if (uptimePercentage > 90) {
                utilityScore += 15; // 1.5% bonus for high uptime
            }
        }
        
        // Network stabilization actions bonus
        if (contribution.networkStabilizationActions > 10) {
            utilityScore += 5; // 0.5% bonus for stabilization contributions
        }
        
        return utilityScore;
    }
    
    /**
     * @dev Update host performance metrics after transaction
     */
    function _updateHostMetrics(address host, bool success, uint256 actualLatency, bool isCrossChain) internal {
        HostRewards storage hostData = hostRewards[host];
        
        hostData.totalRouted++;
        
        if (success) {
            hostData.successfulTransactions++;
            hostData.totalLatencySeconds += actualLatency;
            
            if (isCrossChain) {
                hostData.crossChainTransactions++;
            }
            
            // Improve reputation for successful transactions
            if (hostData.reputationScore < 1000) {
                hostData.reputationScore += 2; // Small reputation increase
            }
        } else {
            hostData.failedTransactions++;
            
            // Decrease reputation for failed transactions
            if (hostData.reputationScore > 10) {
                hostData.reputationScore -= 10; // Larger reputation decrease for failures
            }
        }
        
        emit PerformanceMetricsUpdated(
            host,
            hostData.totalRouted > 0 ? (hostData.successfulTransactions * 1000) / hostData.totalRouted : 0,
            hostData.successfulTransactions > 0 ? hostData.totalLatencySeconds / hostData.successfulTransactions : 0
        );
    }
    
    /**
     * @dev Check daily reward cap to prevent gaming
     */
    function _checkDailyRewardCap(address host, uint256 rewardAmount) internal returns (bool) {
        uint256 today = block.timestamp / 86400; // Get current day
        uint256 lastRewardDay = lastActivityTimestamp[host] / 86400;
        
        // Reset daily cap if new day
        if (today > lastRewardDay) {
            dailyRewardCap[host] = 0;
        }
        
        // Check if adding this reward would exceed daily cap
        uint256 maxDailyCap = rewardPool.totalPoolSize / 100; // 1% of total pool per day max
        if (dailyRewardCap[host] + rewardAmount > maxDailyCap) {
            return false; // Exceeds daily cap
        }
        
        dailyRewardCap[host] += rewardAmount;
        return true;
    }
    
    /**
     * @dev Claim pending rewards (stake-neutral)
     */
    function claimRewards() external nonReentrant whenNotPaused {
        HostRewards storage hostData = hostRewards[msg.sender];
        require(hostData.hostAddress != address(0), "Host not registered");
        require(hostData.pendingRewards > 0, "No pending rewards");
        require(hostData.totalRouted >= rewardConfig.minimumRoutingThreshold, "Insufficient routing activity");
        
        uint256 claimAmount = hostData.pendingRewards;
        
        // Apply reward decay if host has been inactive
        uint256 daysSinceLastClaim = (block.timestamp - hostData.lastRewardClaim) / 86400;
        if (daysSinceLastClaim > rewardConfig.maxRewardAccumulation) {
            // Apply decay for extended inactivity
            uint256 decayFactor = 1000 - (rewardConfig.rewardDecayRate * daysSinceLastClaim);
            if (decayFactor < 500) decayFactor = 500; // Minimum 50% retention
            claimAmount = (claimAmount * decayFactor) / 1000;
        }
        
        // Ensure sufficient pool balance
        require(rewardToken.balanceOf(address(this)) >= claimAmount, "Insufficient reward pool");
        
        // Update host data
        hostData.pendingRewards = 0;
        hostData.totalEarned += claimAmount;
        hostData.lastRewardClaim = block.timestamp;
        
        // Update pool data
        rewardPool.distributedRewards += claimAmount;
        rewardPool.pendingDistributions -= hostData.pendingRewards + claimAmount;
        
        // Transfer rewards
        rewardToken.safeTransfer(msg.sender, claimAmount);
        
        emit RewardClaimed(msg.sender, claimAmount, block.timestamp);
    }
    
    /**
     * @dev Update network contribution metrics
     */
    function updateNetworkContribution(
        address host,
        uint256 liquidityProvided,
        uint256 uptimeSeconds,
        uint256 stabilizationActions
    ) external {
        require(authorizedDistributors[msg.sender] || msg.sender == host, "Unauthorized");
        
        NetworkContribution storage contribution = networkContributions[host];
        contribution.liquidityProvided = liquidityProvided;
        contribution.uptimeSeconds += uptimeSeconds;
        contribution.networkStabilizationActions += stabilizationActions;
        contribution.lastContributionUpdate = block.timestamp;
        
        emit NetworkContributionUpdated(host, 1, liquidityProvided);
    }
    
    /**
     * @dev Get comprehensive host statistics
     */
    function getHostStatistics(address host) external view returns (
        uint256 totalEarned,
        uint256 pendingRewards,
        uint256 successRate,
        uint256 averageLatency,
        uint256 reputationScore,
        uint256 totalRouted,
        uint256 crossChainTransactions,
        bool isActive
    ) {
        HostRewards memory hostData = hostRewards[host];
        
        uint256 avgLatency = hostData.successfulTransactions > 0 ? 
            hostData.totalLatencySeconds / hostData.successfulTransactions : 0;
        uint256 successRatePercent = hostData.totalRouted > 0 ? 
            (hostData.successfulTransactions * 1000) / hostData.totalRouted : 0;
        
        return (
            hostData.totalEarned,
            hostData.pendingRewards,
            successRatePercent,
            avgLatency,
            hostData.reputationScore,
            hostData.totalRouted,
            hostData.crossChainTransactions,
            hostData.isActive
        );
    }
    
    /**
     * @dev Get system-wide reward statistics
     */
    function getSystemRewardStatistics() external view returns (
        uint256 totalPoolSize,
        uint256 totalDistributed,
        uint256 totalPending,
        uint256 totalActiveHosts,
        uint256 systemTransactionsProcessed,
        uint256 averageRewardPerTransaction
    ) {
        uint256 avgReward = totalTransactionsProcessed > 0 ? 
            totalRewardsDistributed / totalTransactionsProcessed : 0;
            
        return (
            rewardPool.totalPoolSize,
            rewardPool.distributedRewards,
            rewardPool.pendingDistributions,
            activeHosts.length,
            totalTransactionsProcessed,
            avgReward
        );
    }
    
    /**
     * @dev Fund reward pool
     */
    function fundRewardPool(uint256 amount) external {
        require(amount > 0, "Invalid amount");
        
        rewardToken.safeTransferFrom(msg.sender, address(this), amount);
        rewardPool.totalPoolSize += amount;
        rewardPool.lastPoolUpdate = block.timestamp;
        
        emit RewardPoolUpdated(rewardPool.totalPoolSize, rewardPool.reserveAmount);
    }
    
    /**
     * @dev Emergency functions (owner only)
     */
    function emergencyWithdrawHost(address host, string calldata reason) external onlyOwner {
        require(rewardPool.emergencyWithdrawEnabled, "Emergency withdraw disabled");
        
        HostRewards storage hostData = hostRewards[host];
        uint256 amount = hostData.pendingRewards;
        
        if (amount > 0) {
            hostData.pendingRewards = 0;
            rewardPool.pendingDistributions -= amount;
            
            rewardToken.safeTransfer(host, amount);
            emit EmergencyWithdrawal(host, amount, reason);
        }
    }
    
    function setEmergencyWithdrawEnabled(bool enabled) external onlyOwner {
        rewardPool.emergencyWithdrawEnabled = enabled;
    }
    
    function updateRewardConfig(RewardConfig calldata newConfig) external onlyOwner {
        // Enforce stake-neutral design
        require(!newConfig.stakingRequirementEnabled, "Staking requirements not allowed");
        rewardConfig = newConfig;
    }
    
    function addAuthorizedDistributor(address distributor) external onlyOwner {
        authorizedDistributors[distributor] = true;
    }
    
    function removeAuthorizedDistributor(address distributor) external onlyOwner {
        authorizedDistributors[distributor] = false;
    }
    
    /**
     * @dev Receive function for direct ETH rewards
     */
    receive() external payable {
        rewardPool.totalPoolSize += msg.value;
        emit RewardPoolUpdated(rewardPool.totalPoolSize, rewardPool.reserveAmount);
    }
}