// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title MockERC20
 * @dev Mock ERC20 token for testing
 */
contract MockERC20 is ERC20, Ownable {
    uint8 private _decimals;
    
    constructor(
        string memory name,
        string memory symbol,
        uint8 decimals_
    ) ERC20(name, symbol) Ownable(msg.sender) {
        _decimals = decimals_;
    }
    
    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }
    
    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }
    
    function burn(address from, uint256 amount) external onlyOwner {
        _burn(from, amount);
    }
}

/**
 * @title MockPriceOracle
 * @dev Mock price oracle for testing
 */
contract MockPriceOracle is Ownable {
    uint256 private _price;
    uint256 private _lastUpdate;
    
    event PriceUpdated(uint256 newPrice, uint256 timestamp);
    
    constructor(uint256 initialPrice) Ownable(msg.sender) {
        _price = initialPrice;
        _lastUpdate = block.timestamp;
    }
    
    function getPrice() external view returns (uint256 price, uint256 timestamp) {
        return (_price, _lastUpdate);
    }
    
    function setPrice(uint256 newPrice) external onlyOwner {
        _price = newPrice;
        _lastUpdate = block.timestamp;
        emit PriceUpdated(newPrice, block.timestamp);
    }
    
    function getLatestPrice() external view returns (uint256) {
        return _price;
    }
}

/**
 * @title MockLayerZeroEndpoint
 * @dev Mock LayerZero endpoint for testing cross-chain functionality
 */
contract MockLayerZeroEndpoint {
    struct StoredPayload {
        uint64 payloadLength;
        address dstAddress;
        bytes32 payloadHash;
    }
    
    mapping(uint16 => mapping(bytes => mapping(uint64 => StoredPayload))) public storedPayloads;
    mapping(uint16 => mapping(bytes => uint64)) public inboundNonce;
    mapping(uint16 => mapping(bytes => uint64)) public outboundNonce;
    mapping(address => mapping(uint16 => bytes)) public trustedRemoteLookup;
    
    event PayloadCleared(uint16 srcChainId, bytes srcAddress, uint64 nonce, address dstAddress);
    event PayloadStored(uint16 srcChainId, bytes srcAddress, address dstAddress, uint64 nonce, bytes payload, bytes reason);
    
    function send(
        uint16 _dstChainId,
        bytes calldata _destination,
        bytes calldata _payload,
        address payable /*_refundAddress*/,
        address /*_zroPaymentAddress*/,
        bytes calldata /*_adapterParams*/
    ) external payable {
        outboundNonce[_dstChainId][_destination]++;
        // For testing, we can emit events or store data as needed
    }
    
    function receivePayload(
        uint16 _srcChainId,
        bytes calldata _srcAddress,
        address _dstAddress,
        uint64 _nonce,
        uint _gasLimit,
        bytes calldata _payload
    ) external {
        // Simulate receiving a payload
        inboundNonce[_srcChainId][_srcAddress] = _nonce;
        
        // For testing, directly call the receive function
        (bool success,) = _dstAddress.call{gas: _gasLimit}(
            abi.encodeWithSignature("lzReceive(uint16,bytes,uint64,bytes)", _srcChainId, _srcAddress, _nonce, _payload)
        );
        
        if (!success) {
            // Store failed payload
            storedPayloads[_srcChainId][_srcAddress][_nonce] = StoredPayload(
                uint64(_payload.length),
                _dstAddress,
                keccak256(_payload)
            );
            emit PayloadStored(_srcChainId, _srcAddress, _dstAddress, _nonce, _payload, "execution failed");
        }
    }
    
    function retryPayload(
        uint16 _srcChainId,
        bytes calldata _srcAddress,
        uint64 _nonce,
        bytes calldata _payload
    ) external {
        StoredPayload memory payload = storedPayloads[_srcChainId][_srcAddress][_nonce];
        require(payload.payloadHash == keccak256(_payload), "Invalid payload");
        
        delete storedPayloads[_srcChainId][_srcAddress][_nonce];
        
        (bool success,) = payload.dstAddress.call(
            abi.encodeWithSignature("lzReceive(uint16,bytes,uint64,bytes)", _srcChainId, _srcAddress, _nonce, _payload)
        );
        
        require(success, "Retry failed");
        emit PayloadCleared(_srcChainId, _srcAddress, _nonce, payload.dstAddress);
    }
    
    function setTrustedRemote(uint16 _srcChainId, bytes calldata _path) external {
        trustedRemoteLookup[msg.sender][_srcChainId] = _path;
    }
    
    function isTrustedRemote(uint16 _srcChainId, bytes calldata _srcAddress) external view returns (bool) {
        bytes memory trustedSource = trustedRemoteLookup[msg.sender][_srcChainId];
        return keccak256(trustedSource) == keccak256(_srcAddress);
    }
    
    function getInboundNonce(uint16 _srcChainId, bytes calldata _srcAddress) external view returns (uint64) {
        return inboundNonce[_srcChainId][_srcAddress];
    }
    
    function getOutboundNonce(uint16 _dstChainId, bytes calldata _dstAddress) external view returns (uint64) {
        return outboundNonce[_dstChainId][_dstAddress];
    }
    
    function estimateFees(
        uint16 /*_dstChainId*/,
        address /*_userApplication*/,
        bytes calldata /*_payload*/,
        bool /*_payInZRO*/,
        bytes calldata /*_adapterParam*/
    ) external pure returns (uint nativeFee, uint zroFee) {
        // Mock fee estimation
        return (0.001 ether, 0);
    }
}

