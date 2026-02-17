// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

/**
 * @title AdvancedMathUtils
 * @dev Advanced mathematical utilities for Caesar Token economic calculations
 * Implements high-precision mathematical functions for demurrage, stability, and risk calculations
 */
library AdvancedMathUtils {
    uint256 public constant PRECISION = 1e18;
    uint256 public constant BASIS_POINTS = 10000;
    uint256 public constant E_APPROXIMATION = 2718281828459045235; // e * 1e18
    uint256 public constant MAX_ITERATIONS = 50;
    uint256 public constant CONVERGENCE_THRESHOLD = 1e12;
    
    /**
     * @dev Calculate exponential decay with high precision
     * Formula: New_Balance = Original_Balance * e^(-rate * time_elapsed)
     * @param principal Initial amount
     * @param ratePerHour Decay rate per hour (basis points)
     * @param hoursElapsed Time elapsed in hours
     * @return Decayed amount
     */
    function calculateExponentialDecay(
        uint256 principal,
        uint256 ratePerHour,
        uint256 hoursElapsed
    ) internal pure returns (uint256) {
        if (principal == 0 || ratePerHour == 0 || hoursElapsed == 0) {
            return principal;
        }
        
        // Convert rate to decimal: rate / 10000 / 100 = rate / 1000000
        uint256 decayExponent = (ratePerHour * hoursElapsed * PRECISION) / 1000000;
        
        // Calculate e^(-decayExponent) using Taylor series approximation
        uint256 expNegativeRate = calculateExpNegative(decayExponent);
        
        return (principal * expNegativeRate) / PRECISION;
    }
    
    /**
     * @dev Calculate e^(-x) using Taylor series expansion
     * e^(-x) = 1 - x + x²/2! - x³/3! + x⁴/4! - ...
     * @param x Exponent (scaled by PRECISION)
     * @return e^(-x) scaled by PRECISION
     */
    function calculateExpNegative(uint256 x) internal pure returns (uint256) {
        if (x == 0) return PRECISION;
        if (x > 20 * PRECISION) return 0; // Underflow protection
        
        uint256 result = PRECISION; // Start with 1
        uint256 term = PRECISION;   // First term = 1
        uint256 factorial = 1;
        
        // Calculate terms of Taylor series
        for (uint256 i = 1; i <= MAX_ITERATIONS; i++) {
            factorial *= i;
            term = (term * x) / PRECISION;
            
            uint256 termValue = term / factorial;
            if (termValue < CONVERGENCE_THRESHOLD) break;
            
            if (i % 2 == 1) {
                // Odd terms are negative
                result = result > termValue ? result - termValue : 0;
            } else {
                // Even terms are positive
                result += termValue;
            }
        }
        
        return result;
    }
    
    /**
     * @dev Calculate compound interest with continuous compounding
     * Formula: A = P * e^(rt)
     * @param principal Initial principal
     * @param rate Interest rate per period
     * @param time Time periods
     * @return Final amount
     */
    function calculateCompoundInterest(
        uint256 principal,
        uint256 rate,
        uint256 time
    ) internal pure returns (uint256) {
        uint256 exponent = (rate * time) / BASIS_POINTS;
        uint256 expValue = calculateExp(exponent);
        return (principal * expValue) / PRECISION;
    }
    
    /**
     * @dev Calculate e^x using Taylor series expansion
     * e^x = 1 + x + x²/2! + x³/3! + x⁴/4! + ...
     * @param x Exponent (scaled by PRECISION)
     * @return e^x scaled by PRECISION
     */
    function calculateExp(uint256 x) internal pure returns (uint256) {
        if (x == 0) return PRECISION;
        if (x > 20 * PRECISION) return type(uint256).max; // Overflow protection
        
        uint256 result = PRECISION; // Start with 1
        uint256 term = PRECISION;   // First term = 1
        uint256 factorial = 1;
        
        for (uint256 i = 1; i <= MAX_ITERATIONS; i++) {
            factorial *= i;
            term = (term * x) / PRECISION;
            
            uint256 termValue = term / factorial;
            if (termValue < CONVERGENCE_THRESHOLD) break;
            
            result += termValue;
        }
        
        return result;
    }
    
    /**
     * @dev Calculate natural logarithm ln(x) using Newton-Raphson method
     * @param x Input value (scaled by PRECISION)
     * @return Natural logarithm scaled by PRECISION
     */
    function calculateLn(uint256 x) internal pure returns (uint256) {
        require(x > 0, "Input must be positive");
        
        if (x == PRECISION) return 0; // ln(1) = 0
        
        // Initial guess
        uint256 result = 0;
        
        // Adjust x to be close to 1 for better convergence
        uint256 adjustedX = x;
        int256 adjustment = 0;
        
        while (adjustedX >= 2 * PRECISION) {
            adjustedX = adjustedX / 2;
            adjustment++;
        }
        
        while (adjustedX < PRECISION / 2) {
            adjustedX = adjustedX * 2;
            adjustment--;
        }
        
        // Newton-Raphson: x_n+1 = x_n + 2 * (y - e^x_n) / (y + e^x_n)
        for (uint256 i = 0; i < MAX_ITERATIONS; i++) {
            uint256 expResult = calculateExp(result);
            if (expResult == adjustedX) break;
            
            uint256 numerator = adjustedX > expResult ? 
                2 * PRECISION * (adjustedX - expResult) :
                2 * PRECISION * (expResult - adjustedX);
            uint256 denominator = adjustedX + expResult;
            
            uint256 delta = (numerator * PRECISION) / (denominator * PRECISION / PRECISION);
            
            if (adjustedX > expResult) {
                result += delta;
            } else {
                result = result > delta ? result - delta : 0;
            }
            
            if (delta < CONVERGENCE_THRESHOLD) break;
        }
        
        // Adjust for the scaling we did earlier
        if (adjustment > 0) {
            result += uint256(adjustment) * 693147180559945309; // ln(2) * 1e18
        } else if (adjustment < 0) {
            uint256 subtraction = uint256(-adjustment) * 693147180559945309;
            result = result > subtraction ? result - subtraction : 0;
        }
        
        return result;
    }
    
    /**
     * @dev Calculate stability index using advanced formula
     * SI = (U_ratio * N_participants) / (S_ratio * T_volume + ε)
     * @param utilizationRatio Network utilization ratio
     * @param participantCount Number of active participants
     * @param speculationRatio Speculation activity ratio
     * @param totalVolume Total transaction volume
     * @return Stability index (0-1000)
     */
    function calculateStabilityIndex(
        uint256 utilizationRatio,
        uint256 participantCount,
        uint256 speculationRatio,
        uint256 totalVolume
    ) internal pure returns (uint256) {
        if (participantCount == 0) return 0;
        
        // Prevent division by zero
        uint256 denominator = speculationRatio * totalVolume / PRECISION + PRECISION / 1000; // Add small epsilon
        
        uint256 numerator = utilizationRatio * participantCount;
        uint256 rawIndex = (numerator * PRECISION) / denominator;
        
        // Normalize to 0-1000 range
        return rawIndex > 1000 ? 1000 : rawIndex;
    }
    
    /**
     * @dev Calculate anti-speculation score with weighted factors
     * AS = (Frequency_penalty + Volume_penalty + Pattern_penalty) / 3
     * @param frequencyScore Transaction frequency score (0-1000)
     * @param volumeScore Volume concentration score (0-1000)  
     * @param patternScore Pattern recognition score (0-1000)
     * @return Combined anti-speculation score (0-1000)
     */
    function calculateAntiSpeculationScore(
        uint256 frequencyScore,
        uint256 volumeScore,
        uint256 patternScore
    ) internal pure returns (uint256) {
        // Weighted average with emphasis on pattern recognition
        uint256 weightedSum = frequencyScore * 25 + volumeScore * 30 + patternScore * 45;
        return weightedSum / 100;
    }
    
    /**
     * @dev Calculate reserve ratio with safety margins
     * RR = (USDC_reserves + Fiat_backing) / Total_GATE_supply
     * @param usdcReserves USDC reserve amount
     * @param fiatBacking Fiat backing amount
     * @param totalSupply Total CAESAR token supply
     * @return Reserve ratio (0-2000, where 1000 = 100%)
     */
    function calculateReserveRatio(
        uint256 usdcReserves,
        uint256 fiatBacking,
        uint256 totalSupply
    ) internal pure returns (uint256) {
        if (totalSupply == 0) return 2000; // Maximum ratio if no supply
        
        uint256 totalBacking = usdcReserves + fiatBacking;
        return (totalBacking * 1000) / totalSupply;
    }
    
    /**
     * @dev Calculate volatility using exponential moving average
     * @param previousVolatility Previous volatility measurement
     * @param currentPriceChange Current price change percentage
     * @param alpha Smoothing factor (0-1000, where 1000 = 1.0)
     * @return Updated volatility measure
     */
    function calculateVolatility(
        uint256 previousVolatility,
        uint256 currentPriceChange,
        uint256 alpha
    ) internal pure returns (uint256) {
        // EMA: volatility = α * |price_change| + (1-α) * previous_volatility
        uint256 alphaFactor = (alpha * currentPriceChange) / 1000;
        uint256 oneMinusAlpha = 1000 - alpha;
        uint256 previousFactor = (oneMinusAlpha * previousVolatility) / 1000;
        
        return alphaFactor + previousFactor;
    }
    
    /**
     * @dev Calculate risk-adjusted return for economic decisions
     * @param expectedReturn Expected return percentage
     * @param volatility Risk volatility measure
     * @param riskFreeRate Risk-free rate
     * @return Risk-adjusted return (Sharpe ratio * 1000)
     */
    function calculateRiskAdjustedReturn(
        uint256 expectedReturn,
        uint256 volatility,
        uint256 riskFreeRate
    ) internal pure returns (uint256) {
        if (volatility == 0) return expectedReturn * 1000;
        
        uint256 excessReturn = expectedReturn > riskFreeRate ? 
            expectedReturn - riskFreeRate : 0;
            
        return (excessReturn * 1000) / volatility;
    }
    
    /**
     * @dev Calculate optimal demurrage rate based on multiple factors
     * @param baseRate Base demurrage rate
     * @param priceDeviation Current price deviation from peg
     * @param velocityRatio Money velocity ratio
     * @param stabilityScore Current stability score
     * @return Optimal demurrage rate
     */
    function calculateOptimalDemurrageRate(
        uint256 baseRate,
        uint256 priceDeviation,
        uint256 velocityRatio,
        uint256 stabilityScore
    ) internal pure returns (uint256) {
        // Adjust rate based on price stability
        uint256 priceAdjustment = (priceDeviation * baseRate) / 1000;
        
        // Adjust for velocity (encourage circulation)
        uint256 velocityAdjustment = velocityRatio < 500 ? 
            (500 - velocityRatio) * baseRate / 1000 : 0;
        
        // Adjust for overall stability
        uint256 stabilityAdjustment = stabilityScore > 800 ?
            baseRate / 4 : // Reduce rate when stable
            baseRate / 2;  // Normal adjustment
        
        uint256 adjustedRate = baseRate + priceAdjustment + velocityAdjustment;
        return adjustedRate > stabilityAdjustment ? 
            adjustedRate - stabilityAdjustment : baseRate / 4;
    }
    
    /**
     * @dev Calculate safe square root using Babylonian method with higher precision
     * @param y Number to find square root of
     * @return Square root scaled by PRECISION
     */
    function sqrt(uint256 y) internal pure returns (uint256) {
        if (y == 0) return 0;
        if (y <= 3 * PRECISION) return PRECISION;
        
        uint256 z = (y + PRECISION) / 2;
        uint256 x = y;
        
        while (z < x) {
            x = z;
            z = (y / z + z) / 2;
        }
        
        return x;
    }
    
    /**
     * @dev Calculate weighted average with safety checks
     * @param values Array of values
     * @param weights Array of weights (must sum to PRECISION)
     * @return Weighted average
     */
    function calculateWeightedAverage(
        uint256[] memory values,
        uint256[] memory weights
    ) internal pure returns (uint256) {
        require(values.length == weights.length, "Array length mismatch");
        require(values.length > 0, "Empty arrays");
        
        uint256 sum = 0;
        uint256 weightSum = 0;
        
        for (uint256 i = 0; i < values.length; i++) {
            sum += values[i] * weights[i];
            weightSum += weights[i];
        }
        
        require(weightSum > 0, "Zero weight sum");
        return sum / weightSum;
    }
    
    /**
     * @dev Safe division with rounding
     * @param a Numerator
     * @param b Denominator
     * @param roundUp Whether to round up or down
     * @return Result of division
     */
    function safeDivision(
        uint256 a,
        uint256 b,
        bool roundUp
    ) internal pure returns (uint256) {
        require(b > 0, "Division by zero");
        
        uint256 result = a / b;
        
        if (roundUp && a % b > 0) {
            result += 1;
        }
        
        return result;
    }
}