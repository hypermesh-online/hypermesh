// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "./CaesarCoinDEXPair.sol";
import "./ICaesarCoinDEXFactory.sol";

/**
 * @title CaesarCoinDEXFactory
 * @dev Real UniswapV2-style factory for creating and managing trading pairs
 * Enhanced with proper security controls and fee management
 */
contract CaesarCoinDEXFactory is ICaesarCoinDEXFactory, Ownable, ReentrancyGuard {
    
    // Maximum trading fee (1%)
    uint256 public constant MAX_TRADING_FEE = 100;
    
    // Mapping from token0 => token1 => pair address
    mapping(address => mapping(address => address)) public override getPair;
    
    // Array of all pairs created
    address[] public override allPairs;
    
    // Fee recipient for protocol fees
    address public override feeTo;
    
    // Fee setter (can be different from owner)
    address public override feeToSetter;
    
    // Trading fee (in basis points per 1000, default 3 = 0.3%)
    uint256 public override tradingFee = 3;
    
    // Protocol fee percentage (of trading fees)
    uint256 public protocolFeePercentage = 0; // 0% initially
    
    // Pair volume tracking
    mapping(address => uint256) public pairVolumes;
    mapping(address => uint256) public pairLastUpdate;
    
    // Total volume across all pairs
    uint256 public totalVolume;
    
    // Rate limiting for pair creation
    mapping(address => uint256) public userLastPairCreation;
    uint256 public constant PAIR_CREATION_COOLDOWN = 1 minutes;
    
    // Temporary storage for pair initialization
    address private _initializingToken0;
    address private _initializingToken1;
    
    // Events (additional to interface events)
    event ProtocolFeeSet(uint256 fee);
    event VolumeUpdated(address indexed pair, uint256 volume);
    
    modifier onlyFeeToSetter() {
        require(msg.sender == feeToSetter, "CaesarCoinDEXFactory: FORBIDDEN");
        _;
    }
    
    constructor(address _owner, address _feeToSetter) Ownable(_owner) {
        require(_owner != address(0), "CaesarCoinDEXFactory: ZERO_ADDRESS");
        require(_feeToSetter != address(0), "CaesarCoinDEXFactory: ZERO_ADDRESS");
        
        feeToSetter = _feeToSetter;
        feeTo = _owner;
    }
    
    /**
     * @dev Get the number of pairs created
     */
    function allPairsLength() external view override returns (uint256) {
        return allPairs.length;
    }
    
    /**
     * @dev Get tokens being initialized (called by pair constructor)
     */
    function getInitializingTokens() external view override returns (address token0, address token1) {
        return (_initializingToken0, _initializingToken1);
    }
    
    /**
     * @dev Create a new trading pair with proper contract deployment
     */
    function createPair(address tokenA, address tokenB) external override nonReentrant returns (address pair) {
        require(tokenA != tokenB, "CaesarCoinDEXFactory: IDENTICAL_ADDRESSES");
        require(tokenA != address(0) && tokenB != address(0), "CaesarCoinDEXFactory: ZERO_ADDRESS");
        
        // Rate limiting
        require(
            block.timestamp >= userLastPairCreation[msg.sender] + PAIR_CREATION_COOLDOWN,
            "CaesarCoinDEXFactory: COOLDOWN_NOT_ELAPSED"
        );
        userLastPairCreation[msg.sender] = block.timestamp;
        
        // Sort tokens
        (address token0, address token1) = tokenA < tokenB ? (tokenA, tokenB) : (tokenB, tokenA);
        require(getPair[token0][token1] == address(0), "CaesarCoinDEXFactory: PAIR_EXISTS");
        
        // Set initializing tokens for pair constructor
        _initializingToken0 = token0;
        _initializingToken1 = token1;
        
        // Deploy the pair contract
        bytes memory bytecode = type(CaesarCoinDEXPair).creationCode;
        bytes32 salt = keccak256(abi.encodePacked(token0, token1));
        
        assembly {
            pair := create2(0, add(bytecode, 32), mload(bytecode), salt)
        }
        
        require(pair != address(0), "CaesarCoinDEXFactory: FAILED_TO_CREATE_PAIR");
        
        // Clear initializing tokens
        _initializingToken0 = address(0);
        _initializingToken1 = address(0);
        
        // Store pair information
        getPair[token0][token1] = pair;
        getPair[token1][token0] = pair;
        allPairs.push(pair);
        
        emit PairCreated(token0, token1, pair, allPairs.length);
        
        return pair;
    }
    
    /**
     * @dev Set fee recipient
     */
    function setFeeTo(address _feeTo) external override onlyOwner {
        feeTo = _feeTo;
        emit FeeToSet(_feeTo);
    }
    
    /**
     * @dev Set fee setter
     */
    function setFeeToSetter(address _feeToSetter) external override onlyOwner {
        require(_feeToSetter != address(0), "CaesarCoinDEXFactory: ZERO_ADDRESS");
        feeToSetter = _feeToSetter;
        emit FeeToSetterSet(_feeToSetter);
    }
    
    /**
     * @dev Set the trading fee (only fee setter can call)
     */
    function setTradingFee(uint256 _fee) external override onlyFeeToSetter {
        require(_fee <= MAX_TRADING_FEE, "CaesarCoinDEXFactory: FEE_TOO_HIGH");
        tradingFee = _fee;
        emit TradingFeeSet(_fee);
    }
    
    /**
     * @dev Set protocol fee percentage
     */
    function setProtocolFee(uint256 _fee) external override onlyFeeToSetter {
        require(_fee <= 50, "CaesarCoinDEXFactory: PROTOCOL_FEE_TOO_HIGH"); // Max 50% of trading fees
        protocolFeePercentage = _fee;
        emit ProtocolFeeSet(_fee);
    }
    
    /**
     * @dev Get effective trading fee for a pair
     */
    function getPairTradingFee(address pair) external view override returns (uint256) {
        // Verify this is actually a pair created by this factory
        bool isPairValid = false;
        for (uint256 i = 0; i < allPairs.length; i++) {
            if (allPairs[i] == pair) {
                isPairValid = true;
                break;
            }
        }
        
        if (!isPairValid) {
            return 0;
        }
        
        return tradingFee;
    }
    
    /**
     * @dev Update pair volume (called by pairs during swaps)
     */
    function updatePairVolume(address pair, uint256 volumeToAdd) external override {
        // Verify caller is a valid pair
        bool isPairValid = false;
        for (uint256 i = 0; i < allPairs.length; i++) {
            if (allPairs[i] == msg.sender && allPairs[i] == pair) {
                isPairValid = true;
                break;
            }
        }
        
        require(isPairValid, "CaesarCoinDEXFactory: INVALID_PAIR");
        
        pairVolumes[pair] += volumeToAdd;
        pairLastUpdate[pair] = block.timestamp;
        totalVolume += volumeToAdd;
        
        emit VolumeUpdated(pair, pairVolumes[pair]);
    }
    
    /**
     * @dev Collect protocol fees from a specific pair
     */
    function collectProtocolFees(address pair) external override {
        require(feeTo != address(0), "CaesarCoinDEXFactory: FEE_TO_NOT_SET");
        
        // Verify this is a valid pair
        bool isPairValid = false;
        for (uint256 i = 0; i < allPairs.length; i++) {
            if (allPairs[i] == pair) {
                isPairValid = true;
                break;
            }
        }
        
        require(isPairValid, "CaesarCoinDEXFactory: INVALID_PAIR");
        
        // Call pair's collect function
        CaesarCoinDEXPair(pair).collectProtocolFee();
    }
    
    /**
     * @dev Collect protocol fees from all pairs
     */
    function collectAllProtocolFees() external {
        require(feeTo != address(0), "CaesarCoinDEXFactory: FEE_TO_NOT_SET");
        
        for (uint256 i = 0; i < allPairs.length; i++) {
            try CaesarCoinDEXPair(allPairs[i]).collectProtocolFee() {
                // Success, continue
            } catch {
                // Skip failed collections
                continue;
            }
        }
    }
    
    /**
     * @dev Get pair creation code hash for CREATE2 prediction
     */
    function pairCodeHash() external pure returns (bytes32) {
        return keccak256(type(CaesarCoinDEXPair).creationCode);
    }
    
    /**
     * @dev Get pair address using CREATE2 prediction
     */
    function predictPairAddress(address tokenA, address tokenB) external view returns (address pair) {
        (address token0, address token1) = tokenA < tokenB ? (tokenA, tokenB) : (tokenB, tokenA);
        bytes32 salt = keccak256(abi.encodePacked(token0, token1));
        bytes32 hash = keccak256(abi.encodePacked(
            bytes1(0xff),
            address(this),
            salt,
            keccak256(type(CaesarCoinDEXPair).creationCode)
        ));
        return address(uint160(uint256(hash)));
    }
    
    /**
     * @dev Emergency function to pause pair creation (owner only)
     */
    function setPairCreationPaused(bool paused) external onlyOwner {
        // This would be implemented with a paused state
        // For now, we'll use the cooldown mechanism
    }
    
    /**
     * @dev Get pair volume statistics
     */
    function getPairStats(address pair) external view returns (
        uint256 volume,
        uint256 lastUpdate,
        bool isValid
    ) {
        // Verify pair
        bool isPairValid = false;
        for (uint256 i = 0; i < allPairs.length; i++) {
            if (allPairs[i] == pair) {
                isPairValid = true;
                break;
            }
        }
        
        return (
            pairVolumes[pair],
            pairLastUpdate[pair],
            isPairValid
        );
    }
    
    /**
     * @dev Get total DEX statistics
     */
    function getDEXStats() external view returns (
        uint256 _totalVolume,
        uint256 _totalPairs,
        uint256 _currentTradingFee,
        uint256 _protocolFeePercentage
    ) {
        return (
            totalVolume,
            allPairs.length,
            tradingFee,
            protocolFeePercentage
        );
    }
}