/**
 * @title MockAMM
 * @dev Mock AMM for testing price impact and trading
 */
contract MockAMM {
    mapping(address => uint256) public reserves;
    uint256 public constant FEE_RATE = 30; // 0.3%
    uint256 public constant FEE_DENOMINATOR = 10000;
    
    address public token0;
    address public token1;
    
    constructor(address _token0, address _token1) {
        token0 = _token0;
        token1 = _token1;
    }
    
    function addLiquidity(
        uint256 amount0,
        uint256 amount1
    ) external {
        IERC20(token0).transferFrom(msg.sender, address(this), amount0);
        IERC20(token1).transferFrom(msg.sender, address(this), amount1);
        
        reserves[token0] += amount0;
        reserves[token1] += amount1;
    }
    
    function swap(
        address tokenIn,
        uint256 amountIn,
        uint256 minAmountOut
    ) external returns (uint256 amountOut) {
        require(tokenIn == token0 || tokenIn == token1, "Invalid token");
        
        address tokenOut = tokenIn == token0 ? token1 : token0;
        
        // Simple constant product formula with fee
        uint256 amountInWithFee = amountIn * (FEE_DENOMINATOR - FEE_RATE) / FEE_DENOMINATOR;
        amountOut = (reserves[tokenOut] * amountInWithFee) / (reserves[tokenIn] + amountInWithFee);
        
        require(amountOut >= minAmountOut, "Insufficient output");
        
        IERC20(tokenIn).transferFrom(msg.sender, address(this), amountIn);
        IERC20(tokenOut).transfer(msg.sender, amountOut);
        
        reserves[tokenIn] += amountIn;
        reserves[tokenOut] -= amountOut;
    }
    
    function getAmountOut(
        address tokenIn,
        uint256 amountIn
    ) external view returns (uint256 amountOut) {
        require(tokenIn == token0 || tokenIn == token1, "Invalid token");
        
        address tokenOut = tokenIn == token0 ? token1 : token0;
        
        uint256 amountInWithFee = amountIn * (FEE_DENOMINATOR - FEE_RATE) / FEE_DENOMINATOR;
        amountOut = (reserves[tokenOut] * amountInWithFee) / (reserves[tokenIn] + amountInWithFee);
    }
    
    function getPriceImpact(
        address tokenIn,
        uint256 amountIn
    ) external view returns (uint256 impact) {
        require(tokenIn == token0 || tokenIn == token1, "Invalid token");
        
        address tokenOut = tokenIn == token0 ? token1 : token0;
        
        uint256 priceBefore = (reserves[tokenOut] * 1e18) / reserves[tokenIn];
        
        uint256 newReserveIn = reserves[tokenIn] + amountIn;
        uint256 amountOut = getAmountOut(tokenIn, amountIn);
        uint256 newReserveOut = reserves[tokenOut] - amountOut;
        
        uint256 priceAfter = (newReserveOut * 1e18) / newReserveIn;
        
        if (priceAfter < priceBefore) {
            impact = ((priceBefore - priceAfter) * 10000) / priceBefore;
        } else {
            impact = ((priceAfter - priceBefore) * 10000) / priceBefore;
        }
    }
}

