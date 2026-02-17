// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "./DynamicEconomicsOracle.sol";

/**
 * @title CrossChainRouteOptimizer
 * @dev Intelligent rate limiting and cross-chain rerouting system
 * 
 * Implements dynamic transaction routing based on:
 * - Network capacity and performance metrics
 * - Real-time cost analysis
 * - Automated failover and load balancing
 * - Circuit breaker mechanisms for network protection
 */
contract CrossChainRouteOptimizer is Ownable, ReentrancyGuard, Pausable {
    
    // Route optimization configuration
    struct RouteConfig {
        uint256 maxRetries;                 // Maximum routing retry attempts
        uint256 timeoutWindow;              // Route timeout in seconds
        uint256 costThresholdPercent;       // Cost increase threshold for rerouting (%)
        uint256 latencyThresholdMs;         // Maximum acceptable latency (ms)
        uint256 minQualityScore;            // Minimum network quality for routing
        uint256 emergencyRerouteThreshold;  // Emergency rerouting quality threshold
        bool adaptiveRoutingEnabled;        // Enable/disable adaptive routing
    }
    
    // Transaction routing request
    struct RouteRequest {
        bytes32 requestId;                  // Unique request identifier
        address sender;                     // Transaction sender
        address targetNetwork;              // Preferred target network
        uint256 amount;                     // Transaction amount
        uint256 maxCost;                    // Maximum acceptable cost
        uint256 maxLatency;                 // Maximum acceptable latency
        uint256 priority;                   // Transaction priority (0-1000)
        uint256 timestamp;                  // Request timestamp
        RouteStatus status;                 // Current routing status
    }
    
    // Route path information
    struct RoutePath {
        address[] networks;                 // Network path
        uint256[] expectedCosts;            // Cost per network hop
        uint256[] expectedLatencies;        // Latency per network hop
        uint256 totalCost;                  // Total routing cost
        uint256 totalLatency;               // Total expected latency
        uint256 qualityScore;               // Route quality score
        uint256 reliabilityScore;           // Route reliability score
        bool isActive;                      // Route active status
    }
    
    // Network rate limiting state
    struct RateLimit {
        uint256 currentCount;               // Current transaction count in window
        uint256 maxCount;                   // Maximum allowed transactions
        uint256 windowStart;                // Rate limiting window start time
        uint256 windowDuration;             // Window duration in seconds
        uint256 backoffUntil;               // Backoff period end timestamp
        bool isThrottled;                   // Current throttling status
    }
    
    // Network performance metrics
    struct NetworkMetrics {
        uint256 successRate;                // Historical success rate (0-1000)
        uint256 averageLatency;             // Average transaction latency
        uint256 currentLoad;                // Current network load (0-1000)
        uint256 capacityUtilization;       // Capacity utilization (0-1000)
        uint256 lastFailureTime;           // Last recorded failure timestamp
        uint256 consecutiveFailures;       // Consecutive failure count
        uint256 totalTransactions;         // Total transactions routed
        uint256 lastUpdate;                 // Last metrics update
    }
    
    enum RouteStatus {
        PENDING,        // Awaiting route calculation
        CALCULATED,     // Route calculated successfully
        EXECUTING,      // Route execution in progress
        COMPLETED,      // Route completed successfully
        FAILED,         // Route execution failed
        REROUTED,       // Route was rerouted to alternative
        TIMEOUT         // Route execution timed out
    }
    
    // Emergency throttling modes
    enum ThrottleMode {
        NORMAL,         // 100% capacity
        CONGESTED,      // 70% capacity
        EMERGENCY,      // 30% capacity
        HALT            // 0% capacity (trading halted)
    }
    
    // State variables
    RouteConfig public routeConfig;
    DynamicEconomicsOracle public economicsOracle;
    
    mapping(bytes32 => RouteRequest) public routeRequests;
    mapping(address => RateLimit) public rateLimits;
    mapping(address => NetworkMetrics) public networkMetrics;
    mapping(address => bool) public supportedNetworks;
    mapping(bytes32 => RoutePath) public calculatedRoutes;
    
    address[] public activeNetworks;
    bytes32[] public pendingRequests;
    
    uint256 public totalRequests;
    uint256 public successfulRoutes;
    uint256 public failedRoutes;
    uint256 public reroutedTransactions;
    
    ThrottleMode public currentThrottleMode;
    uint256 public lastThrottleUpdate;
    
    // Events
    event RouteRequested(bytes32 indexed requestId, address sender, address targetNetwork, uint256 amount);
    event RouteCalculated(bytes32 indexed requestId, address[] networks, uint256 totalCost, uint256 totalLatency);
    event RouteExecuted(bytes32 indexed requestId, bool success, uint256 actualCost, uint256 actualLatency);
    event RouteRerouted(bytes32 indexed requestId, address fromNetwork, address toNetwork, string reason);
    event RateLimitTriggered(address indexed network, uint256 currentLoad, uint256 backoffDuration);
    event ThrottleModeChanged(ThrottleMode oldMode, ThrottleMode newMode, string reason);
    event NetworkMetricsUpdated(address indexed network, uint256 successRate, uint256 averageLatency);
    event EmergencyRerouteActivated(address indexed network, string reason);
    
    constructor(address owner, address payable _economicsOracle) Ownable(owner) {
        economicsOracle = DynamicEconomicsOracle(_economicsOracle);
        
        // Initialize default configuration
        routeConfig = RouteConfig({
            maxRetries: 3,
            timeoutWindow: 300,             // 5 minutes
            costThresholdPercent: 150,      // 150% cost increase threshold
            latencyThresholdMs: 30000,      // 30 seconds max latency
            minQualityScore: 300,           // Minimum 30% quality score
            emergencyRerouteThreshold: 200, // 20% quality for emergency reroute
            adaptiveRoutingEnabled: true
        });
        
        currentThrottleMode = ThrottleMode.NORMAL;
        lastThrottleUpdate = block.timestamp;
    }
    
    /**
     * @dev Request optimal route for cross-chain transaction
     */
    function requestRoute(
        address targetNetwork,
        uint256 amount,
        uint256 maxCost,
        uint256 maxLatency,
        uint256 priority
    ) external nonReentrant whenNotPaused returns (bytes32 requestId) {
        require(supportedNetworks[targetNetwork], "Unsupported target network");
        require(amount > 0, "Invalid amount");
        require(priority <= 1000, "Invalid priority");
        
        // Check rate limiting
        require(!_isRateLimited(targetNetwork), "Network rate limited");
        
        // Generate unique request ID
        requestId = keccak256(abi.encodePacked(
            msg.sender,
            targetNetwork,
            amount,
            block.timestamp,
            totalRequests
        ));
        
        // Create route request
        routeRequests[requestId] = RouteRequest({
            requestId: requestId,
            sender: msg.sender,
            targetNetwork: targetNetwork,
            amount: amount,
            maxCost: maxCost,
            maxLatency: maxLatency,
            priority: priority,
            timestamp: block.timestamp,
            status: RouteStatus.PENDING
        });
        
        pendingRequests.push(requestId);
        totalRequests++;
        
        // Calculate optimal route
        _calculateOptimalRoute(requestId);
        
        emit RouteRequested(requestId, msg.sender, targetNetwork, amount);
        return requestId;
    }
    
    /**
     * @dev Calculate optimal route based on current network conditions
     */
    function _calculateOptimalRoute(bytes32 requestId) internal {
        RouteRequest storage request = routeRequests[requestId];
        
        // Get current economic parameters
        DynamicEconomicsOracle.EconomicParameters memory economicParams = economicsOracle.getEconomicParameters();
        
        // Find available networks
        address[] memory availableNetworks = _getAvailableNetworks();
        
        // Calculate route options
        RoutePath memory bestRoute;
        uint256 bestScore = 0;
        
        for (uint256 i = 0; i < availableNetworks.length; i++) {
            address network = availableNetworks[i];
            
            // Skip if network doesn't meet minimum requirements
            NetworkMetrics memory metrics = networkMetrics[network];
            if (metrics.successRate < routeConfig.minQualityScore) {
                continue;
            }
            
            // Calculate route score
            uint256 routeScore = _calculateRouteScore(network, request, economicParams);
            
            if (routeScore > bestScore) {
                bestScore = routeScore;
                bestRoute = _buildRoutePath(network, request, economicParams);
            }
        }
        
        // Store calculated route
        if (bestRoute.networks.length > 0) {
            calculatedRoutes[requestId] = bestRoute;
            request.status = RouteStatus.CALCULATED;
            
            emit RouteCalculated(
                requestId, 
                bestRoute.networks, 
                bestRoute.totalCost, 
                bestRoute.totalLatency
            );
        } else {
            request.status = RouteStatus.FAILED;
        }
    }
    
    /**
     * @dev Calculate route score based on cost, latency, and reliability
     * Formula: Route_Score = (1/Cost) * Quality * (1/Latency) * Success_Rate
     */
    function _calculateRouteScore(
        address network,
        RouteRequest memory request,
        DynamicEconomicsOracle.EconomicParameters memory economicParams
    ) internal view returns (uint256) {
        NetworkMetrics memory metrics = networkMetrics[network];
        
        // Cost factor (inversely proportional)
        uint256 estimatedCost = _estimateRoutingCost(network, request.amount, economicParams);
        uint256 costScore = estimatedCost > 0 ? (1000 ether) / estimatedCost : 0;
        
        // Latency factor (inversely proportional)
        uint256 latencyScore = metrics.averageLatency > 0 ? 
            (routeConfig.latencyThresholdMs * 1000) / metrics.averageLatency : 1000;
        
        // Quality factor (network health) - use success rate as fallback
        uint256 qualityScore = metrics.successRate;
        
        // Success rate factor
        uint256 successScore = metrics.successRate;
        
        // Priority adjustment
        uint256 priorityMultiplier = 500 + (request.priority / 2); // Range: 500-1000
        
        // Calculate final score
        uint256 score = (costScore * qualityScore * latencyScore * successScore * priorityMultiplier) / 
                       (1000 * 1000 * 1000 * 1000);
        
        return score;
    }
    
    /**
     * @dev Build complete route path for network
     */
    function _buildRoutePath(
        address network,
        RouteRequest memory request,
        DynamicEconomicsOracle.EconomicParameters memory economicParams
    ) internal view returns (RoutePath memory) {
        address[] memory networks = new address[](1);
        uint256[] memory costs = new uint256[](1);
        uint256[] memory latencies = new uint256[](1);
        
        networks[0] = network;
        costs[0] = _estimateRoutingCost(network, request.amount, economicParams);
        latencies[0] = networkMetrics[network].averageLatency;
        
        return RoutePath({
            networks: networks,
            expectedCosts: costs,
            expectedLatencies: latencies,
            totalCost: costs[0],
            totalLatency: latencies[0],
            qualityScore: networkMetrics[network].successRate,
            reliabilityScore: _calculateReliabilityScore(network),
            isActive: true
        });
    }
    
    /**
     * @dev Estimate routing cost for network
     */
    function _estimateRoutingCost(
        address network,
        uint256 amount,
        DynamicEconomicsOracle.EconomicParameters memory economicParams
    ) internal view returns (uint256) {
        // Base cost from dynamic gas fee
        uint256 baseCost = economicParams.dynamicGasFee;
        
        // Network-specific cost multiplier
        NetworkMetrics memory metrics = networkMetrics[network];
        uint256 loadMultiplier = 1000 + (metrics.currentLoad / 2); // Up to 1.5x for high load
        
        // Amount-based scaling (larger amounts may have higher costs)
        uint256 amountMultiplier = 1000 + (amount / 100 ether); // Small scaling factor
        
        return (baseCost * loadMultiplier * amountMultiplier) / (1000 * 1000);
    }
    
    /**
     * @dev Calculate network reliability score
     */
    function _calculateReliabilityScore(address network) internal view returns (uint256) {
        NetworkMetrics memory metrics = networkMetrics[network];
        
        // Base score from success rate
        uint256 baseScore = metrics.successRate;
        
        // Penalty for consecutive failures
        if (metrics.consecutiveFailures > 0) {
            uint256 failurePenalty = metrics.consecutiveFailures * 50; // -50 per consecutive failure
            baseScore = baseScore > failurePenalty ? baseScore - failurePenalty : 0;
        }
        
        // Bonus for high transaction volume (proven reliability)
        if (metrics.totalTransactions > 1000) {
            uint256 volumeBonus = 50; // +50 for high volume networks
            baseScore = baseScore + volumeBonus > 1000 ? 1000 : baseScore + volumeBonus;
        }
        
        return baseScore;
    }
    
    /**
     * @dev Execute calculated route
     */
    function executeRoute(bytes32 requestId) external nonReentrant {
        RouteRequest storage request = routeRequests[requestId];
        require(request.sender == msg.sender, "Unauthorized");
        require(request.status == RouteStatus.CALCULATED, "Route not ready");
        
        RoutePath memory route = calculatedRoutes[requestId];
        require(route.isActive, "Route inactive");
        
        // Check if rerouting is needed
        (bool needsReroute, address alternativeNetwork) = economicsOracle.shouldTriggerReroute(route.networks[0]);
        
        if (needsReroute && routeConfig.adaptiveRoutingEnabled) {
            _handleReroute(requestId, route.networks[0], alternativeNetwork);
            return;
        }
        
        // Update request status
        request.status = RouteStatus.EXECUTING;
        
        // Update rate limiting
        _updateRateLimit(route.networks[0]);
        
        // Simulate route execution (in production, this would interact with actual cross-chain protocols)
        bool success = _simulateRouteExecution(route);
        
        // Update metrics and status
        _updateNetworkMetrics(route.networks[0], success, route.totalLatency);
        
        if (success) {
            request.status = RouteStatus.COMPLETED;
            successfulRoutes++;
            emit RouteExecuted(requestId, true, route.totalCost, route.totalLatency);
        } else {
            request.status = RouteStatus.FAILED;
            failedRoutes++;
            emit RouteExecuted(requestId, false, 0, 0);
        }
    }
    
    /**
     * @dev Handle route rerouting
     */
    function _handleReroute(bytes32 requestId, address originalNetwork, address alternativeNetwork) internal {
        RouteRequest storage request = routeRequests[requestId];
        
        if (alternativeNetwork != address(0) && supportedNetworks[alternativeNetwork]) {
            // Update target network
            request.targetNetwork = alternativeNetwork;
            request.status = RouteStatus.REROUTED;
            
            // Recalculate route for alternative network
            _calculateOptimalRoute(requestId);
            
            reroutedTransactions++;
            emit RouteRerouted(requestId, originalNetwork, alternativeNetwork, "Network quality below threshold");
        } else {
            request.status = RouteStatus.FAILED;
            emit RouteRerouted(requestId, originalNetwork, address(0), "No alternative network available");
        }
    }
    
    /**
     * @dev Simulate route execution (replace with actual cross-chain integration)
     */
    function _simulateRouteExecution(RoutePath memory route) internal view returns (bool) {
        // Simulate based on network success rate
        NetworkMetrics memory metrics = networkMetrics[route.networks[0]];
        
        // Generate pseudo-random success based on network reliability
        uint256 randomValue = uint256(keccak256(abi.encodePacked(block.timestamp, msg.sender))) % 1000;
        
        return randomValue < metrics.successRate;
    }
    
    /**
     * @dev Update network metrics after route execution
     */
    function _updateNetworkMetrics(address network, bool success, uint256 actualLatency) internal {
        NetworkMetrics storage metrics = networkMetrics[network];
        
        // Update total transactions
        metrics.totalTransactions++;
        
        // Update success rate (exponential moving average)
        uint256 newSuccessValue = success ? 1000 : 0;
        metrics.successRate = (metrics.successRate * 9 + newSuccessValue) / 10;
        
        // Update average latency (exponential moving average)
        if (success) {
            metrics.averageLatency = (metrics.averageLatency * 9 + actualLatency) / 10;
            metrics.consecutiveFailures = 0;
        } else {
            metrics.consecutiveFailures++;
            metrics.lastFailureTime = block.timestamp;
        }
        
        metrics.lastUpdate = block.timestamp;
        
        // Check for throttle mode changes
        _evaluateThrottleMode();
        
        emit NetworkMetricsUpdated(network, metrics.successRate, metrics.averageLatency);
    }
    
    /**
     * @dev Check if network is rate limited
     */
    function _isRateLimited(address network) internal view returns (bool) {
        RateLimit memory limit = rateLimits[network];
        
        // Check backoff period
        if (limit.backoffUntil > block.timestamp) {
            return true;
        }
        
        // Check current throttle mode
        uint256 capacityMultiplier = _getCapacityMultiplier();
        uint256 effectiveLimit = (limit.maxCount * capacityMultiplier) / 1000;
        
        // Check if within rate window and under limit
        if (block.timestamp >= limit.windowStart + limit.windowDuration) {
            return false; // New window, reset
        }
        
        return limit.currentCount >= effectiveLimit;
    }
    
    /**
     * @dev Update rate limit counters
     */
    function _updateRateLimit(address network) internal {
        RateLimit storage limit = rateLimits[network];
        
        // Initialize if first time
        if (limit.windowStart == 0) {
            limit.maxCount = 100; // Default 100 transactions per window
            limit.windowDuration = 3600; // 1 hour window
        }
        
        // Check if new window
        if (block.timestamp >= limit.windowStart + limit.windowDuration) {
            limit.currentCount = 1;
            limit.windowStart = block.timestamp;
            limit.isThrottled = false;
        } else {
            limit.currentCount++;
        }
        
        // Check if rate limit exceeded
        uint256 capacityMultiplier = _getCapacityMultiplier();
        uint256 effectiveLimit = (limit.maxCount * capacityMultiplier) / 1000;
        
        if (limit.currentCount >= effectiveLimit) {
            limit.isThrottled = true;
            limit.backoffUntil = block.timestamp + 300; // 5 minute backoff
            
            emit RateLimitTriggered(network, limit.currentCount, 300);
        }
    }
    
    /**
     * @dev Get capacity multiplier based on current throttle mode
     */
    function _getCapacityMultiplier() internal view returns (uint256) {
        if (currentThrottleMode == ThrottleMode.NORMAL) return 1000;      // 100%
        if (currentThrottleMode == ThrottleMode.CONGESTED) return 700;    // 70%
        if (currentThrottleMode == ThrottleMode.EMERGENCY) return 300;    // 30%
        return 0; // HALT mode
    }
    
    /**
     * @dev Evaluate and update throttle mode based on network conditions
     */
    function _evaluateThrottleMode() internal {
        // Get overall network health from economics oracle
        (uint256 activeNodes, uint256 avgSuccessRate, uint256 avgLatency, , ) = 
            economicsOracle.getNetworkHealth();
        
        ThrottleMode newMode = currentThrottleMode;
        
        // Determine new throttle mode
        if (avgSuccessRate < 500 || avgLatency > 20000) {           // < 50% success or > 20s latency
            newMode = ThrottleMode.HALT;
        } else if (avgSuccessRate < 700 || avgLatency > 10000) {    // < 70% success or > 10s latency
            newMode = ThrottleMode.EMERGENCY;
        } else if (avgSuccessRate < 850 || avgLatency > 5000) {     // < 85% success or > 5s latency
            newMode = ThrottleMode.CONGESTED;
        } else {
            newMode = ThrottleMode.NORMAL;
        }
        
        // Update mode if changed
        if (newMode != currentThrottleMode) {
            ThrottleMode oldMode = currentThrottleMode;
            currentThrottleMode = newMode;
            lastThrottleUpdate = block.timestamp;
            
            string memory reason = _getThrottleModeReason(newMode, avgSuccessRate, avgLatency);
            emit ThrottleModeChanged(oldMode, newMode, reason);
        }
    }
    
    /**
     * @dev Get throttle mode change reason
     */
    function _getThrottleModeReason(
        ThrottleMode mode, 
        uint256 successRate, 
        uint256 latency
    ) internal pure returns (string memory) {
        if (mode == ThrottleMode.HALT) {
            return successRate < 500 ? "Success rate below 50%" : "Latency above 20 seconds";
        } else if (mode == ThrottleMode.EMERGENCY) {
            return successRate < 700 ? "Success rate below 70%" : "Latency above 10 seconds";
        } else if (mode == ThrottleMode.CONGESTED) {
            return successRate < 850 ? "Success rate below 85%" : "Latency above 5 seconds";
        }
        return "Network conditions normalized";
    }
    
    /**
     * @dev Get available networks for routing
     */
    function _getAvailableNetworks() internal view returns (address[] memory) {
        uint256 availableCount = 0;
        
        // Count available networks
        for (uint256 i = 0; i < activeNetworks.length; i++) {
            if (supportedNetworks[activeNetworks[i]] && !_isRateLimited(activeNetworks[i])) {
                availableCount++;
            }
        }
        
        // Build array of available networks
        address[] memory available = new address[](availableCount);
        uint256 index = 0;
        
        for (uint256 i = 0; i < activeNetworks.length; i++) {
            if (supportedNetworks[activeNetworks[i]] && !_isRateLimited(activeNetworks[i])) {
                available[index] = activeNetworks[i];
                index++;
            }
        }
        
        return available;
    }
    
    /**
     * @dev Add supported network
     */
    function addSupportedNetwork(address network) external onlyOwner {
        require(!supportedNetworks[network], "Network already supported");
        
        supportedNetworks[network] = true;
        activeNetworks.push(network);
        
        // Initialize network metrics
        networkMetrics[network] = NetworkMetrics({
            successRate: 1000,              // Start with perfect score
            averageLatency: 5000,           // Default 5 second latency
            currentLoad: 0,
            capacityUtilization: 0,
            lastFailureTime: 0,
            consecutiveFailures: 0,
            totalTransactions: 0,
            lastUpdate: block.timestamp
        });
        
        // Initialize rate limiting
        rateLimits[network] = RateLimit({
            currentCount: 0,
            maxCount: 100,                  // Default 100 transactions per hour
            windowStart: block.timestamp,
            windowDuration: 3600,           // 1 hour window
            backoffUntil: 0,
            isThrottled: false
        });
    }
    
    /**
     * @dev Get route request details
     */
    function getRouteRequest(bytes32 requestId) external view returns (RouteRequest memory) {
        return routeRequests[requestId];
    }
    
    /**
     * @dev Get calculated route details
     */
    function getCalculatedRoute(bytes32 requestId) external view returns (RoutePath memory) {
        return calculatedRoutes[requestId];
    }
    
    /**
     * @dev Get network performance metrics
     */
    function getNetworkMetrics(address network) external view returns (NetworkMetrics memory) {
        return networkMetrics[network];
    }
    
    /**
     * @dev Get system performance statistics
     */
    function getSystemStats() external view returns (
        uint256 totalRequestCount,
        uint256 successfulRouteCount,
        uint256 failedRouteCount,
        uint256 rerouteCount,
        uint256 activeNetworkCount,
        ThrottleMode throttleMode
    ) {
        return (
            totalRequests,
            successfulRoutes,
            failedRoutes,
            reroutedTransactions,
            activeNetworks.length,
            currentThrottleMode
        );
    }
    
    /**
     * @dev Emergency functions
     */
    function setThrottleMode(ThrottleMode mode, string calldata reason) external onlyOwner {
        ThrottleMode oldMode = currentThrottleMode;
        currentThrottleMode = mode;
        lastThrottleUpdate = block.timestamp;
        
        emit ThrottleModeChanged(oldMode, mode, reason);
    }
    
    function updateRouteConfig(RouteConfig calldata newConfig) external onlyOwner {
        routeConfig = newConfig;
    }
    
    function emergencyPauseNetwork(address network, string calldata reason) external onlyOwner {
        supportedNetworks[network] = false;
        rateLimits[network].backoffUntil = block.timestamp + 86400; // 24 hour backoff
        
        emit EmergencyRerouteActivated(network, reason);
    }
}