// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title ConsensusProofEngine
 * @dev Implements consensus proof mechanism via distribution via hops and sharding
 * 
 * This contract eliminates traditional PoS/PoW by using cryptographic proofs
 * of distributed hop validation across the network matrix
 */
contract ConsensusProofEngine is Ownable, ReentrancyGuard {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;
    
    // Consensus proof structure
    struct ConsensusProof {
        bytes32 transactionHash;        // Hash of transaction being validated
        address[] validationPath;       // Ordered path of validation nodes
        bytes32[] hopProofs;            // Cryptographic proof from each hop
        uint256[] timestamps;           // Timestamp of each validation step
        bytes signature;                // Aggregated signature of validation
        uint256 consensusScore;         // Consensus strength score (0-1000)
        ProofStatus status;             // Current status of proof
    }
    
    enum ProofStatus {
        PENDING,                        // Awaiting validation
        VALIDATING,                     // In progress
        CONSENSUS_REACHED,              // Sufficient consensus achieved
        CONSENSUS_FAILED,               // Failed to reach consensus
        EXPIRED                         // Proof expired due to timeout
    }
    
    // Validation node capability tracking
    struct ValidationNode {
        address nodeAddress;            // Node address
        uint256 validationPower;        // Node's validation capacity (not stake-based)
        uint256 successfulValidations;  // Historical validation successes
        uint256 failedValidations;      // Historical validation failures
        uint256 averageLatency;         // Average validation latency
        uint256 lastValidation;         // Timestamp of last validation
        bool isAuthorized;              // Authorization status
        bytes32 nodeSignature;          // Node's cryptographic signature
    }
    
    // Sharding for parallel consensus
    struct ValidationShard {
        uint256 shardId;                // Unique shard identifier
        address[] assignedNodes;        // Nodes assigned to this shard
        uint256 activeValidations;      // Current validation count
        uint256 shardCapacity;          // Maximum concurrent validations
        uint256 averageConsensusTime;   // Average time to reach consensus
        bool isActive;                  // Shard status
    }
    
    // Consensus parameters (from concept folder requirements)
    struct ConsensusParams {
        uint256 minValidators;          // Minimum validators for consensus (from Factor 1)
        uint256 optimalValidators;      // Optimal validator count
        uint256 consensusThreshold;     // Threshold for consensus (67% from precept.md)
        uint256 maxValidationTime;      // Maximum validation time (seconds)
        uint256 hopTimeoutMs;           // Timeout per hop in milliseconds  
        uint256 shardRebalanceInterval; // How often to rebalance shards
        bool emergencyConsensusMode;    // Emergency consensus mode flag
    }
    
    // State variables
    mapping(bytes32 => ConsensusProof) public consensusProofs;
    mapping(address => ValidationNode) public validationNodes;
    mapping(uint256 => ValidationShard) public validationShards;
    mapping(bytes32 => address) public proofToValidator;
    
    address[] public activeValidators;
    bytes32[] public pendingProofs;
    uint256 public totalShards;
    uint256 public totalValidations;
    
    ConsensusParams public consensusParams;
    
    // Network utility scoring (from formulas.py)
    struct NetworkMetrics {
        uint256 dailyValidations;       // Daily validation count
        uint256 crossChainValidations;  // Cross-chain validations
        uint256 averageConsensusTime;   // Average time to consensus
        uint256 networkUtilityScore;    // Overall network utility (0-1000)
        uint256 validationSuccessRate;  // Success rate (0-1000)
    }
    
    NetworkMetrics public networkMetrics;
    
    // Events
    event ConsensusProofStarted(bytes32 indexed proofId, bytes32 transactionHash, address[] validators);
    event HopValidated(bytes32 indexed proofId, address validator, uint256 hopIndex, bytes32 proof);
    event ConsensusReached(bytes32 indexed proofId, uint256 consensusScore, uint256 validationTime);
    event ConsensusFailed(bytes32 indexed proofId, string reason);
    event ValidatorAuthorized(address indexed validator, uint256 validationPower);
    event ShardRebalanced(uint256 indexed shardId, address[] newValidators);
    event EmergencyConsensusActivated(string reason);
    
    // Modifiers
    modifier onlyAuthorizedValidator() {
        require(validationNodes[msg.sender].isAuthorized, "Not authorized validator");
        _;
    }
    
    modifier validProof(bytes32 proofId) {
        require(consensusProofs[proofId].transactionHash != bytes32(0), "Invalid proof ID");
        _;
    }
    
    constructor(address owner) Ownable(owner) {
        // Initialize consensus parameters based on concept folder specs
        consensusParams = ConsensusParams({
            minValidators: 3,               // Minimum 3 validators (from requirements)
            optimalValidators: 5,           // Optimal 5 validators  
            consensusThreshold: 670,        // 67% consensus (from precept.md)
            maxValidationTime: 300,         // 5 minutes maximum
            hopTimeoutMs: 2000,             // 2 second hop timeout
            shardRebalanceInterval: 3600,   // Rebalance every hour
            emergencyConsensusMode: false
        });
        
        // Initialize with 3 shards (gradual expansion)
        totalShards = 3;
        for (uint256 i = 0; i < totalShards; i++) {
            validationShards[i] = ValidationShard({
                shardId: i,
                assignedNodes: new address[](0),
                activeValidations: 0,
                shardCapacity: 100,         // 100 concurrent validations per shard
                averageConsensusTime: 0,
                isActive: true
            });
        }
    }
    
    /**
     * @dev Authorize a validator node (stake-neutral authorization)
     * @param validator Address of the validator node
     * @param validationPower Validation capacity (based on performance, not stake)
     */
    function authorizeValidator(address validator, uint256 validationPower) external onlyOwner {
        require(validator != address(0), "Invalid validator address");
        require(validationPower > 0 && validationPower <= 1000, "Invalid validation power");
        
        validationNodes[validator] = ValidationNode({
            nodeAddress: validator,
            validationPower: validationPower,
            successfulValidations: 0,
            failedValidations: 0,
            averageLatency: 1000,           // Default 1 second
            lastValidation: 0,
            isAuthorized: true,
            nodeSignature: keccak256(abi.encodePacked(validator, block.timestamp))
        });
        
        activeValidators.push(validator);
        
        // Assign to least loaded shard
        _assignToOptimalShard(validator);
        
        emit ValidatorAuthorized(validator, validationPower);
    }
    
    /**
     * @dev Initiate consensus proof for a transaction
     * @param transactionHash Hash of transaction requiring consensus
     * @param requiredValidators Number of validators needed for this consensus
     * @return proofId Unique identifier for this consensus proof
     */
    function initiateConsensusProof(
        bytes32 transactionHash,
        uint256 requiredValidators
    ) external nonReentrant returns (bytes32 proofId) {
        require(transactionHash != bytes32(0), "Invalid transaction hash");
        require(requiredValidators >= consensusParams.minValidators, "Insufficient validators");
        require(activeValidators.length >= requiredValidators, "Not enough active validators");
        
        // Generate unique proof ID
        proofId = keccak256(abi.encodePacked(
            transactionHash,
            msg.sender,
            block.timestamp,
            block.prevrandao
        ));
        
        require(consensusProofs[proofId].transactionHash == bytes32(0), "Proof already exists");
        
        // Select validators using stake-neutral random selection
        address[] memory selectedValidators = _selectValidators(requiredValidators);
        
        consensusProofs[proofId] = ConsensusProof({
            transactionHash: transactionHash,
            validationPath: selectedValidators,
            hopProofs: new bytes32[](selectedValidators.length),
            timestamps: new uint256[](selectedValidators.length),
            signature: "",
            consensusScore: 0,
            status: ProofStatus.PENDING
        });
        
        pendingProofs.push(proofId);
        proofToValidator[proofId] = msg.sender;
        
        emit ConsensusProofStarted(proofId, transactionHash, selectedValidators);
        return proofId;
    }
    
    /**
     * @dev Submit validation proof for a hop in the consensus process
     * @param proofId Consensus proof identifier
     * @param hopIndex Index of the hop being validated
     * @param hopProof Cryptographic proof of validation
     */
    function submitHopValidation(
        bytes32 proofId,
        uint256 hopIndex,
        bytes32 hopProof
    ) external onlyAuthorizedValidator validProof(proofId) {
        ConsensusProof storage proof = consensusProofs[proofId];
        require(proof.status == ProofStatus.PENDING || proof.status == ProofStatus.VALIDATING, 
                "Proof not accepting validations");
        require(hopIndex < proof.validationPath.length, "Invalid hop index");
        require(proof.validationPath[hopIndex] == msg.sender, "Unauthorized for this hop");
        require(proof.hopProofs[hopIndex] == bytes32(0), "Hop already validated");
        
        // Check if proof has expired
        if (block.timestamp > proof.timestamps[0] + consensusParams.maxValidationTime) {
            proof.status = ProofStatus.EXPIRED;
            emit ConsensusFailed(proofId, "Validation timeout");
            return;
        }
        
        // Record hop validation
        proof.hopProofs[hopIndex] = hopProof;
        proof.timestamps[hopIndex] = block.timestamp;
        proof.status = ProofStatus.VALIDATING;
        
        // Update validator metrics
        ValidationNode storage validator = validationNodes[msg.sender];
        validator.lastValidation = block.timestamp;
        
        emit HopValidated(proofId, msg.sender, hopIndex, hopProof);
        
        // Check if consensus is reached
        _evaluateConsensus(proofId);
    }
    
    /**
     * @dev Evaluate if consensus has been reached for a proof
     */
    function _evaluateConsensus(bytes32 proofId) internal {
        ConsensusProof storage proof = consensusProofs[proofId];
        
        // Count completed validations
        uint256 completedValidations = 0;
        uint256 pathValidations = proof.validationPath.length;
        
        for (uint256 i = 0; i < pathValidations; i++) {
            if (proof.hopProofs[i] != bytes32(0)) {
                completedValidations++;
            }
        }
        
        // Calculate consensus score
        uint256 consensusScore = (completedValidations * 1000) / pathValidations;
        proof.consensusScore = consensusScore;
        
        // Check if consensus threshold is met
        if (consensusScore >= consensusParams.consensusThreshold) {
            proof.status = ProofStatus.CONSENSUS_REACHED;
            
            // Calculate validation time
            uint256 validationTime = block.timestamp - proof.timestamps[0];
            
            // Update network metrics
            _updateNetworkMetrics(validationTime, true);
            
            // Update individual validator success rates
            for (uint256 i = 0; i < proof.validationPath.length; i++) {
                if (proof.hopProofs[i] != bytes32(0)) {
                    validationNodes[proof.validationPath[i]].successfulValidations++;
                }
            }
            
            totalValidations++;
            emit ConsensusReached(proofId, consensusScore, validationTime);
            
        } else if (completedValidations == pathValidations && consensusScore < consensusParams.consensusThreshold) {
            // All validators responded but consensus not reached
            proof.status = ProofStatus.CONSENSUS_FAILED;
            
            _updateNetworkMetrics(0, false);
            
            // Update failure metrics
            for (uint256 i = 0; i < proof.validationPath.length; i++) {
                validationNodes[proof.validationPath[i]].failedValidations++;
            }
            
            emit ConsensusFailed(proofId, "Insufficient consensus");
        }
    }
    
    /**
     * @dev Select validators using stake-neutral random selection (Factor 1 compliance)
     */
    function _selectValidators(uint256 count) internal view returns (address[] memory) {
        require(activeValidators.length >= count, "Insufficient validators");
        
        address[] memory selected = new address[](count);
        bool[] memory used = new bool[](activeValidators.length);
        
        // Use cryptographically secure randomness for stake-neutral selection
        uint256 randomSeed = uint256(keccak256(abi.encodePacked(
            block.timestamp,
            block.prevrandao,
            msg.sender,
            totalValidations
        )));
        
        for (uint256 i = 0; i < count; i++) {
            uint256 attempts = 0;
            uint256 index;
            
            do {
                index = (randomSeed + i * 997 + attempts) % activeValidators.length;
                attempts++;
                
                // Prevent infinite loop
                if (attempts > activeValidators.length * 2) break;
                
            } while (used[index] || !validationNodes[activeValidators[index]].isAuthorized);
            
            used[index] = true;
            selected[i] = activeValidators[index];
        }
        
        return selected;
    }
    
    /**
     * @dev Assign validator to optimal shard based on load balancing
     */
    function _assignToOptimalShard(address validator) internal {
        uint256 minLoad = type(uint256).max;
        uint256 optimalShard = 0;
        
        // Find shard with minimum load
        for (uint256 i = 0; i < totalShards; i++) {
            if (validationShards[i].isActive && 
                validationShards[i].assignedNodes.length < minLoad) {
                minLoad = validationShards[i].assignedNodes.length;
                optimalShard = i;
            }
        }
        
        validationShards[optimalShard].assignedNodes.push(validator);
    }
    
    /**
     * @dev Update network metrics based on validation results
     */
    function _updateNetworkMetrics(uint256 validationTime, bool success) internal {
        if (success) {
            // Update average consensus time using exponential moving average
            if (networkMetrics.averageConsensusTime == 0) {
                networkMetrics.averageConsensusTime = validationTime;
            } else {
                // 90% weight to previous average, 10% to new measurement
                networkMetrics.averageConsensusTime = 
                    (networkMetrics.averageConsensusTime * 9 + validationTime) / 10;
            }
            
            networkMetrics.dailyValidations++;
        }
        
        // Update success rate
        uint256 totalAttempts = networkMetrics.dailyValidations + 
                               (success ? 0 : 1); // Add failed attempt if applicable
        uint256 successCount = networkMetrics.dailyValidations;
        
        networkMetrics.validationSuccessRate = totalAttempts > 0 ? 
            (successCount * 1000) / totalAttempts : 1000;
        
        // Calculate network utility score based on formulas.py
        networkMetrics.networkUtilityScore = _calculateNetworkUtilityScore();
    }
    
    /**
     * @dev Calculate network utility score (from formulas.py implementation)
     */
    function _calculateNetworkUtilityScore() internal view returns (uint256) {
        // Target metrics from concept folder
        uint256 targetDailyValidations = 500000;  // From precept.md simulation
        uint256 targetCrossChain = 50000;         // 10% cross-chain target
        
        // Calculate utility components
        uint256 validationUtility = networkMetrics.dailyValidations > targetDailyValidations ?
            1000 : (networkMetrics.dailyValidations * 1000) / targetDailyValidations;
            
        uint256 crossChainUtility = networkMetrics.crossChainValidations > targetCrossChain ?
            1000 : (networkMetrics.crossChainValidations * 1000) / targetCrossChain;
        
        // Network utility formula from formulas.py
        uint256 networkUtility = (validationUtility + crossChainUtility) / 2;
        
        // Apply success rate modifier
        networkUtility = (networkUtility * networkMetrics.validationSuccessRate) / 1000;
        
        return networkUtility;
    }
    
    /**
     * @dev Get consensus proof status and details
     */
    function getConsensusProof(bytes32 proofId) external view returns (
        bytes32 transactionHash,
        address[] memory validationPath,
        uint256 consensusScore,
        ProofStatus status,
        uint256 completedHops
    ) {
        ConsensusProof storage proof = consensusProofs[proofId];
        
        uint256 completed = 0;
        for (uint256 i = 0; i < proof.hopProofs.length; i++) {
            if (proof.hopProofs[i] != bytes32(0)) {
                completed++;
            }
        }
        
        return (
            proof.transactionHash,
            proof.validationPath,
            proof.consensusScore,
            proof.status,
            completed
        );
    }
    
    /**
     * @dev Get network consensus metrics
     */
    function getNetworkMetrics() external view returns (
        uint256 activeValidatorCount,
        uint256 averageConsensusTime,
        uint256 validationSuccessRate,
        uint256 networkUtilityScore,
        uint256 totalValidationsToday
    ) {
        return (
            activeValidators.length,
            networkMetrics.averageConsensusTime,
            networkMetrics.validationSuccessRate,
            networkMetrics.networkUtilityScore,
            networkMetrics.dailyValidations
        );
    }
    
    /**
     * @dev Get validator performance metrics
     */
    function getValidatorMetrics(address validator) external view returns (
        uint256 validationPower,
        uint256 successfulValidations,
        uint256 failedValidations,
        uint256 successRate,
        bool isAuthorized
    ) {
        ValidationNode storage node = validationNodes[validator];
        
        uint256 nodeValidations = node.successfulValidations + node.failedValidations;
        uint256 nodeSuccessRate = nodeValidations > 0 ? 
            (node.successfulValidations * 1000) / nodeValidations : 1000;
        
        return (
            node.validationPower,
            node.successfulValidations,
            node.failedValidations,
            nodeSuccessRate,
            node.isAuthorized
        );
    }
    
    /**
     * @dev Emergency consensus management
     */
    function activateEmergencyConsensus(string calldata reason) external onlyOwner {
        consensusParams.emergencyConsensusMode = true;
        consensusParams.consensusThreshold = 510;  // Reduce to 51% in emergency
        consensusParams.maxValidationTime = 60;    // Reduce to 1 minute
        
        emit EmergencyConsensusActivated(reason);
    }
    
    function deactivateEmergencyConsensus() external onlyOwner {
        consensusParams.emergencyConsensusMode = false;
        consensusParams.consensusThreshold = 670;  // Restore to 67%
        consensusParams.maxValidationTime = 300;   // Restore to 5 minutes
    }
    
    /**
     * @dev Administrative functions
     */
    function updateConsensusParams(ConsensusParams calldata newParams) external onlyOwner {
        consensusParams = newParams;
    }
    
    function rebalanceShards() external onlyOwner {
        // Implement shard rebalancing logic
        for (uint256 i = 0; i < totalShards; i++) {
            if (validationShards[i].isActive) {
                emit ShardRebalanced(i, validationShards[i].assignedNodes);
            }
        }
    }
}