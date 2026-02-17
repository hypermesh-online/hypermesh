// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "../libs/AdvancedMathUtils.sol";

/**
 * @title GoldPriceOracle
 * @dev Dynamic gold price oracle with statistical analysis for Caesar Token economic engine
 * 
 * This contract implements:
 * - Real-time gold price feeds from multiple sources
 * - Rolling average calculation with configurable window
 * - Statistical standard deviation calculation for dynamic bands
 * - Volatility-adaptive thresholds
 * - Circuit breakers for extreme market conditions
 * 
 * Key Features:
 * - NO FIXED TARGET: Gold price constantly updates with market conditions
 * - STATISTICAL BANDS: Uses standard deviation, not fixed percentages
 * - REAL-TIME CALCULATION: Continuous recalculation of bands
 * - VOLATILITY ADAPTIVE: Bands expand/contract with gold market volatility
 */
contract GoldPriceOracle is Ownable, ReentrancyGuard, Pausable {
    using AdvancedMathUtils for uint256;

    // ============ Constants ============
    uint256 public constant PRECISION = 1e18;
    uint256 public constant BASIS_POINTS = 10000;
    uint256 public constant GRAMS_PER_OUNCE = 31103476800000000000; // 31.1034768 grams per troy ounce (18 decimals)
    uint256 public constant MAX_PRICE_SOURCES = 5;
    uint256 public constant MIN_PRICE_SOURCES = 2;
    uint256 public constant DEFAULT_WINDOW_SIZE = 72; // 72 hours default
    uint256 public constant MAX_DEVIATION_MULTIPLIER = 5e18; // 5.0x standard deviation
    uint256 public constant MIN_DEVIATION_MULTIPLIER = 1e18; // 1.0x standard deviation

    // ============ Structs ============
    
    struct PriceSource {
        string name;
        address oracle;
        uint256 weight;
        uint256 lastPrice;
        uint256 lastUpdate;
        bool isActive;
        uint256 errorCount;
    }

    struct PriceData {
        uint256 price; // USD per gram (18 decimals)
        uint256 timestamp;
        uint256 confidence; // 0-1000 (1000 = 100% confidence)
        uint32 sourceCount; // Number of sources contributing
    }

    struct StatisticalMetrics {
        uint256 rollingAverage;     // Current rolling average
        uint256 standardDeviation;  // Current standard deviation
        uint256 upperBand;          // Upper statistical band
        uint256 lowerBand;          // Lower statistical band
        uint256 volatility;         // Current volatility measure
        uint256 lastCalculation;    // Timestamp of last calculation
    }

    struct MarketConditions {
        uint256 volatilityIndex;    // 0-1000 (1000 = extremely volatile)
        uint256 confidenceScore;    // 0-1000 (1000 = extremely confident)
        uint256 trendDirection;     // 0=down, 500=flat, 1000=up
        uint256 marketStress;       // 0-1000 (1000 = extreme stress)
    }

    // ============ State Variables ============
    
    // Price history for statistical calculations
    PriceData[] public priceHistory;
    mapping(uint256 => PriceData) public historicalPrices; // timestamp => PriceData
    
    // Price sources configuration
    PriceSource[] public priceSources;
    mapping(address => uint256) public sourceIndex;
    
    // Statistical metrics
    StatisticalMetrics public currentMetrics;
    MarketConditions public marketConditions;
    
    // Configuration
    uint256 public windowSize = DEFAULT_WINDOW_SIZE; // Hours for rolling calculations
    uint256 public deviationMultiplier = 2e18; // 2.0x standard deviation default
    uint256 public updateFrequency = 300; // 5 minutes default
    uint256 public maxPriceAge = 3600; // 1 hour max age
    
    // Emergency controls
    bool public emergencyMode;
    uint256 public emergencyPrice;
    uint256 public lastEmergencyUpdate;
    
    // Authorized updaters (for off-chain data feeds)
    mapping(address => bool) public authorizedUpdaters;
    
    // ============ Events ============
    
    event PriceUpdated(
        uint256 indexed timestamp,
        uint256 price,
        uint256 confidence,
        uint32 sourceCount
    );
    
    event StatisticalMetricsUpdated(
        uint256 indexed timestamp,
        uint256 rollingAverage,
        uint256 standardDeviation,
        uint256 upperBand,
        uint256 lowerBand
    );
    
    event PriceSourceAdded(
        address indexed oracle,
        string name,
        uint256 weight
    );
    
    event PriceSourceUpdated(
        address indexed oracle,
        uint256 newWeight,
        bool isActive
    );
    
    event EmergencyModeActivated(
        uint256 timestamp,
        uint256 emergencyPrice,
        string reason
    );
    
    event MarketConditionsUpdated(
        uint256 volatilityIndex,
        uint256 confidenceScore,
        uint256 trendDirection,
        uint256 marketStress
    );

    // ============ Constructor ============
    
    constructor(address initialOwner) Ownable(initialOwner) {
        // Initialize with reasonable defaults
        currentMetrics = StatisticalMetrics({
            rollingAverage: 117e18,  // ~$117/gram current estimate
            standardDeviation: 5e18, // Initial $5/gram std dev
            upperBand: 122e18,       // Initial upper band
            lowerBand: 112e18,       // Initial lower band
            volatility: 100,         // Low initial volatility
            lastCalculation: block.timestamp
        });
        
        marketConditions = MarketConditions({
            volatilityIndex: 100,    // Low initial volatility
            confidenceScore: 800,    // High initial confidence
            trendDirection: 500,     // Neutral trend
            marketStress: 100        // Low initial stress
        });
    }

    // ============ Core Oracle Functions ============
    
    /**
     * @dev Get current gold price with confidence metrics
     * @return price Current gold price in USD per gram
     * @return confidence Confidence level (0-1000)
     * @return timestamp Last update timestamp
     */
    function getCurrentGoldPrice() external view returns (
        uint256 price,
        uint256 confidence,
        uint256 timestamp
    ) {
        if (emergencyMode) {
            return (emergencyPrice, 1000, lastEmergencyUpdate);
        }
        
        if (priceHistory.length == 0) {
            return (currentMetrics.rollingAverage, 500, block.timestamp);
        }
        
        PriceData memory latest = priceHistory[priceHistory.length - 1];
        
        // Check if price is stale
        if (block.timestamp - latest.timestamp > maxPriceAge) {
            return (currentMetrics.rollingAverage, latest.confidence / 2, latest.timestamp);
        }
        
        return (latest.price, latest.confidence, latest.timestamp);
    }
    
    /**
     * @dev Get statistical bands for economic engine
     * @return average Current rolling average
     * @return stdDev Current standard deviation
     * @return upperBand Upper statistical band
     * @return lowerBand Lower statistical band
     * @return multiplier Current deviation multiplier
     */
    function getStatisticalBands() external view returns (
        uint256 average,
        uint256 stdDev,
        uint256 upperBand,
        uint256 lowerBand,
        uint256 multiplier
    ) {
        return (
            currentMetrics.rollingAverage,
            currentMetrics.standardDeviation,
            currentMetrics.upperBand,
            currentMetrics.lowerBand,
            deviationMultiplier
        );
    }
    
    /**
     * @dev Calculate deviation score for a given price
     * @param price Price to calculate deviation for
     * @return deviationScore Standard deviations from average (-5.0 to +5.0 scaled to 18 decimals)
     */
    function calculateDeviationScore(uint256 price) external view returns (int256 deviationScore) {
        if (currentMetrics.standardDeviation == 0) {
            return 0;
        }
        
        int256 priceDiff = int256(price) - int256(currentMetrics.rollingAverage);
        int256 deviation = (priceDiff * int256(PRECISION)) / int256(currentMetrics.standardDeviation);
        
        // Cap at ±5 standard deviations
        if (deviation > int256(MAX_DEVIATION_MULTIPLIER)) {
            return int256(MAX_DEVIATION_MULTIPLIER);
        } else if (deviation < -int256(MAX_DEVIATION_MULTIPLIER)) {
            return -int256(MAX_DEVIATION_MULTIPLIER);
        }
        
        return deviation;
    }
    
    /**
     * @dev Update gold price from authorized source
     * @param pricePerGram Price in USD per gram (18 decimals)
     * @param confidence Confidence level (0-1000)
     */
    function updatePrice(
        uint256 pricePerGram,
        uint256 confidence
    ) external nonReentrant whenNotPaused {
        require(authorizedUpdaters[msg.sender] || msg.sender == owner(), "Unauthorized");
        require(confidence <= 1000, "Invalid confidence");
        require(pricePerGram > 0, "Invalid price");
        
        // Validate price is reasonable (between $10 and $1000 per gram)
        require(pricePerGram >= 10e18 && pricePerGram <= 1000e18, "Price out of range");
        
        _addPriceData(pricePerGram, confidence, 1);
        _updateStatisticalMetrics();
        _updateMarketConditions();
        
        emit PriceUpdated(block.timestamp, pricePerGram, confidence, 1);
    }
    
    /**
     * @dev Update price from multiple sources with weighted average
     * @param prices Array of prices from different sources
     * @param confidences Array of confidence levels
     * @param sourceIndices Array of source indices
     */
    function updatePriceMultiSource(
        uint256[] calldata prices,
        uint256[] calldata confidences,
        uint256[] calldata sourceIndices
    ) external nonReentrant whenNotPaused {
        require(authorizedUpdaters[msg.sender] || msg.sender == owner(), "Unauthorized");
        require(prices.length == confidences.length, "Array length mismatch");
        require(prices.length == sourceIndices.length, "Array length mismatch");
        require(prices.length >= MIN_PRICE_SOURCES, "Too few sources");
        require(prices.length <= MAX_PRICE_SOURCES, "Too many sources");
        
        uint256 weightedSum = 0;
        uint256 totalWeight = 0;
        uint256 avgConfidence = 0;
        
        for (uint256 i = 0; i < prices.length; i++) {
            require(sourceIndices[i] < priceSources.length, "Invalid source index");
            require(priceSources[sourceIndices[i]].isActive, "Source inactive");
            require(prices[i] >= 10e18 && prices[i] <= 1000e18, "Price out of range");
            require(confidences[i] <= 1000, "Invalid confidence");
            
            uint256 weight = priceSources[sourceIndices[i]].weight;
            weightedSum += prices[i] * weight;
            totalWeight += weight;
            avgConfidence += confidences[i];
            
            // Update source data
            priceSources[sourceIndices[i]].lastPrice = prices[i];
            priceSources[sourceIndices[i]].lastUpdate = block.timestamp;
        }
        
        require(totalWeight > 0, "No valid sources");
        
        uint256 weightedPrice = weightedSum / totalWeight;
        uint256 finalConfidence = avgConfidence / prices.length;
        
        _addPriceData(weightedPrice, finalConfidence, uint32(prices.length));
        _updateStatisticalMetrics();
        _updateMarketConditions();
        
        emit PriceUpdated(block.timestamp, weightedPrice, finalConfidence, uint32(prices.length));
    }
    
    // ============ Statistical Calculation Functions ============
    
    /**
     * @dev Update statistical metrics based on price history
     */
    function _updateStatisticalMetrics() internal {
        if (priceHistory.length < 2) {
            return;
        }
        
        uint256 startIndex = priceHistory.length > windowSize ? 
            priceHistory.length - windowSize : 0;
        
        // Calculate rolling average
        uint256 sum = 0;
        uint256 count = 0;
        
        for (uint256 i = startIndex; i < priceHistory.length; i++) {
            sum += priceHistory[i].price;
            count++;
        }
        
        uint256 average = sum / count;
        
        // Calculate standard deviation
        uint256 varianceSum = 0;
        for (uint256 i = startIndex; i < priceHistory.length; i++) {
            uint256 diff = priceHistory[i].price > average ? 
                priceHistory[i].price - average : 
                average - priceHistory[i].price;
            varianceSum += (diff * diff) / PRECISION;
        }
        
        uint256 variance = varianceSum / count;
        uint256 standardDeviation = variance.sqrt();
        
        // Calculate adaptive multiplier based on volatility
        uint256 adaptiveMultiplier = _calculateAdaptiveMultiplier(standardDeviation, average);
        
        // Calculate bands
        uint256 bandWidth = (standardDeviation * adaptiveMultiplier) / PRECISION;
        uint256 upperBand = average + bandWidth;
        uint256 lowerBand = average > bandWidth ? average - bandWidth : 0;
        
        // Update current metrics
        currentMetrics = StatisticalMetrics({
            rollingAverage: average,
            standardDeviation: standardDeviation,
            upperBand: upperBand,
            lowerBand: lowerBand,
            volatility: _calculateVolatility(startIndex),
            lastCalculation: block.timestamp
        });
        
        emit StatisticalMetricsUpdated(
            block.timestamp,
            average,
            standardDeviation,
            upperBand,
            lowerBand
        );
    }
    
    /**
     * @dev Calculate adaptive multiplier based on market volatility
     * @param stdDev Current standard deviation
     * @param average Current average price
     * @return multiplier Adaptive multiplier for bands
     */
    function _calculateAdaptiveMultiplier(
        uint256 stdDev,
        uint256 average
    ) internal view returns (uint256 multiplier) {
        if (average == 0) return deviationMultiplier;
        
        // Calculate coefficient of variation (CV = stdDev / average)
        uint256 coefficientOfVariation = (stdDev * PRECISION) / average;
        
        // Adaptive multiplier based on CV
        // High volatility -> larger bands
        // Low volatility -> smaller bands
        if (coefficientOfVariation > 200e15) { // CV > 20%
            multiplier = deviationMultiplier + 1e18; // +1 std dev
        } else if (coefficientOfVariation > 100e15) { // CV > 10%
            multiplier = deviationMultiplier + 5e17; // +0.5 std dev
        } else if (coefficientOfVariation < 20e15) { // CV < 2%
            multiplier = deviationMultiplier > 5e17 ? deviationMultiplier - 5e17 : 5e17; // -0.5 std dev
        } else {
            multiplier = deviationMultiplier; // Default multiplier
        }
        
        // Ensure multiplier stays within bounds
        if (multiplier > MAX_DEVIATION_MULTIPLIER) {
            multiplier = MAX_DEVIATION_MULTIPLIER;
        } else if (multiplier < MIN_DEVIATION_MULTIPLIER) {
            multiplier = MIN_DEVIATION_MULTIPLIER;
        }
        
        return multiplier;
    }
    
    /**
     * @dev Calculate volatility measure
     * @param startIndex Start index for calculation window
     * @return volatility Volatility measure (0-1000)
     */
    function _calculateVolatility(uint256 startIndex) internal view returns (uint256 volatility) {
        if (priceHistory.length - startIndex < 2) {
            return currentMetrics.volatility; // Return previous volatility
        }
        
        uint256 maxPrice = priceHistory[startIndex].price;
        uint256 minPrice = priceHistory[startIndex].price;
        
        // Find max and min in window
        for (uint256 i = startIndex; i < priceHistory.length; i++) {
            if (priceHistory[i].price > maxPrice) {
                maxPrice = priceHistory[i].price;
            }
            if (priceHistory[i].price < minPrice) {
                minPrice = priceHistory[i].price;
            }
        }
        
        // Calculate volatility as percentage range
        if (minPrice == 0) return 1000; // Maximum volatility
        
        uint256 range = ((maxPrice - minPrice) * 1000) / minPrice;
        return range > 1000 ? 1000 : range;
    }
    
    /**
     * @dev Update market conditions based on recent data
     */
    function _updateMarketConditions() internal {
        if (priceHistory.length < 5) return;
        
        uint256 volatilityIndex = currentMetrics.volatility;
        uint256 confidenceScore = _calculateConfidenceScore();
        uint256 trendDirection = _calculateTrendDirection();
        uint256 marketStress = _calculateMarketStress();
        
        marketConditions = MarketConditions({
            volatilityIndex: volatilityIndex,
            confidenceScore: confidenceScore,
            trendDirection: trendDirection,
            marketStress: marketStress
        });
        
        emit MarketConditionsUpdated(
            volatilityIndex,
            confidenceScore,
            trendDirection,
            marketStress
        );
    }
    
    /**
     * @dev Add new price data to history
     * @param price Price to add
     * @param confidence Confidence level
     * @param sourceCount Number of sources
     */
    function _addPriceData(
        uint256 price,
        uint256 confidence,
        uint32 sourceCount
    ) internal {
        PriceData memory newPrice = PriceData({
            price: price,
            timestamp: block.timestamp,
            confidence: confidence,
            sourceCount: sourceCount
        });
        
        priceHistory.push(newPrice);
        historicalPrices[block.timestamp] = newPrice;
        
        // Keep only recent history to manage storage
        if (priceHistory.length > windowSize * 2) {
            // Remove oldest entries
            for (uint256 i = 0; i < windowSize / 2; i++) {
                delete historicalPrices[priceHistory[i].timestamp];
            }
            
            // Shift array (this is gas-expensive, consider circular buffer in production)
            for (uint256 i = windowSize / 2; i < priceHistory.length; i++) {
                priceHistory[i - windowSize / 2] = priceHistory[i];
            }
            
            // Resize array
            for (uint256 i = 0; i < windowSize / 2; i++) {
                priceHistory.pop();
            }
        }
    }
    
    /**
     * @dev Calculate overall confidence score
     * @return confidence Overall confidence (0-1000)
     */
    function _calculateConfidenceScore() internal view returns (uint256 confidence) {
        if (priceHistory.length == 0) return 500;
        
        uint256 recentCount = priceHistory.length > 10 ? 10 : priceHistory.length;
        uint256 startIndex = priceHistory.length - recentCount;
        uint256 totalConfidence = 0;
        
        for (uint256 i = startIndex; i < priceHistory.length; i++) {
            totalConfidence += priceHistory[i].confidence;
        }
        
        return totalConfidence / recentCount;
    }
    
    /**
     * @dev Calculate trend direction
     * @return trend Trend direction (0=down, 500=flat, 1000=up)
     */
    function _calculateTrendDirection() internal view returns (uint256 trend) {
        if (priceHistory.length < 2) return 500;
        
        uint256 recentCount = priceHistory.length > 20 ? 20 : priceHistory.length;
        uint256 startIndex = priceHistory.length - recentCount;
        
        uint256 startPrice = priceHistory[startIndex].price;
        uint256 endPrice = priceHistory[priceHistory.length - 1].price;
        
        if (endPrice > startPrice) {
            uint256 increase = ((endPrice - startPrice) * 500) / startPrice;
            return 500 + (increase > 500 ? 500 : increase);
        } else if (endPrice < startPrice) {
            uint256 decrease = ((startPrice - endPrice) * 500) / startPrice;
            return decrease > 500 ? 0 : 500 - decrease;
        } else {
            return 500; // No change
        }
    }
    
    /**
     * @dev Calculate market stress level
     * @return stress Market stress (0-1000)
     */
    function _calculateMarketStress() internal view returns (uint256 stress) {
        uint256 volatilityStress = currentMetrics.volatility;
        uint256 confidenceStress = 1000 - _calculateConfidenceScore();
        
        // Check for unusual price movements
        uint256 deviationStress = 0;
        if (priceHistory.length > 0) {
            int256 deviation = this.calculateDeviationScore(
                priceHistory[priceHistory.length - 1].price
            );
            uint256 absDeviation = deviation < 0 ? uint256(-deviation) : uint256(deviation);
            deviationStress = absDeviation > PRECISION ? 1000 : (absDeviation * 1000) / PRECISION;
        }
        
        // Weighted combination of stress factors
        return (volatilityStress * 40 + confidenceStress * 30 + deviationStress * 30) / 100;
    }
    
    // ============ Admin Functions ============
    
    /**
     * @dev Add new price source
     * @param oracle Oracle contract address
     * @param name Source name
     * @param weight Source weight for averaging
     */
    function addPriceSource(
        address oracle,
        string calldata name,
        uint256 weight
    ) external onlyOwner {
        require(oracle != address(0), "Invalid oracle address");
        require(weight > 0 && weight <= 1000, "Invalid weight");
        require(priceSources.length < MAX_PRICE_SOURCES, "Too many sources");
        
        priceSources.push(PriceSource({
            name: name,
            oracle: oracle,
            weight: weight,
            lastPrice: 0,
            lastUpdate: 0,
            isActive: true,
            errorCount: 0
        }));
        
        sourceIndex[oracle] = priceSources.length - 1;
        
        emit PriceSourceAdded(oracle, name, weight);
    }
    
    /**
     * @dev Update price source configuration
     * @param oracle Oracle address
     * @param weight New weight
     * @param isActive Whether source is active
     */
    function updatePriceSource(
        address oracle,
        uint256 weight,
        bool isActive
    ) external onlyOwner {
        uint256 index = sourceIndex[oracle];
        require(index < priceSources.length, "Source not found");
        require(weight > 0 && weight <= 1000, "Invalid weight");
        
        priceSources[index].weight = weight;
        priceSources[index].isActive = isActive;
        
        emit PriceSourceUpdated(oracle, weight, isActive);
    }
    
    /**
     * @dev Set authorized updater
     * @param updater Address to authorize/deauthorize
     * @param authorized Whether address is authorized
     */
    function setAuthorizedUpdater(address updater, bool authorized) external onlyOwner {
        authorizedUpdaters[updater] = authorized;
    }
    
    /**
     * @dev Update oracle configuration
     * @param _windowSize New window size for calculations
     * @param _deviationMultiplier New deviation multiplier
     * @param _updateFrequency New update frequency
     * @param _maxPriceAge New maximum price age
     */
    function updateConfiguration(
        uint256 _windowSize,
        uint256 _deviationMultiplier,
        uint256 _updateFrequency,
        uint256 _maxPriceAge
    ) external onlyOwner {
        require(_windowSize >= 12 && _windowSize <= 168, "Invalid window size"); // 12 hours to 1 week
        require(_deviationMultiplier >= MIN_DEVIATION_MULTIPLIER && 
                _deviationMultiplier <= MAX_DEVIATION_MULTIPLIER, "Invalid multiplier");
        require(_updateFrequency >= 60 && _updateFrequency <= 3600, "Invalid frequency"); // 1 min to 1 hour
        require(_maxPriceAge >= _updateFrequency && _maxPriceAge <= 86400, "Invalid max age"); // Up to 1 day
        
        windowSize = _windowSize;
        deviationMultiplier = _deviationMultiplier;
        updateFrequency = _updateFrequency;
        maxPriceAge = _maxPriceAge;
    }
    
    /**
     * @dev Activate emergency mode with manual price
     * @param price Emergency price to use
     * @param reason Reason for emergency activation
     */
    function activateEmergencyMode(uint256 price, string calldata reason) external onlyOwner {
        require(price >= 10e18 && price <= 1000e18, "Price out of range");
        
        emergencyMode = true;
        emergencyPrice = price;
        lastEmergencyUpdate = block.timestamp;
        
        emit EmergencyModeActivated(block.timestamp, price, reason);
    }
    
    /**
     * @dev Deactivate emergency mode
     */
    function deactivateEmergencyMode() external onlyOwner {
        emergencyMode = false;
        emergencyPrice = 0;
        lastEmergencyUpdate = 0;
    }
    
    // ============ View Functions ============
    
    function getPriceHistoryLength() external view returns (uint256) {
        return priceHistory.length;
    }
    
    function getPriceHistory(uint256 start, uint256 end) 
        external 
        view 
        returns (PriceData[] memory) 
    {
        require(start <= end, "Invalid range");
        require(end < priceHistory.length, "End out of bounds");
        
        PriceData[] memory result = new PriceData[](end - start + 1);
        for (uint256 i = start; i <= end; i++) {
            result[i - start] = priceHistory[i];
        }
        
        return result;
    }
    
    function getPriceSourcesCount() external view returns (uint256) {
        return priceSources.length;
    }
    
    function getCurrentMetrics() external view returns (StatisticalMetrics memory) {
        return currentMetrics;
    }
    
    function getMarketConditions() external view returns (MarketConditions memory) {
        return marketConditions;
    }
    
    function isHealthy() external view returns (bool) {
        return !emergencyMode && 
               marketConditions.marketStress < 800 &&
               marketConditions.confidenceScore > 300;
    }
}