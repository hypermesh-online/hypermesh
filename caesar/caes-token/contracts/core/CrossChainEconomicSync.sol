// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLibManager.sol";
import "@layerzerolabs/lz-evm-oapp-v2/contracts/oapp/OApp.sol";
import "../interfaces/IEconomicEngine.sol";
import "../libs/AdvancedMathUtils.sol";

/**
 * @title CrossChainEconomicSync
 * @dev LayerZero V2 based cross-chain economic parameter synchronization system
 * Enables real-time synchronization of economic parameters, stability metrics,
 * and intervention commands across all supported chains for Caesar Token
 */
contract CrossChainEconomicSync is OApp, ReentrancyGuard, Pausable {
    using AdvancedMathUtils for uint256;
    
    // ============ Constants ============
    uint256 public constant MAX_BATCH_SIZE = 50;
    uint256 public constant SYNC_TIMEOUT = 1 hours;
    uint256 public constant PARAMETER_VERSION = 1;
    uint256 public constant MAX_RETRIES = 3;
    
    // ============ Message Types ============
    uint8 public constant MSG_PARAMETER_SYNC = 1;
    uint8 public constant MSG_STABILITY_UPDATE = 2;
    uint8 public constant MSG_EMERGENCY_BROADCAST = 3;
    uint8 public constant MSG_INTERVENTION_COMMAND = 4;
    uint8 public constant MSG_HEALTH_METRICS = 5;
    uint8 public constant MSG_BATCH_SYNC = 6;
    
    // ============ Structs ============
    
    struct CrossChainParameters {
        uint256 baseDemurrageRate;           // Base demurrage rate
        uint256 maxDemurrageRate;            // Maximum demurrage rate
        uint256 stabilityThreshold;          // Stability threshold
        uint256 fiatDiscountFactor;          // Fiat discount factor
        uint256 antiSpeculationPenaltyRate;  // Anti-speculation penalty rate
        uint256 circuitBreakerThreshold;     // Circuit breaker threshold
        uint256 rebalanceFrequency;          // Rebalance frequency
        uint256 timestamp;                   // Parameter timestamp
        bytes32 parameterHash;               // Hash for integrity verification
    }
    
    struct StabilityMetrics {
        uint256 priceStability;              // Price stability index
        uint256 liquidityHealth;             // Liquidity health score
        uint256 participationRate;           // Network participation rate
        uint256 reserveRatio;                // Reserve backing ratio
        uint256 overallHealth;               // Overall system health
        uint256 timestamp;                   // Metrics timestamp
        uint32 sourceChain;                  // Source chain ID
    }
    
    struct EmergencyBroadcast {
        uint8 emergencyType;                 // Type of emergency (pause, circuit breaker, etc.)
        uint256 severity;                    // Emergency severity level
        uint256 duration;                    // Duration of emergency measures
        string reason;                       // Emergency reason
        address initiator;                   // Emergency initiator
        uint256 timestamp;                   // Broadcast timestamp
    }
    
    struct InterventionCommand {
        uint8 commandType;                   // Type of intervention
        uint256 targetParameter;             // Parameter to adjust
        uint256 newValue;                    // New parameter value
        uint256 executionTime;               // When to execute (0 = immediate)
        bool requiresConsensus;              // Whether consensus is required
        uint256 consensusThreshold;          // Consensus threshold if required
        bytes additionalData;                // Additional command data
    }
    
    struct SyncState {
        uint256 lastSyncTime;                // Last sync timestamp
        uint256 syncNonce;                   // Sync nonce for ordering
        bytes32 lastParameterHash;           // Last parameter hash
        bool isPending;                      // Whether sync is pending
        uint256 retryCount;                  // Number of retry attempts
        uint32[] targetChains;               // Target chains for sync
    }
    
    struct ChainStatus {
        bool isActive;                       // Whether chain is active
        uint256 lastHeartbeat;               // Last heartbeat timestamp
        uint256 latency;                     // Average message latency
        uint256 failureCount;                // Number of failures
        StabilityMetrics lastMetrics;        // Last received stability metrics
        CrossChainParameters lastParameters; // Last synchronized parameters
    }
    
    struct BatchSyncData {
        CrossChainParameters[] parameters;   // Batch of parameters
        StabilityMetrics[] metrics;          // Batch of metrics
        uint256 batchId;                     // Unique batch ID
        uint256 totalBatches;                // Total number of batches
        uint256 batchIndex;                  // Index of this batch
    }
    
    // ============ State Variables ============
    
    // Core economic engine reference
    IEconomicEngine public economicEngine;
    
    // Cross-chain state management
    mapping(uint32 => ChainStatus) public chainStatus;
    mapping(uint32 => SyncState) public chainSyncState;
    mapping(bytes32 => bool) public processedMessages;
    mapping(uint32 => uint256) public chainLatency;
    
    // Parameter synchronization
    CrossChainParameters public masterParameters;
    StabilityMetrics public aggregatedMetrics;
    uint256 public lastParameterUpdate;
    uint256 public lastMetricsAggregation;
    
    // Emergency management
    EmergencyBroadcast public activeEmergency;
    bool public isEmergencyActive;
    mapping(address => bool) public emergencyOperators;
    
    // Consensus system
    mapping(bytes32 => mapping(uint32 => bool)) public consensusVotes;
    mapping(bytes32 => uint256) public consensusCount;
    mapping(bytes32 => InterventionCommand) public pendingInterventions;
    
    // Performance metrics
    uint256 public totalMessagesSent;
    uint256 public totalMessagesReceived;
    uint256 public failedSyncAttempts;
    uint256 public averageSyncTime;
    
    // Supported chains
    uint32[] public supportedChains;
    mapping(uint32 => bool) public isSupportedChain;
    
    // ============ Events ============
    
    event ParametersSynchronized(uint32[] chains, bytes32 parameterHash);
    event StabilityMetricsReceived(uint32 fromChain, StabilityMetrics metrics);
    event EmergencyBroadcastSent(uint8 emergencyType, uint32[] targetChains);
    event InterventionCommandExecuted(bytes32 commandId, uint8 commandType);
    event ChainStatusUpdated(uint32 chainId, bool isActive, uint256 latency);
    event ConsensusReached(bytes32 interventionId, uint256 votes);
    event SyncFailure(uint32 chainId, string reason, uint256 retryCount);
    event BatchSyncCompleted(uint256 batchId, uint32[] chains);
    event CrossChainMessage(uint32 fromChain, uint32 toChain, uint8 messageType);
    
    /**
     * @dev Constructor
     * @param _endpoint LayerZero endpoint address
     * @param _owner Contract owner
     * @param _economicEngine Economic engine address
     */
    constructor(
        address _endpoint,
        address _owner,
        address _economicEngine
    ) OApp(_endpoint, _owner) Ownable(_owner) {
        economicEngine = IEconomicEngine(_economicEngine);
        lastParameterUpdate = block.timestamp;
        lastMetricsAggregation = block.timestamp;
    }
    
    // ============ Core Synchronization Functions ============
    
    /**
     * @dev Synchronize economic parameters across all chains
     * @param parameters Parameters to synchronize
     * @param targetChains Target chains (empty for all chains)
     */
    function synchronizeParameters(
        CrossChainParameters calldata parameters,
        uint32[] calldata targetChains
    ) external onlyOwner nonReentrant whenNotPaused {
        require(parameters.timestamp > lastParameterUpdate, "Parameters are not newer");
        
        // Validate parameter integrity
        bytes32 parameterHash = _calculateParameterHash(parameters);
        require(parameterHash == parameters.parameterHash, "Parameter hash mismatch");
        
        // Update master parameters
        masterParameters = parameters;
        lastParameterUpdate = block.timestamp;
        
        // Determine target chains
        uint32[] memory chains;
        if (targetChains.length > 0) {
            chains = targetChains;
        } else {
            chains = supportedChains;
        }
        
        // Send to each target chain
        for (uint256 i = 0; i < chains.length; i++) {
            if (chains[i] != 0 && isSupportedChain[chains[i]]) {
                _sendParameterSync(chains[i], parameters);
            }
        }
        
        totalMessagesSent += chains.length;
        emit ParametersSynchronized(chains, parameterHash);
    }
    
    /**
     * @dev Broadcast stability metrics to all chains
     * @param metrics Stability metrics to broadcast
     */
    function broadcastStabilityMetrics(
        StabilityMetrics calldata metrics
    ) external nonReentrant whenNotPaused {
        require(
            msg.sender == address(economicEngine) || msg.sender == owner(),
            "Unauthorized"
        );
        
        // Update aggregated metrics
        _updateAggregatedMetrics(metrics);
        
        // Broadcast to all chains
        for (uint256 i = 0; i < supportedChains.length; i++) {
            uint32 chainId = supportedChains[i];
            if (chainId != 0) {
                _sendStabilityUpdate(chainId, metrics);
            }
        }
        
        totalMessagesSent += supportedChains.length;
    }
    
    /**
     * @dev Send emergency broadcast to all chains
     * @param broadcast Emergency broadcast data
     */
    function sendEmergencyBroadcast(
        EmergencyBroadcast calldata broadcast
    ) external nonReentrant whenNotPaused {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        
        activeEmergency = broadcast;
        isEmergencyActive = true;
        
        // Send to all chains
        uint32[] memory chains = supportedChains;
        for (uint256 i = 0; i < chains.length; i++) {
            if (chains[i] != 0) {
                _sendEmergencyBroadcast(chains[i], broadcast);
            }
        }
        
        totalMessagesSent += chains.length;
        emit EmergencyBroadcastSent(broadcast.emergencyType, chains);
    }
    
    /**
     * @dev Send intervention command with optional consensus requirement
     * @param command Intervention command data
     * @param targetChains Target chains for intervention
     */
    function sendInterventionCommand(
        InterventionCommand calldata command,
        uint32[] calldata targetChains
    ) external onlyOwner nonReentrant whenNotPaused {
        bytes32 commandId = keccak256(abi.encode(command, block.timestamp));
        
        if (command.requiresConsensus) {
            pendingInterventions[commandId] = command;
            // Start consensus process
            _initiateConsensus(commandId, command);
        } else {
            // Execute immediately
            _executeInterventionCommand(commandId, command, targetChains);
        }
    }
    
    /**
     * @dev Batch synchronization for efficiency
     * @param batchData Batch synchronization data
     * @param targetChains Target chains
     */
    function batchSync(
        BatchSyncData calldata batchData,
        uint32[] calldata targetChains
    ) external onlyOwner nonReentrant whenNotPaused {
        require(batchData.parameters.length <= MAX_BATCH_SIZE, "Batch too large");
        require(batchData.batchIndex < batchData.totalBatches, "Invalid batch index");
        
        // Send batch to each target chain
        for (uint256 i = 0; i < targetChains.length; i++) {
            if (isSupportedChain[targetChains[i]]) {
                _sendBatchSync(targetChains[i], batchData);
            }
        }
        
        emit BatchSyncCompleted(batchData.batchId, targetChains);
    }
    
    // ============ LayerZero Message Handling ============
    
    /**
     * @dev Handle incoming LayerZero messages
     * @param _origin Message origin information
     * @param _message Encoded message data
     */
    function _lzReceive(
        Origin calldata _origin,
        bytes32 /*_guid*/,
        bytes calldata _message,
        address /*_executor*/,
        bytes calldata /*_extraData*/
    ) internal override {
        uint32 fromChain = _origin.srcEid;
        totalMessagesReceived++;
        
        // Decode message type
        uint8 messageType = uint8(_message[0]);
        bytes calldata messageData = _message[1:];
        
        // Update chain heartbeat
        chainStatus[fromChain].lastHeartbeat = block.timestamp;
        
        // Route message based on type
        if (messageType == MSG_PARAMETER_SYNC) {
            _handleParameterSync(fromChain, messageData);
        } else if (messageType == MSG_STABILITY_UPDATE) {
            _handleStabilityUpdate(fromChain, messageData);
        } else if (messageType == MSG_EMERGENCY_BROADCAST) {
            _handleEmergencyBroadcast(fromChain, messageData);
        } else if (messageType == MSG_INTERVENTION_COMMAND) {
            _handleInterventionCommand(fromChain, messageData);
        } else if (messageType == MSG_HEALTH_METRICS) {
            _handleHealthMetrics(fromChain, messageData);
        } else if (messageType == MSG_BATCH_SYNC) {
            _handleBatchSync(fromChain, messageData);
        }
        
        emit CrossChainMessage(fromChain, uint32(block.chainid), messageType);
    }
    
    // ============ Message Handlers ============
    
    function _handleParameterSync(uint32 fromChain, bytes calldata data) internal {
        CrossChainParameters memory parameters = abi.decode(data, (CrossChainParameters));
        
        // Verify parameter hash
        bytes32 expectedHash = _calculateParameterHash(parameters);
        require(expectedHash == parameters.parameterHash, "Invalid parameter hash");
        
        // Update chain status
        chainStatus[fromChain].lastParameters = parameters;
        
        // Apply parameters to local economic engine if newer
        if (parameters.timestamp > lastParameterUpdate) {
            _applyParametersToEngine(parameters);
            lastParameterUpdate = parameters.timestamp;
        }
    }
    
    function _handleStabilityUpdate(uint32 fromChain, bytes calldata data) internal {
        StabilityMetrics memory metrics = abi.decode(data, (StabilityMetrics));
        
        // Update chain status
        chainStatus[fromChain].lastMetrics = metrics;
        
        // Update aggregated metrics
        _updateAggregatedMetrics(metrics);
        
        emit StabilityMetricsReceived(fromChain, metrics);
    }
    
    function _handleEmergencyBroadcast(uint32 fromChain, bytes calldata data) internal {
        EmergencyBroadcast memory broadcast = abi.decode(data, (EmergencyBroadcast));
        
        // Process emergency if more severe than current
        if (!isEmergencyActive || broadcast.severity > activeEmergency.severity) {
            activeEmergency = broadcast;
            isEmergencyActive = true;
            
            // Apply emergency measures locally
            _applyEmergencyMeasures(broadcast);
        }
    }
    
    function _handleInterventionCommand(uint32 fromChain, bytes calldata data) internal {
        (bytes32 commandId, InterventionCommand memory command) = abi.decode(data, (bytes32, InterventionCommand));
        
        if (command.requiresConsensus) {
            // Record consensus vote
            consensusVotes[commandId][fromChain] = true;
            consensusCount[commandId]++;
            
            // Check if consensus reached
            if (consensusCount[commandId] >= command.consensusThreshold) {
                _executeInterventionCommand(commandId, command, supportedChains);
                emit ConsensusReached(commandId, consensusCount[commandId]);
            }
        } else {
            // Execute immediately
            _executeInterventionCommand(commandId, command, supportedChains);
        }
    }
    
    function _handleHealthMetrics(uint32 fromChain, bytes calldata data) internal {
        StabilityMetrics memory metrics = abi.decode(data, (StabilityMetrics));
        chainStatus[fromChain].lastMetrics = metrics;
    }
    
    function _handleBatchSync(uint32 fromChain, bytes calldata data) internal {
        BatchSyncData memory batchData = abi.decode(data, (BatchSyncData));
        
        // Process batch data
        for (uint256 i = 0; i < batchData.parameters.length; i++) {
            if (batchData.parameters[i].timestamp > lastParameterUpdate) {
                _applyParametersToEngine(batchData.parameters[i]);
            }
        }
        
        for (uint256 i = 0; i < batchData.metrics.length; i++) {
            _updateAggregatedMetrics(batchData.metrics[i]);
        }
    }
    
    // ============ Internal Functions ============
    
    function _sendParameterSync(uint32 targetChain, CrossChainParameters memory parameters) internal {
        bytes memory message = abi.encodePacked(MSG_PARAMETER_SYNC, abi.encode(parameters));
        _sendMessage(targetChain, message);
        
        // Update sync state
        chainSyncState[targetChain].lastSyncTime = block.timestamp;
        chainSyncState[targetChain].syncNonce++;
        chainSyncState[targetChain].lastParameterHash = parameters.parameterHash;
    }
    
    function _sendStabilityUpdate(uint32 targetChain, StabilityMetrics memory metrics) internal {
        bytes memory message = abi.encodePacked(MSG_STABILITY_UPDATE, abi.encode(metrics));
        _sendMessage(targetChain, message);
    }
    
    function _sendEmergencyBroadcast(uint32 targetChain, EmergencyBroadcast memory broadcast) internal {
        bytes memory message = abi.encodePacked(MSG_EMERGENCY_BROADCAST, abi.encode(broadcast));
        _sendMessage(targetChain, message);
    }
    
    function _sendInterventionCommand(
        uint32 targetChain,
        bytes32 commandId,
        InterventionCommand memory command
    ) internal {
        bytes memory message = abi.encodePacked(MSG_INTERVENTION_COMMAND, abi.encode(commandId, command));
        _sendMessage(targetChain, message);
    }
    
    function _sendBatchSync(uint32 targetChain, BatchSyncData memory batchData) internal {
        bytes memory message = abi.encodePacked(MSG_BATCH_SYNC, abi.encode(batchData));
        _sendMessage(targetChain, message);
    }
    
    function _sendMessage(uint32 targetChain, bytes memory message) internal {
        // Create LayerZero V2 options manually (TYPE_3 with executor gas limit)
        bytes memory options = abi.encodePacked(
            uint16(3), // TYPE_3
            uint16(1), // Executor option type
            uint16(16), // Option data length (8 + 8 bytes)
            uint64(200000), // Gas limit
            uint64(0) // Value
        );
        MessagingFee memory fee = _quote(targetChain, message, options, false);
        
        _lzSend(
            targetChain,
            message,
            options,
            MessagingFee(fee.nativeFee, 0),
            payable(msg.sender)
        );
    }
    
    function _calculateParameterHash(CrossChainParameters memory parameters) internal pure returns (bytes32) {
        return keccak256(abi.encode(
            parameters.baseDemurrageRate,
            parameters.maxDemurrageRate,
            parameters.stabilityThreshold,
            parameters.fiatDiscountFactor,
            parameters.antiSpeculationPenaltyRate,
            parameters.circuitBreakerThreshold,
            parameters.rebalanceFrequency,
            parameters.timestamp
        ));
    }
    
    function _applyParametersToEngine(CrossChainParameters memory parameters) internal {
        // Convert to economic engine parameters format
        IEconomicEngine.EconomicParameters memory engineParams = IEconomicEngine.EconomicParameters({
            baseDemurrageRate: parameters.baseDemurrageRate,
            maxDemurrageRate: parameters.maxDemurrageRate,
            stabilityThreshold: parameters.stabilityThreshold,
            fiatDiscountFactor: parameters.fiatDiscountFactor,
            gracePeriodsHours: 48, // Default value
            interventionThreshold: 500, // Default value
            rebalanceFrequency: parameters.rebalanceFrequency,
            emergencyThreshold: parameters.circuitBreakerThreshold
        });
        
        economicEngine.updateEconomicParameters(engineParams);
    }
    
    function _updateAggregatedMetrics(StabilityMetrics memory metrics) internal {
        // Simple aggregation - in practice would use more sophisticated algorithms
        if (aggregatedMetrics.timestamp == 0) {
            aggregatedMetrics = metrics;
        } else {
            aggregatedMetrics.priceStability = (aggregatedMetrics.priceStability + metrics.priceStability) / 2;
            aggregatedMetrics.liquidityHealth = (aggregatedMetrics.liquidityHealth + metrics.liquidityHealth) / 2;
            aggregatedMetrics.participationRate = (aggregatedMetrics.participationRate + metrics.participationRate) / 2;
            aggregatedMetrics.reserveRatio = (aggregatedMetrics.reserveRatio + metrics.reserveRatio) / 2;
            aggregatedMetrics.overallHealth = (aggregatedMetrics.overallHealth + metrics.overallHealth) / 2;
        }
        
        aggregatedMetrics.timestamp = block.timestamp;
        lastMetricsAggregation = block.timestamp;
    }
    
    function _applyEmergencyMeasures(EmergencyBroadcast memory broadcast) internal {
        if (broadcast.emergencyType == 1) { // Pause operations
            _pause();
        } else if (broadcast.emergencyType == 2) { // Circuit breaker
            // Activate circuit breaker in economic engine
            economicEngine.activateEmergencyMode("Cross-chain emergency");
        }
        // Add more emergency measure types as needed
    }
    
    function _initiateConsensus(bytes32 commandId, InterventionCommand memory command) internal {
        pendingInterventions[commandId] = command;
        
        // Send consensus request to all chains
        for (uint256 i = 0; i < supportedChains.length; i++) {
            if (supportedChains[i] != 0) {
                _sendInterventionCommand(supportedChains[i], commandId, command);
            }
        }
    }
    
    function _executeInterventionCommand(
        bytes32 commandId,
        InterventionCommand memory command,
        uint32[] memory targetChains
    ) internal {
        // Apply intervention locally first
        _applyInterventionLocally(command);
        
        // Send to target chains if not already done
        for (uint256 i = 0; i < targetChains.length; i++) {
            if (targetChains[i] != uint32(block.chainid)) {
                _sendInterventionCommand(targetChains[i], commandId, command);
            }
        }
        
        emit InterventionCommandExecuted(commandId, command.commandType);
    }
    
    function _applyInterventionLocally(InterventionCommand memory command) internal {
        if (command.commandType == 1) { // Parameter adjustment
            // Apply parameter change to economic engine
            // Implementation depends on specific parameter being adjusted
        } else if (command.commandType == 2) { // Emergency activation
            economicEngine.activateEmergencyMode("Intervention command");
        }
        // Add more intervention types as needed
    }
    
    // ============ Administrative Functions ============
    
    /**
     * @dev Add supported chain
     * @param chainId Chain ID to add
     */
    function addSupportedChain(uint32 chainId) external onlyOwner {
        require(!isSupportedChain[chainId], "Chain already supported");
        
        supportedChains.push(chainId);
        isSupportedChain[chainId] = true;
        
        // Initialize chain status
        chainStatus[chainId] = ChainStatus({
            isActive: true,
            lastHeartbeat: block.timestamp,
            latency: 0,
            failureCount: 0,
            lastMetrics: StabilityMetrics(0, 0, 0, 0, 0, 0, 0),
            lastParameters: CrossChainParameters(0, 0, 0, 0, 0, 0, 0, 0, bytes32(0))
        });
        
        emit ChainStatusUpdated(chainId, true, 0);
    }
    
    /**
     * @dev Remove supported chain
     * @param chainId Chain ID to remove
     */
    function removeSupportedChain(uint32 chainId) external onlyOwner {
        require(isSupportedChain[chainId], "Chain not supported");
        
        // Remove from supported chains array
        for (uint256 i = 0; i < supportedChains.length; i++) {
            if (supportedChains[i] == chainId) {
                supportedChains[i] = supportedChains[supportedChains.length - 1];
                supportedChains.pop();
                break;
            }
        }
        
        isSupportedChain[chainId] = false;
        chainStatus[chainId].isActive = false;
        
        emit ChainStatusUpdated(chainId, false, 0);
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
     * @dev Update economic engine reference
     * @param _economicEngine New economic engine address
     */
    function updateEconomicEngine(address _economicEngine) external onlyOwner {
        require(_economicEngine != address(0), "Invalid address");
        economicEngine = IEconomicEngine(_economicEngine);
    }
    
    // ============ View Functions ============
    
    function getSupportedChains() external view returns (uint32[] memory) {
        return supportedChains;
    }
    
    function getChainStatus(uint32 chainId) external view returns (ChainStatus memory) {
        return chainStatus[chainId];
    }
    
    function getSyncState(uint32 chainId) external view returns (SyncState memory) {
        return chainSyncState[chainId];
    }
    
    function getMasterParameters() external view returns (CrossChainParameters memory) {
        return masterParameters;
    }
    
    function getAggregatedMetrics() external view returns (StabilityMetrics memory) {
        return aggregatedMetrics;
    }
    
    function getActiveEmergency() external view returns (EmergencyBroadcast memory) {
        return activeEmergency;
    }
    
    function getSyncMetrics() external view returns (
        uint256 messagesSent,
        uint256 messagesReceived,
        uint256 failedSyncs,
        uint256 avgSyncTime
    ) {
        return (totalMessagesSent, totalMessagesReceived, failedSyncAttempts, averageSyncTime);
    }
    
    // ============ Emergency Functions ============
    
    /**
     * @dev Emergency pause
     */
    function pause() external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        _pause();
    }
    
    /**
     * @dev Emergency unpause
     */
    function unpause() external onlyOwner {
        _unpause();
    }
    
    /**
     * @dev Emergency clear active emergency
     */
    function clearEmergency() external onlyOwner {
        isEmergencyActive = false;
        delete activeEmergency;
    }
}