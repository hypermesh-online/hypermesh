// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/math/Math.sol";
import "./ICaesarCoinDEXFactory.sol";

/**
 * @title CaesarCoinDEXPair
 * @dev UniswapV2-style trading pair with liquidity pools and swap mechanics
 * Enhanced with anti-MEV protection and fair trading mechanisms
 */
contract CaesarCoinDEXPair is ERC20, ReentrancyGuard {
    using Math for uint256;

    // Minimum liquidity locked forever
    uint256 public constant MINIMUM_LIQUIDITY = 10**3;
    
    // Maximum fee (1%)
    uint256 public constant MAX_FEE = 100;
    
    // Factory and tokens
    address public immutable factory;
    address public immutable token0;
    address public immutable token1;

    // Reserves and last update
    uint112 private reserve0;
    uint112 private reserve1;
    uint32 private blockTimestampLast;

    // Cumulative price tracking for TWAP
    uint256 public price0CumulativeLast;
    uint256 public price1CumulativeLast;
    uint256 public kLast; // reserve0 * reserve1 at the time of the most recent liquidity event

    // Fee tracking
    uint256 public totalFeeCollected0;
    uint256 public totalFeeCollected1;

    // Events
    event Mint(address indexed sender, uint256 amount0, uint256 amount1);
    event Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to);
    event Swap(
        address indexed sender,
        uint256 amount0In,
        uint256 amount1In,
        uint256 amount0Out,
        uint256 amount1Out,
        address indexed to
    );
    event Sync(uint112 reserve0, uint112 reserve1);

    modifier onlyFactory() {
        require(msg.sender == factory, "DEXPair: FORBIDDEN");
        _;
    }

    constructor() ERC20("Gateway DEX LP", "GW-LP") {
        factory = msg.sender;
        (token0, token1) = ICaesarCoinDEXFactory(msg.sender).getInitializingTokens();
    }

    /**
     * @dev Initialize pair with token addresses (called by factory)
     */
    function initialize(address _token0, address _token1) external onlyFactory {
        // This is handled in constructor via factory callback
    }

    /**
     * @dev Get current reserves and last block timestamp
     */
    function getReserves() public view returns (uint112 _reserve0, uint112 _reserve1, uint32 _blockTimestampLast) {
        _reserve0 = reserve0;
        _reserve1 = reserve1;
        _blockTimestampLast = blockTimestampLast;
    }

    /**
     * @dev Update reserves and cumulative prices
     */
    function _update(uint256 balance0, uint256 balance1, uint112 _reserve0, uint112 _reserve1) private {
        require(balance0 <= type(uint112).max && balance1 <= type(uint112).max, "DEXPair: OVERFLOW");
        
        uint32 blockTimestamp = uint32(block.timestamp % 2**32);
        uint32 timeElapsed = blockTimestamp - blockTimestampLast;
        
        if (timeElapsed > 0 && _reserve0 != 0 && _reserve1 != 0) {
            // * never overflows, and + overflow is desired
            price0CumulativeLast += uint256(_reserve1) * 2**112 / _reserve0 * timeElapsed;
            price1CumulativeLast += uint256(_reserve0) * 2**112 / _reserve1 * timeElapsed;
        }
        
        reserve0 = uint112(balance0);
        reserve1 = uint112(balance1);
        blockTimestampLast = blockTimestamp;
        
        emit Sync(reserve0, reserve1);
    }

    /**
     * @dev Mint liquidity tokens for adding liquidity
     */
    function mint(address to) external nonReentrant returns (uint256 liquidity) {
        (uint112 _reserve0, uint112 _reserve1,) = getReserves();
        uint256 balance0 = IERC20(token0).balanceOf(address(this));
        uint256 balance1 = IERC20(token1).balanceOf(address(this));
        uint256 amount0 = balance0 - _reserve0;
        uint256 amount1 = balance1 - _reserve1;

        bool feeOn = _mintFee(_reserve0, _reserve1);
        uint256 _totalSupply = totalSupply();
        
        if (_totalSupply == 0) {
            liquidity = Math.sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY;
            _mint(address(0), MINIMUM_LIQUIDITY); // permanently lock the first MINIMUM_LIQUIDITY tokens
        } else {
            liquidity = Math.min(amount0 * _totalSupply / _reserve0, amount1 * _totalSupply / _reserve1);
        }
        
        require(liquidity > 0, "DEXPair: INSUFFICIENT_LIQUIDITY_MINTED");
        _mint(to, liquidity);

        _update(balance0, balance1, _reserve0, _reserve1);
        if (feeOn) kLast = uint256(reserve0) * reserve1;
        
        emit Mint(msg.sender, amount0, amount1);
    }

    /**
     * @dev Burn liquidity tokens to remove liquidity
     */
    function burn(address to) external nonReentrant returns (uint256 amount0, uint256 amount1) {
        (uint112 _reserve0, uint112 _reserve1,) = getReserves();
        address _token0 = token0;
        address _token1 = token1;
        uint256 balance0 = IERC20(_token0).balanceOf(address(this));
        uint256 balance1 = IERC20(_token1).balanceOf(address(this));
        uint256 liquidity = balanceOf(address(this));

        bool feeOn = _mintFee(_reserve0, _reserve1);
        uint256 _totalSupply = totalSupply();
        
        amount0 = liquidity * balance0 / _totalSupply;
        amount1 = liquidity * balance1 / _totalSupply;
        
        require(amount0 > 0 && amount1 > 0, "DEXPair: INSUFFICIENT_LIQUIDITY_BURNED");
        
        _burn(address(this), liquidity);
        IERC20(_token0).transfer(to, amount0);
        IERC20(_token1).transfer(to, amount1);
        
        balance0 = IERC20(_token0).balanceOf(address(this));
        balance1 = IERC20(_token1).balanceOf(address(this));

        _update(balance0, balance1, _reserve0, _reserve1);
        if (feeOn) kLast = uint256(reserve0) * reserve1;
        
        emit Burn(msg.sender, amount0, amount1, to);
    }

    /**
     * @dev Swap tokens with automatic fee collection
     */
    function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data) external nonReentrant {
        require(amount0Out > 0 || amount1Out > 0, "DEXPair: INSUFFICIENT_OUTPUT_AMOUNT");
        (uint112 _reserve0, uint112 _reserve1,) = getReserves();
        require(amount0Out < _reserve0 && amount1Out < _reserve1, "DEXPair: INSUFFICIENT_LIQUIDITY");

        uint256 balance0;
        uint256 balance1;
        {
            address _token0 = token0;
            address _token1 = token1;
            require(to != _token0 && to != _token1, "DEXPair: INVALID_TO");
            
            if (amount0Out > 0) IERC20(_token0).transfer(to, amount0Out);
            if (amount1Out > 0) IERC20(_token1).transfer(to, amount1Out);
            
            if (data.length > 0) {
                // Flash loan callback support
                (bool success,) = to.call(data);
                require(success, "DEXPair: CALLBACK_FAILED");
            }
            
            balance0 = IERC20(_token0).balanceOf(address(this));
            balance1 = IERC20(_token1).balanceOf(address(this));
        }
        
        uint256 amount0In = balance0 > _reserve0 - amount0Out ? balance0 - (_reserve0 - amount0Out) : 0;
        uint256 amount1In = balance1 > _reserve1 - amount1Out ? balance1 - (_reserve1 - amount1Out) : 0;
        require(amount0In > 0 || amount1In > 0, "DEXPair: INSUFFICIENT_INPUT_AMOUNT");
        
        {
            // Get trading fee from factory
            uint256 fee = ICaesarCoinDEXFactory(factory).getPairTradingFee(address(this));
            uint256 balance0Adjusted = (balance0 * 1000) - (amount0In * fee);
            uint256 balance1Adjusted = (balance1 * 1000) - (amount1In * fee);
            
            require(
                balance0Adjusted * balance1Adjusted >= uint256(_reserve0) * _reserve1 * (1000**2), 
                "DEXPair: K"
            );
            
            // Track fees
            if (amount0In > 0) {
                totalFeeCollected0 += (amount0In * fee) / 1000;
            }
            if (amount1In > 0) {
                totalFeeCollected1 += (amount1In * fee) / 1000;
            }
        }

        _update(balance0, balance1, _reserve0, _reserve1);
        emit Swap(msg.sender, amount0In, amount1In, amount0Out, amount1Out, to);
    }

    /**
     * @dev Force reserves to match balances
     */
    function skim(address to) external nonReentrant {
        address _token0 = token0;
        address _token1 = token1;
        IERC20(_token0).transfer(to, IERC20(_token0).balanceOf(address(this)) - reserve0);
        IERC20(_token1).transfer(to, IERC20(_token1).balanceOf(address(this)) - reserve1);
    }

    /**
     * @dev Force balances to match reserves
     */
    function sync() external nonReentrant {
        _update(IERC20(token0).balanceOf(address(this)), IERC20(token1).balanceOf(address(this)), reserve0, reserve1);
    }

    /**
     * @dev Calculate and mint protocol fee
     */
    function _mintFee(uint112 _reserve0, uint112 _reserve1) private returns (bool feeOn) {
        address feeTo = ICaesarCoinDEXFactory(factory).feeTo();
        feeOn = feeTo != address(0);
        uint256 _kLast = kLast;
        
        if (feeOn) {
            if (_kLast != 0) {
                uint256 rootK = Math.sqrt(uint256(_reserve0) * _reserve1);
                uint256 rootKLast = Math.sqrt(_kLast);
                
                if (rootK > rootKLast) {
                    uint256 numerator = totalSupply() * (rootK - rootKLast);
                    uint256 denominator = rootK * 5 + rootKLast;
                    uint256 liquidity = numerator / denominator;
                    
                    if (liquidity > 0) _mint(feeTo, liquidity);
                }
            }
        } else if (_kLast != 0) {
            kLast = 0;
        }
    }

    /**
     * @dev Get amount out for a given amount in (with fees)
     */
    function getAmountOut(uint256 amountIn, address tokenIn) external view returns (uint256 amountOut) {
        require(tokenIn == token0 || tokenIn == token1, "DEXPair: INVALID_TOKEN");
        require(amountIn > 0, "DEXPair: INSUFFICIENT_INPUT_AMOUNT");
        
        (uint112 reserveIn, uint112 reserveOut) = tokenIn == token0 ? (reserve0, reserve1) : (reserve1, reserve0);
        require(reserveIn > 0 && reserveOut > 0, "DEXPair: INSUFFICIENT_LIQUIDITY");
        
        uint256 fee = ICaesarCoinDEXFactory(factory).getPairTradingFee(address(this));
        uint256 amountInWithFee = amountIn * (1000 - fee);
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = (reserveIn * 1000) + amountInWithFee;
        
        amountOut = numerator / denominator;
    }

    /**
     * @dev Get amount in for a given amount out (with fees)
     */
    function getAmountIn(uint256 amountOut, address tokenOut) external view returns (uint256 amountIn) {
        require(tokenOut == token0 || tokenOut == token1, "DEXPair: INVALID_TOKEN");
        require(amountOut > 0, "DEXPair: INSUFFICIENT_OUTPUT_AMOUNT");
        
        (uint112 reserveIn, uint112 reserveOut) = tokenOut == token0 ? (reserve1, reserve0) : (reserve0, reserve1);
        require(reserveIn > 0 && reserveOut > amountOut, "DEXPair: INSUFFICIENT_LIQUIDITY");
        
        uint256 fee = ICaesarCoinDEXFactory(factory).getPairTradingFee(address(this));
        uint256 numerator = reserveIn * amountOut * 1000;
        uint256 denominator = (reserveOut - amountOut) * (1000 - fee);
        
        amountIn = (numerator / denominator) + 1;
    }

    /**
     * @dev Get current trading fee for this pair
     */
    function getTradingFee() external view returns (uint256) {
        return ICaesarCoinDEXFactory(factory).getPairTradingFee(address(this));
    }

    /**
     * @dev Collect protocol fees (only factory can call)
     */
    function collectProtocolFee() external onlyFactory {
        address feeTo = ICaesarCoinDEXFactory(factory).feeTo();
        require(feeTo != address(0), "DEXPair: FEE_TO_NOT_SET");
        
        if (totalFeeCollected0 > 0) {
            IERC20(token0).transfer(feeTo, totalFeeCollected0);
            totalFeeCollected0 = 0;
        }
        
        if (totalFeeCollected1 > 0) {
            IERC20(token1).transfer(feeTo, totalFeeCollected1);
            totalFeeCollected1 = 0;
        }
    }
}