/**
 * @title MockStripeOracle
 * @dev Mock Stripe oracle for testing fiat integration
 */
contract MockStripeOracle is Ownable {
    struct FiatBalance {
        uint256 balance;
        uint256 lastUpdate;
        bool isValid;
    }
    
    mapping(string => FiatBalance) public stripeBalances;
    mapping(string => bool) public verifiedAccounts;
    
    event BalanceUpdated(string indexed accountId, uint256 balance);
    event AccountVerified(string indexed accountId, bool isVerified);
    
    constructor() Ownable(msg.sender) {}
    
    function updateBalance(
        string calldata accountId,
        uint256 balance
    ) external onlyOwner {
        stripeBalances[accountId] = FiatBalance({
            balance: balance,
            lastUpdate: block.timestamp,
            isValid: true
        });
        
        emit BalanceUpdated(accountId, balance);
    }
    
    function verifyAccount(
        string calldata accountId,
        bool isVerified
    ) external onlyOwner {
        verifiedAccounts[accountId] = isVerified;
        emit AccountVerified(accountId, isVerified);
    }
    
    function getBalance(string calldata accountId) external view returns (
        uint256 balance,
        uint256 lastUpdate,
        bool isValid
    ) {
        FiatBalance memory fiatBalance = stripeBalances[accountId];
        return (fiatBalance.balance, fiatBalance.lastUpdate, fiatBalance.isValid);
    }
    
    function getTotalBalance(
        string[] calldata accountIds
    ) external view returns (uint256 totalBalance) {
        for (uint256 i = 0; i < accountIds.length; i++) {
            totalBalance += stripeBalances[accountIds[i]].balance;
        }
    }
    
    function isAccountVerified(string calldata accountId) external view returns (bool) {
        return verifiedAccounts[accountId];
    }
}

/**
 * @title MockTimelock
 * @dev Mock timelock for testing governance delays
 */
contract MockTimelock {
    mapping(bytes32 => uint256) public timestamps;
    uint256 public delay = 2 days;
    
    event CallScheduled(bytes32 indexed id, address target, bytes data, uint256 executeTime);
    event CallExecuted(bytes32 indexed id, address target, bytes data);
    
    function schedule(
        address target,
        bytes calldata data,
        uint256 executeTime
    ) external returns (bytes32 id) {
        require(executeTime >= block.timestamp + delay, "Insufficient delay");
        
        id = keccak256(abi.encode(target, data, executeTime));
        timestamps[id] = executeTime;
        
        emit CallScheduled(id, target, data, executeTime);
    }
    
    function execute(
        address target,
        bytes calldata data,
        uint256 executeTime
    ) external returns (bytes memory result) {
        bytes32 id = keccak256(abi.encode(target, data, executeTime));
        require(timestamps[id] > 0, "Call not scheduled");
        require(block.timestamp >= timestamps[id], "Call not ready");
        
        delete timestamps[id];
        
        (bool success, bytes memory returnData) = target.call(data);
        require(success, "Call execution failed");
        
        emit CallExecuted(id, target, data);
        return returnData;
    }
    
    function setDelay(uint256 newDelay) external {
        delay = newDelay;
    }
}

/**
 * @title MockChainlinkOracle
 * @dev Mock Chainlink oracle for price feeds
 */
contract MockChainlinkOracle {
    int256 private _price;
    uint256 private _timestamp;
    uint80 private _roundId;
    
    constructor(int256 initialPrice) {
        _price = initialPrice;
        _timestamp = block.timestamp;
        _roundId = 1;
    }
    
    function latestRoundData() external view returns (
        uint80 roundId,
        int256 answer,
        uint256 startedAt,
        uint256 updatedAt,
        uint80 answeredInRound
    ) {
        return (_roundId, _price, _timestamp, _timestamp, _roundId);
    }
    
    function updatePrice(int256 newPrice) external {
        _price = newPrice;
        _timestamp = block.timestamp;
        _roundId++;
    }
    
    function decimals() external pure returns (uint8) {
        return 8; // Standard Chainlink decimals
    }
}