// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title MathUtils
 * @dev Mathematical utilities for Caesar Token calculations
 */
library MathUtils {
    uint256 public constant BASIS_POINTS = 10000;
    uint256 public constant PRECISION = 1e18;
    uint256 public constant SECONDS_PER_DAY = 86400;
    uint256 public constant SECONDS_PER_YEAR = 31536000;
    
    /**
     * @dev Calculate exponential decay based on time elapsed
     * @param principal The initial amount
     * @param rate The decay rate (basis points per period)
     * @param timeElapsed The time elapsed in seconds
     * @param period The period for rate calculation (e.g., daily = 86400)
     * @return The decayed amount
     */
    function calculateExponentialDecay(
        uint256 principal,
        uint256 rate,
        uint256 timeElapsed,
        uint256 period
    ) internal pure returns (uint256) {
        if (timeElapsed == 0 || rate == 0) {
            return principal;
        }
        
        uint256 periods = (timeElapsed * PRECISION) / period;
        uint256 decayFactor = PRECISION - (rate * PRECISION) / BASIS_POINTS;
        
        // Use approximation for exponential: e^(-x) ≈ 1 - x + x²/2 - x³/6
        // For small x, this simplifies to compound decay
        uint256 compoundPeriods = periods / PRECISION;
        uint256 remainingFraction = periods % PRECISION;
        
        uint256 result = principal;
        
        // Apply full periods
        for (uint256 i = 0; i < compoundPeriods && i < 100; i++) {
            result = (result * decayFactor) / PRECISION;
        }
        
        // Apply fractional period
        if (remainingFraction > 0) {
            uint256 fractionalDecay = (rate * remainingFraction) / BASIS_POINTS;
            result = result - (result * fractionalDecay) / PRECISION;
        }
        
        return result;
    }
    
    /**
     * @dev Calculate linear decay
     * @param principal The initial amount
     * @param rate The decay rate per period
     * @param timeElapsed Time elapsed in seconds
     * @param period The period for rate calculation
     * @return The decayed amount
     */
    function calculateLinearDecay(
        uint256 principal,
        uint256 rate,
        uint256 timeElapsed,
        uint256 period
    ) internal pure returns (uint256) {
        if (timeElapsed == 0 || rate == 0) {
            return principal;
        }
        
        uint256 decayAmount = (principal * rate * timeElapsed) / (BASIS_POINTS * period);
        
        if (decayAmount >= principal) {
            return 0;
        }
        
        return principal - decayAmount;
    }
    
    /**
     * @dev Calculate stability index based on price deviation
     * @param currentPrice Current price in wei
     * @param targetPrice Target price in wei
     * @param activeParticipants Number of active participants
     * @param totalHolders Total number of holders
     * @return Stability index (0-1000 basis points)
     */
    function calculateStabilityIndex(
        uint256 currentPrice,
        uint256 targetPrice,
        uint256 activeParticipants,
        uint256 totalHolders
    ) internal pure returns (uint256) {
        if (totalHolders == 0) {
            return 0;
        }
        
        // Price stability component (0-500 basis points)
        uint256 priceDeviation = currentPrice > targetPrice
            ? currentPrice - targetPrice
            : targetPrice - currentPrice;
            
        uint256 priceStability = priceDeviation > targetPrice
            ? 0
            : 500 - ((priceDeviation * 500) / targetPrice);
        
        // Participation component (0-500 basis points)
        uint256 participationRatio = (activeParticipants * PRECISION) / totalHolders;
        uint256 participationStability = (participationRatio * 500) / PRECISION;
        
        return priceStability + participationStability;
    }
    
    /**
     * @dev Calculate transaction fee based on spread formula
     * @param baseRate Base spread rate in basis points
     * @param liquidityRatio Current liquidity ratio (0-1000 basis points)
     * @param participationIndicator Binary participation indicator (0 or 1)
     * @return Transaction fee in basis points
     */
    function calculateTransactionFee(
        uint256 baseRate,
        uint256 liquidityRatio,
        uint256 participationIndicator
    ) internal pure returns (uint256) {
        if (participationIndicator == 0) {
            return 0; // Invalid participants pay no fee (transaction rejected)
        }
        
        if (liquidityRatio == 0) {
            return baseRate * 10; // Emergency high fee
        }
        
        return (baseRate * BASIS_POINTS) / liquidityRatio;
    }
    
    /**
     * @dev Calculate rebase ratio based on price deviation
     * @param currentPrice Current price in wei
     * @param targetPrice Target price in wei
     * @param maxRebasePercent Maximum rebase percentage (basis points)
     * @return Rebase ratio (1000 = 1.0x, 2000 = 2.0x)
     */
    function calculateRebaseRatio(
        uint256 currentPrice,
        uint256 targetPrice,
        uint256 maxRebasePercent
    ) internal pure returns (uint256) {
        if (currentPrice == targetPrice) {
            return BASIS_POINTS; // 1.0x ratio
        }
        
        uint256 priceRatio = (currentPrice * BASIS_POINTS) / targetPrice;
        
        if (priceRatio > BASIS_POINTS) {
            // Price too high, need to increase supply (ratio > 1.0)
            uint256 deviation = priceRatio - BASIS_POINTS;
            uint256 adjustment = (deviation * maxRebasePercent) / BASIS_POINTS;
            return BASIS_POINTS + adjustment;
        } else {
            // Price too low, need to decrease supply (ratio < 1.0)
            uint256 deviation = BASIS_POINTS - priceRatio;
            uint256 adjustment = (deviation * maxRebasePercent) / BASIS_POINTS;
            return BASIS_POINTS - adjustment;
        }
    }
    
    /**
     * @dev Safe multiplication with overflow protection
     * @param a First number
     * @param b Second number
     * @return Product, or type(uint256).max if overflow
     */
    function safeMul(uint256 a, uint256 b) internal pure returns (uint256) {
        if (a == 0) {
            return 0;
        }
        
        uint256 c = a * b;
        if (c / a != b) {
            return type(uint256).max; // Overflow occurred
        }
        
        return c;
    }
    
    /**
     * @dev Calculate square root using Babylonian method
     * @param y The number to find square root of
     * @return z The square root
     */
    function sqrt(uint256 y) internal pure returns (uint256 z) {
        if (y > 3) {
            z = y;
            uint256 x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }
}