// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title MockTimelock
 * @dev Mock timelock contract for testing governance functionality
 */
contract MockTimelock {
    uint256 public constant MINIMUM_DELAY = 1 hours;
    uint256 public constant MAXIMUM_DELAY = 30 days;
    uint256 public constant GRACE_PERIOD = 14 days;
    
    address public admin;
    address public pendingAdmin;
    uint256 public delay;
    
    mapping(bytes32 => bool) public queuedTransactions;
    
    event NewAdmin(address indexed newAdmin);
    event NewPendingAdmin(address indexed newPendingAdmin);
    event NewDelay(uint256 indexed newDelay);
    event QueueTransaction(
        bytes32 indexed txHash,
        address indexed target,
        uint256 value,
        string signature,
        bytes data,
        uint256 eta
    );
    event CancelTransaction(
        bytes32 indexed txHash,
        address indexed target,
        uint256 value,
        string signature,
        bytes data,
        uint256 eta
    );
    event ExecuteTransaction(
        bytes32 indexed txHash,
        address indexed target,
        uint256 value,
        string signature,
        bytes data,
        uint256 eta
    );
    
    modifier onlyAdmin() {
        require(msg.sender == admin, "Timelock: Call must come from admin");
        _;
    }
    
    constructor() {
        admin = msg.sender;
        delay = MINIMUM_DELAY;
    }
    
    receive() external payable {}
    
    function setDelay(uint256 delay_) external onlyAdmin {
        require(
            delay_ >= MINIMUM_DELAY && delay_ <= MAXIMUM_DELAY,
            "Timelock: Delay must be between minimum and maximum"
        );
        delay = delay_;
        emit NewDelay(delay);
    }
    
    function acceptAdmin() external {
        require(msg.sender == pendingAdmin, "Timelock: Call must come from pendingAdmin");
        admin = msg.sender;
        pendingAdmin = address(0);
        emit NewAdmin(admin);
    }
    
    function setPendingAdmin(address pendingAdmin_) external onlyAdmin {
        pendingAdmin = pendingAdmin_;
        emit NewPendingAdmin(pendingAdmin);
    }
    
    function queueTransaction(
        address target,
        uint256 value,
        string memory signature,
        bytes memory data,
        uint256 eta
    ) external onlyAdmin returns (bytes32) {
        require(eta >= block.timestamp + delay, "Timelock: ETA must be in the future");
        
        bytes32 txHash = keccak256(abi.encode(target, value, signature, data, eta));
        queuedTransactions[txHash] = true;
        
        emit QueueTransaction(txHash, target, value, signature, data, eta);
        return txHash;
    }
    
    function cancelTransaction(
        address target,
        uint256 value,
        string memory signature,
        bytes memory data,
        uint256 eta
    ) external onlyAdmin {
        bytes32 txHash = keccak256(abi.encode(target, value, signature, data, eta));
        queuedTransactions[txHash] = false;
        
        emit CancelTransaction(txHash, target, value, signature, data, eta);
    }
    
    function executeTransaction(
        address target,
        uint256 value,
        string memory signature,
        bytes memory data,
        uint256 eta
    ) external onlyAdmin returns (bytes memory) {
        bytes32 txHash = keccak256(abi.encode(target, value, signature, data, eta));
        
        require(queuedTransactions[txHash], "Timelock: Transaction not queued");
        require(block.timestamp >= eta, "Timelock: Transaction not ready");
        require(block.timestamp <= eta + GRACE_PERIOD, "Timelock: Transaction expired");
        
        queuedTransactions[txHash] = false;
        
        bytes memory callData;
        
        if (bytes(signature).length == 0) {
            callData = data;
        } else {
            callData = abi.encodePacked(bytes4(keccak256(bytes(signature))), data);
        }
        
        (bool success, bytes memory returnData) = target.call{value: value}(callData);
        require(success, "Timelock: Transaction execution reverted");
        
        emit ExecuteTransaction(txHash, target, value, signature, data, eta);
        
        return returnData;
    }
    
    // Mock functions for testing
    function fastTrackExecution(
        address target,
        uint256 value,
        string memory signature,
        bytes memory data
    ) external onlyAdmin returns (bytes memory) {
        bytes memory callData;
        
        if (bytes(signature).length == 0) {
            callData = data;
        } else {
            callData = abi.encodePacked(bytes4(keccak256(bytes(signature))), data);
        }
        
        (bool success, bytes memory returnData) = target.call{value: value}(callData);
        require(success, "Timelock: Fast track execution reverted");
        
        return returnData;
    }
}
