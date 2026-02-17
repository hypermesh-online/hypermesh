// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "../libs/AdvancedMathUtils.sol";
import "../interfaces/ICaesar.sol";
import "../interfaces/IEconomicEngine.sol";

/**
 * @title AdvancedDemurrageManager
 * @dev Advanced demurrage system with mathematical precision, stability-based adjustments,
 * fiat activity discounts, and grace periods for Caesar Token economic model
 */
contract AdvancedDemurrageManager is Ownable, ReentrancyGuard, Pausable {
    using AdvancedMathUtils for uint256;
    
    // ============ Constants ============
    uint256 public constant PRECISION = 1e18;
    uint256 public constant BASIS_POINTS = 10000;
    uint256 public constant SECONDS_PER_HOUR = 3600;
    uint256 public constant MIN_DEMURRAGE_RATE = 5;   // 0.05% per hour minimum
    uint256 public constant MAX_DEMURRAGE_RATE = 200; // 2% per hour maximum
    uint256 public constant MAX_FIAT_DISCOUNT = 5000; // 50% maximum discount
    uint256 public constant MAX_GRACE_PERIOD = 168 hours; // 7 days maximum
    
    // ============ Structs ============
    
    struct AdvancedDemurrageConfig {
        uint256 baseRate;                // Base demurrage rate (basis points per hour)
        uint256 maxRate;                 // Maximum demurrage rate (basis points per hour)
        uint256 stabilityThreshold;      // Price stability threshold (basis points)
        uint256 fiatDiscountFactor;      // Maximum fiat activity discount (basis points)
        uint256 gracePeriodsHours;       // Grace period for new users (hours)
        uint256 decayAcceleration;       // Rate at which demurrage accelerates
        uint256 velocityThreshold;       // Money velocity threshold for adjustments
        bool adaptiveRateEnabled;        // Whether adaptive rate adjustment is enabled
    }
    
    struct AccountDemurrageData {
        uint256 lastDemurrageApplication;    // Last demurrage application timestamp
        uint256 totalDemurragePaid;          // Cumulative demurrage paid
        uint256 effectiveRate;               // Current effective demurrage rate
        uint256 graceEndTime;                // When grace period ends (0 if not applicable)
        uint256 lastFiatActivity;            // Last fiat activity timestamp
        uint256 fiatVolume30Days;            // Fiat activity volume in last 30 days
        bool isExempt;                       // Whether account is exempt
        bool isNewUser;                      // Whether account is in grace period
    }
    
    struct StabilityMetrics {
        uint256 currentStabilityIndex;       // Current stability index (0-1000)
        uint256 priceDeviation;              // Price deviation from target (basis points)
        uint256 velocityRatio;               // Money velocity ratio
        uint256 participationRate;           // Network participation rate
        uint256 lastStabilityUpdate;        // Last stability metrics update
    }
    
    struct FiatActivityDiscount {
        uint256 volume24h;                   // Fiat volume in last 24 hours
        uint256 volume7d;                    // Fiat volume in last 7 days
        uint256 volume30d;                   // Fiat volume in last 30 days
        uint256 discountPercentage;          // Current discount percentage
        bool isEligible;                     // Whether eligible for discount
        uint256 lastActivityTime;            // Last fiat activity timestamp
    }
    
    // ============ State Variables ============
    
    AdvancedDemurrageConfig public config;
    StabilityMetrics public stabilityMetrics;
    
    // Account tracking
    mapping(address => AccountDemurrageData) public accountData;
    mapping(address => FiatActivityDiscount) public fiatDiscounts;
    mapping(address => bool) public exemptAccounts;
    
    // Global metrics
    uint256 public totalDemurrageCollected;
    uint256 public totalAccountsProcessed;
    uint256 public averageDemurrageRate;
    uint256 public lastGlobalUpdate;
    
    // Fiat integration
    address public fiatOracle; // Oracle for fiat activity validation
    mapping(address => bool) public authorizedFiatReporters;
    
    // Emergency controls
    bool public emergencyMode;
    uint256 public emergencyRateOverride;
    mapping(address => bool) public emergencyOperators;
    
    // ============ Events ============
    
    event DemurrageConfigUpdated(AdvancedDemurrageConfig config);
    event DemurrageApplied(address indexed account, uint256 amount, uint256 effectiveRate);
    event AccountExemptionSet(address indexed account, bool exempt);
    event StabilityIndexUpdated(uint256 oldIndex, uint256 newIndex);
    event FiatActivityRecorded(address indexed account, uint256 amount, uint8 activityType);
    event FiatDiscountApplied(address indexed account, uint256 originalRate, uint256 discountedRate);
    event GracePeriodStarted(address indexed account, uint256 endTime);
    event GracePeriodEnded(address indexed account);
    event EmergencyModeActivated(address operator, uint256 rateOverride);
    event AdaptiveRateAdjustment(address indexed account, uint256 oldRate, uint256 newRate);
    
    /**
     * @dev Constructor
     * @param initialOwner Contract owner
     * @param _fiatOracle Fiat activity oracle address
     */
    constructor(address initialOwner, address _fiatOracle) Ownable(initialOwner) {
        fiatOracle = _fiatOracle;
        
        config = AdvancedDemurrageConfig({
            baseRate: 50,                    // 0.5% per hour
            maxRate: 200,                    // 2% per hour
            stabilityThreshold: 100,         // 1% stability threshold
            fiatDiscountFactor: 5000,        // Up to 50% discount
            gracePeriodsHours: 48,           // 48 hours grace period
            decayAcceleration: 10,           // 10% acceleration per instability period
            velocityThreshold: 500,          // 50% of optimal velocity
            adaptiveRateEnabled: true        // Enable adaptive rates
        });
        
        stabilityMetrics = StabilityMetrics({
            currentStabilityIndex: 1000,     // Perfect stability initially
            priceDeviation: 0,               // No deviation initially
            velocityRatio: 500,              // 50% velocity initially
            participationRate: 0,            // No participation initially
            lastStabilityUpdate: block.timestamp
        });
        
        lastGlobalUpdate = block.timestamp;
    }
    
    // ============ Core Demurrage Functions ============
    
    /**
     * @dev Calculate demurrage with all advanced features
     * @param account Account to calculate demurrage for
     * @param balance Current account balance
     * @return demurrageAmount Calculated demurrage amount
     */
    function calculateAdvancedDemurrage(
        address account,
        uint256 balance
    ) external view returns (uint256 demurrageAmount) {
        if (balance == 0 || exemptAccounts[account]) {
            return 0;
        }
        
        AccountDemurrageData memory data = accountData[account];
        
        // Check grace period
        if (data.isNewUser && block.timestamp < data.graceEndTime) {
            return 0;
        }
        
        // Calculate time elapsed since last application
        uint256 lastApplication = data.lastDemurrageApplication;
        if (lastApplication == 0) {
            lastApplication = data.graceEndTime > 0 ? data.graceEndTime : block.timestamp - SECONDS_PER_HOUR;
        }
        
        uint256 timeElapsed = block.timestamp - lastApplication;
        if (timeElapsed < SECONDS_PER_HOUR) {
            return 0; // No demurrage within first hour
        }
        
        // Calculate effective rate with all adjustments
        uint256 effectiveRate = _calculateEffectiveRate(account);
        
        // Apply exponential decay formula: New_Balance = Original_Balance * e^(-rate * time_elapsed)
        uint256 hoursElapsed = timeElapsed / SECONDS_PER_HOUR;
        
        // Cap hours elapsed to prevent extreme demurrage (max 30 days = 720 hours)
        if (hoursElapsed > 720) {
            hoursElapsed = 720;
        }
        
        uint256 decayedBalance = balance.calculateExponentialDecay(effectiveRate, hoursElapsed);
        uint256 calculatedDemurrage = balance > decayedBalance ? balance - decayedBalance : 0;
        
        // Cap demurrage at 50% of balance to prevent "Insufficient balance" errors
        uint256 maxDemurrage = balance / 2; // 50% maximum
        
        return calculatedDemurrage > maxDemurrage ? maxDemurrage : calculatedDemurrage;
    }
    
    /**
     * @dev Apply demurrage with full tracking and metrics
     * @param account Account to apply demurrage to
     * @param balance Current account balance
     * @return appliedAmount Amount of demurrage applied
     */
    function applyAdvancedDemurrage(
        address account,
        uint256 balance
    ) external nonReentrant whenNotPaused returns (uint256 appliedAmount) {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        appliedAmount = this.calculateAdvancedDemurrage(account, balance);
        
        if (appliedAmount > 0) {
            AccountDemurrageData storage data = accountData[account];
            uint256 effectiveRate = _calculateEffectiveRate(account);
            
            // Update account data
            data.lastDemurrageApplication = block.timestamp;
            data.totalDemurragePaid += appliedAmount;
            data.effectiveRate = effectiveRate;
            
            // Update global metrics
            totalDemurrageCollected += appliedAmount;
            totalAccountsProcessed++;
            
            // Update average rate
            averageDemurrageRate = ((averageDemurrageRate * (totalAccountsProcessed - 1)) + effectiveRate) / totalAccountsProcessed;
            
            emit DemurrageApplied(account, appliedAmount, effectiveRate);
        }
        
        return appliedAmount;
    }
    
    /**
     * @dev Record fiat activity for discount calculation
     * @param account Account with fiat activity
     * @param amount Fiat activity amount (in USD equivalent)
     * @param activityType Type of activity (0=purchase, 1=redemption, 2=payment)
     */
    function recordFiatActivity(
        address account,
        uint256 amount,
        uint8 activityType
    ) external nonReentrant {
        require(authorizedFiatReporters[msg.sender] || msg.sender == owner(), "Unauthorized reporter");
        require(amount > 0, "Amount must be positive");
        
        FiatActivityDiscount storage discount = fiatDiscounts[account];
        uint256 currentTime = block.timestamp;
        
        // Update volume tracking with time-weighted decay
        _updateFiatVolumeTracking(discount, amount, currentTime);
        
        // Calculate new discount percentage
        uint256 newDiscountPercentage = _calculateFiatDiscount(account);
        discount.discountPercentage = newDiscountPercentage;
        discount.isEligible = newDiscountPercentage > 0;
        discount.lastActivityTime = currentTime;
        
        emit FiatActivityRecorded(account, amount, activityType);
        
        if (newDiscountPercentage > 0) {
            uint256 originalRate = _calculateBaseStabilityRate();
            uint256 discountedRate = _applyFiatActivityDiscount(account, originalRate);
            emit FiatDiscountApplied(account, originalRate, discountedRate);
        }
    }
    
    /**
     * @dev Start grace period for new user
     * @param account New user account
     */
    function startGracePeriod(address account) external {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        AccountDemurrageData storage data = accountData[account];
        require(!data.isNewUser, "Already in grace period");
        
        data.isNewUser = true;
        data.graceEndTime = block.timestamp + (config.gracePeriodsHours * SECONDS_PER_HOUR);
        
        emit GracePeriodStarted(account, data.graceEndTime);
    }
    
    /**
     * @dev Update stability metrics for rate adjustments
     * @param priceDeviation Current price deviation from peg (basis points)
     * @param velocityRatio Current money velocity ratio
     * @param participationRate Current network participation rate
     */
    function updateStabilityMetrics(
        uint256 priceDeviation,
        uint256 velocityRatio,
        uint256 participationRate
    ) external {
        require(msg.sender == owner() || emergencyOperators[msg.sender], "Unauthorized");
        
        uint256 oldStabilityIndex = stabilityMetrics.currentStabilityIndex;
        
        // Calculate new stability index using advanced formula
        uint256 newStabilityIndex = _calculateAdvancedStabilityIndex(
            priceDeviation,
            velocityRatio,
            participationRate
        );
        
        stabilityMetrics.currentStabilityIndex = newStabilityIndex;
        stabilityMetrics.priceDeviation = priceDeviation;
        stabilityMetrics.velocityRatio = velocityRatio;
        stabilityMetrics.participationRate = participationRate;
        stabilityMetrics.lastStabilityUpdate = block.timestamp;
        
        emit StabilityIndexUpdated(oldStabilityIndex, newStabilityIndex);
        
        // Trigger adaptive rate adjustments if enabled
        if (config.adaptiveRateEnabled) {
            _triggerAdaptiveRateAdjustments();
        }
    }
    
    // ============ Configuration Functions ============
    
    /**
     * @dev Update demurrage configuration
     * @param newConfig New configuration parameters
     */
    function updateDemurrageConfig(
        AdvancedDemurrageConfig calldata newConfig
    ) external onlyOwner {
        require(newConfig.baseRate >= MIN_DEMURRAGE_RATE, "Base rate too low");
        require(newConfig.maxRate <= MAX_DEMURRAGE_RATE, "Max rate too high");
        require(newConfig.baseRate <= newConfig.maxRate, "Invalid rate range");
        require(newConfig.fiatDiscountFactor <= MAX_FIAT_DISCOUNT, "Discount factor too high");
        require(newConfig.gracePeriodsHours <= MAX_GRACE_PERIOD / SECONDS_PER_HOUR, "Grace period too long");
        
        config = newConfig;
        emit DemurrageConfigUpdated(newConfig);
    }
    
    /**
     * @dev Set account exemption status
     * @param account Account address
     * @param exempt Whether to exempt account
     */
    function setAccountExemption(address account, bool exempt) external onlyOwner {
        exemptAccounts[account] = exempt;
        accountData[account].isExempt = exempt;
        emit AccountExemptionSet(account, exempt);
    }
    
    /**
     * @dev Add authorized fiat reporter
     * @param reporter Reporter address
     */
    function addFiatReporter(address reporter) external onlyOwner {
        authorizedFiatReporters[reporter] = true;
    }
    
    /**
     * @dev Remove authorized fiat reporter
     * @param reporter Reporter address
     */
    function removeFiatReporter(address reporter) external onlyOwner {
        authorizedFiatReporters[reporter] = false;
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
    
    // ============ Emergency Functions ============
    
    /**
     * @dev Activate emergency mode with rate override
     * @param rateOverride Emergency rate override (0 to pause demurrage)
     */
    function activateEmergencyMode(uint256 rateOverride) external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        require(rateOverride <= MAX_DEMURRAGE_RATE, "Rate override too high");
        
        emergencyMode = true;
        emergencyRateOverride = rateOverride;
        
        emit EmergencyModeActivated(msg.sender, rateOverride);
    }
    
    /**
     * @dev Deactivate emergency mode
     */
    function deactivateEmergencyMode() external onlyOwner {
        emergencyMode = false;
        emergencyRateOverride = 0;
    }
    
    // ============ View Functions ============
    
    function getAdvancedDemurrageConfig() external view returns (AdvancedDemurrageConfig memory) {
        return config;
    }
    
    function getAccountDemurrageData(address account) external view returns (AccountDemurrageData memory) {
        return accountData[account];
    }
    
    function getFiatDiscountData(address account) external view returns (FiatActivityDiscount memory) {
        return fiatDiscounts[account];
    }
    
    function getStabilityMetrics() external view returns (StabilityMetrics memory) {
        return stabilityMetrics;
    }
    
    function getCurrentEffectiveRate(address account) external view returns (uint256) {
        return _calculateEffectiveRate(account);
    }
    
    function isAccountInGracePeriod(address account) external view returns (bool) {
        AccountDemurrageData memory data = accountData[account];
        return data.isNewUser && block.timestamp < data.graceEndTime;
    }
    
    // ============ Internal Functions ============
    
    function _calculateEffectiveRate(address account) internal view returns (uint256) {
        if (exemptAccounts[account]) {
            return 0;
        }
        
        if (emergencyMode) {
            return emergencyRateOverride;
        }
        
        // Start with base stability-adjusted rate
        uint256 baseRate = _calculateBaseStabilityRate();
        
        // Apply fiat activity discount
        uint256 discountedRate = _applyFiatActivityDiscount(account, baseRate);
        
        // Apply adaptive adjustments if enabled
        if (config.adaptiveRateEnabled) {
            discountedRate = _applyAdaptiveAdjustments(account, discountedRate);
        }
        
        // Ensure rate is within bounds
        return _clampRate(discountedRate);
    }
    
    function _calculateBaseStabilityRate() internal view returns (uint256) {
        uint256 stabilityIndex = stabilityMetrics.currentStabilityIndex;
        uint256 baseRate = config.baseRate;
        uint256 maxRate = config.maxRate;
        
        if (stabilityIndex >= 950) {
            // Very stable - minimal demurrage
            return baseRate / 4;
        } else if (stabilityIndex <= 200) {
            // Very unstable - maximum demurrage
            return maxRate;
        }
        
        // Linear interpolation based on stability
        // Rate increases as stability decreases
        uint256 instability = 1000 - stabilityIndex;
        uint256 rateRange = maxRate - baseRate;
        uint256 additionalRate = (rateRange * instability) / 800; // Scale factor
        
        return baseRate + additionalRate;
    }
    
    function _applyFiatActivityDiscount(address account, uint256 rate) internal view returns (uint256) {
        FiatActivityDiscount memory discount = fiatDiscounts[account];
        
        if (!discount.isEligible || discount.discountPercentage == 0) {
            return rate;
        }
        
        uint256 discountAmount = (rate * discount.discountPercentage) / BASIS_POINTS;
        return rate > discountAmount ? rate - discountAmount : 0;
    }
    
    function _applyAdaptiveAdjustments(address account, uint256 rate) internal view returns (uint256) {
        AccountDemurrageData memory data = accountData[account];
        
        // Adjust based on account's historical behavior
        // Accounts with consistent low-risk behavior get rate reduction
        if (data.totalDemurragePaid > 0 && data.lastDemurrageApplication > 0) {
            uint256 timeSinceLastPayment = block.timestamp - data.lastDemurrageApplication;
            
            if (timeSinceLastPayment >= 30 days) {
                // Long-term holders get bonus reduction
                uint256 loyaltyDiscount = rate / 10; // 10% discount
                return rate > loyaltyDiscount ? rate - loyaltyDiscount : rate / 2;
            }
        }
        
        return rate;
    }
    
    function _calculateFiatDiscount(address account) internal view returns (uint256) {
        FiatActivityDiscount memory discount = fiatDiscounts[account];
        uint256 currentTime = block.timestamp;
        
        // Time-decay the discount eligibility
        if (currentTime > discount.lastActivityTime + 30 days) {
            return 0; // No discount after 30 days of inactivity
        }
        
        // Calculate discount based on recent activity
        uint256 volume24h = _getVolumeInPeriod(discount, currentTime - 1 days);
        uint256 volume7d = _getVolumeInPeriod(discount, currentTime - 7 days);
        
        if (volume24h >= 1000 * PRECISION) {
            return config.fiatDiscountFactor; // Maximum discount
        } else if (volume7d >= 5000 * PRECISION) {
            return config.fiatDiscountFactor * 3 / 4; // 75% of max discount
        } else if (volume7d >= 1000 * PRECISION) {
            return config.fiatDiscountFactor / 2; // 50% of max discount
        } else if (volume7d >= 100 * PRECISION) {
            return config.fiatDiscountFactor / 4; // 25% of max discount
        }
        
        return 0;
    }
    
    function _updateFiatVolumeTracking(
        FiatActivityDiscount storage discount,
        uint256 amount,
        uint256 currentTime
    ) internal {
        // Simple time-weighted decay for volume tracking
        uint256 timeSinceLastActivity = currentTime - discount.lastActivityTime;
        
        if (timeSinceLastActivity > 0) {
            // Apply decay to existing volumes
            if (timeSinceLastActivity >= 24 hours) {
                discount.volume24h = 0;
            } else {
                discount.volume24h = (discount.volume24h * (24 hours - timeSinceLastActivity)) / 24 hours;
            }
            
            if (timeSinceLastActivity >= 7 days) {
                discount.volume7d = 0;
            } else {
                discount.volume7d = (discount.volume7d * (7 days - timeSinceLastActivity)) / 7 days;
            }
            
            if (timeSinceLastActivity >= 30 days) {
                discount.volume30d = 0;
            } else {
                discount.volume30d = (discount.volume30d * (30 days - timeSinceLastActivity)) / 30 days;
            }
        }
        
        // Add new activity
        discount.volume24h += amount;
        discount.volume7d += amount;
        discount.volume30d += amount;
    }
    
    function _getVolumeInPeriod(
        FiatActivityDiscount memory discount,
        uint256 periodStart
    ) internal pure returns (uint256) {
        if (discount.lastActivityTime < periodStart) {
            return 0;
        }
        
        // This is a simplified implementation
        // In practice, you'd need more sophisticated time-series tracking
        return discount.volume24h;
    }
    
    function _calculateAdvancedStabilityIndex(
        uint256 priceDeviation,
        uint256 velocityRatio,
        uint256 participationRate
    ) internal pure returns (uint256) {
        // Advanced stability calculation using weighted factors
        uint256 priceStability = priceDeviation > 1000 ? 0 : 1000 - priceDeviation;
        uint256 velocityStability = velocityRatio > 1000 ? 1000 : velocityRatio;
        uint256 participationStability = participationRate > 1000 ? 1000 : participationRate;
        
        // Weighted combination: 50% price, 30% velocity, 20% participation
        return (priceStability * 50 + velocityStability * 30 + participationStability * 20) / 100;
    }
    
    function _triggerAdaptiveRateAdjustments() internal {
        // This would trigger rate adjustments for accounts based on new stability metrics
        // Implementation would depend on how many accounts to process at once
        // For gas efficiency, this might be done in batches or triggered externally
    }
    
    function _clampRate(uint256 rate) internal view returns (uint256) {
        if (rate < MIN_DEMURRAGE_RATE) {
            return MIN_DEMURRAGE_RATE;
        } else if (rate > MAX_DEMURRAGE_RATE) {
            return MAX_DEMURRAGE_RATE;
        }
        return rate;
    }
    
    /**
     * @dev Pause contract operations
     */
    function pause() external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        _pause();
    }
    
    /**
     * @dev Unpause contract operations
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}