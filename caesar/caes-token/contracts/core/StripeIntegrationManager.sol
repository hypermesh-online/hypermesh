// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "../interfaces/IEconomicEngine.sol";

/**
 * @title StripeIntegrationManager
 * @dev Stripe Connect integration for real-time fiat backing validation,
 * fiat activity tracking, and demurrage discount calculation for Caesar Token
 */
contract StripeIntegrationManager is Ownable, ReentrancyGuard, Pausable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;
    
    // ============ Constants ============
    uint256 public constant SIGNATURE_VALIDITY_PERIOD = 5 minutes;
    uint256 public constant MAX_WEBHOOK_RETRY = 3;
    uint256 public constant BACKING_VALIDATION_TOLERANCE = 500; // 5% tolerance
    uint256 public constant MIN_FIAT_ACTIVITY_FOR_DISCOUNT = 100e18; // $100 minimum
    uint256 public constant MAX_DISCOUNT_PERCENTAGE = 5000; // 50% maximum discount
    
    // ============ Enums ============
    
    enum TransactionType {
        PAYMENT,
        REFUND,
        PAYOUT,
        TRANSFER,
        PURCHASE,
        REDEMPTION
    }
    
    enum TransactionStatus {
        PENDING,
        SUCCEEDED,
        FAILED,
        CANCELLED
    }
    
    // ============ Structs ============
    
    struct StripeAccountData {
        string stripeAccountId;              // Stripe Connect account ID
        address ethereumAddress;             // Associated Ethereum address
        bool isVerified;                     // Whether account is verified
        uint256 totalVolume;                 // Total fiat volume processed
        uint256 lastActivity;                // Last fiat activity timestamp
        uint256 currentBalance;              // Current fiat balance
        uint8 accountType;                   // Account type (0=individual, 1=business)
        string businessCategory;             // Business category for risk assessment
        uint256 riskScore;                   // Risk score (0-1000)
        bool isBlocked;                      // Whether account is blocked
    }
    
    struct FiatTransaction {
        string transactionId;                // Transaction ID
        string stripePaymentId;              // Stripe payment ID  
        address userAddress;                 // User's Ethereum address
        uint256 amount;                      // Transaction amount (in cents)
        string currency;                     // Currency code (USD, EUR, etc.)
        TransactionType transactionType;     // Transaction type enum
        TransactionStatus status;            // Transaction status enum
        uint256 timestamp;                   // Transaction timestamp
        string description;                  // Transaction description
        bytes32 hash;                        // Transaction hash for integrity
    }
    
    struct BackingValidationData {
        uint256 totalUSDCReserves;           // Total USDC reserves
        uint256 totalStripeBalance;          // Total Stripe account balance
        uint256 pendingTransactions;         // Pending transaction amount
        uint256 reservedFunds;               // Reserved funds amount
        uint256 availableFunds;              // Available funds for backing
        uint256 lastValidation;              // Last validation timestamp
        bool isValid;                        // Whether backing is currently valid
        uint256 validationNonce;             // Validation nonce for replay protection
    }
    
    struct WebhookData {
        string eventId;                      // Stripe event ID
        string eventType;                    // Event type (payment_intent.succeeded, etc.)
        bytes eventData;                     // Encoded event data
        uint256 timestamp;                   // Event timestamp
        bytes signature;                     // Stripe webhook signature
        bool processed;                      // Whether event has been processed
        uint256 retryCount;                  // Number of retry attempts
    }
    
    struct FiatActivitySummary {
        uint256 volume24h;                   // Volume in last 24 hours
        uint256 volume7d;                    // Volume in last 7 days
        uint256 volume30d;                   // Volume in last 30 days
        uint256 transactionCount24h;         // Transaction count in 24h
        uint256 averageTransactionSize;      // Average transaction size
        uint256 discountEligibility;         // Discount eligibility percentage
        uint256 lastUpdate;                  // Last summary update
    }
    
    // ============ State Variables ============
    
    // Economic engine integration
    IEconomicEngine public economicEngine;
    IERC20 public csrToken;
    
    // Stripe configuration
    string public stripePublishableKey;      // Stripe publishable key
    address public stripeWebhookSigner;      // Address authorized to sign webhooks
    mapping(string => bool) public processedEvents; // Processed Stripe events
    
    // Account management
    mapping(address => StripeAccountData) public stripeAccounts;
    mapping(string => address) public stripeIdToAddress; // Stripe ID -> Ethereum address
    mapping(address => string) public addressToStripeId; // Ethereum address -> Stripe ID
    
    // Transaction tracking
    mapping(string => FiatTransaction) public fiatTransactions;
    mapping(address => string[]) public userTransactions; // User -> transaction IDs
    mapping(address => FiatActivitySummary) public activitySummaries;
    
    // Backing validation
    BackingValidationData public backingData;
    mapping(address => bool) public authorizedValidators; // Authorized backing validators
    uint256 public lastBackingCheck;
    
    // Webhook processing
    mapping(string => WebhookData) public webhookEvents;
    string[] public pendingWebhooks;
    
    // Discount calculation
    mapping(address => uint256) public fiatDiscountPercentages;
    mapping(address => uint256) public lastDiscountCalculation;
    
    // Emergency controls
    bool public emergencyMode;
    mapping(address => bool) public emergencyOperators;
    uint256 public emergencyActivatedAt;
    
    // ============ Events ============
    
    event StripeAccountLinked(address indexed user, string stripeAccountId);
    event FiatTransactionProcessed(string transactionId, address indexed user, uint256 amount, TransactionType transactionType);
    event BackingValidated(uint256 totalBacking, uint256 requiredBacking, bool isValid);
    event FiatDiscountCalculated(address indexed user, uint256 discountPercentage);
    event WebhookProcessed(string eventId, string eventType, bool success);
    event EmergencyModeActivated(address operator, string reason);
    event StripeConfigurationUpdated(string publishableKey);
    event AccountVerificationStatusChanged(address indexed user, bool isVerified);
    
    /**
     * @dev Constructor
     * @param initialOwner Contract owner
     * @param _economicEngine Economic engine address
     * @param _stripeWebhookSigner Authorized webhook signer
     */
    constructor(
        address initialOwner,
        address _economicEngine,
        address _stripeWebhookSigner
    ) Ownable(initialOwner) {
        economicEngine = IEconomicEngine(_economicEngine);
        stripeWebhookSigner = _stripeWebhookSigner;
        lastBackingCheck = block.timestamp;
    }
    
    // ============ Account Management ============
    
    /**
     * @dev Link Stripe account to Ethereum address
     * @param stripeAccountId Stripe Connect account ID
     * @param accountType Account type (0=individual, 1=business)
     * @param businessCategory Business category for risk assessment
     */
    function linkStripeAccount(
        string calldata stripeAccountId,
        uint8 accountType,
        string calldata businessCategory
    ) external whenNotPaused {
        require(bytes(stripeAccountId).length > 0, "Invalid Stripe account ID");
        require(stripeIdToAddress[stripeAccountId] == address(0), "Stripe account already linked");
        require(bytes(addressToStripeId[msg.sender]).length == 0, "Address already has Stripe account");
        
        // Create account data
        stripeAccounts[msg.sender] = StripeAccountData({
            stripeAccountId: stripeAccountId,
            ethereumAddress: msg.sender,
            isVerified: false, // Requires separate verification
            totalVolume: 0,
            lastActivity: block.timestamp,
            currentBalance: 0,
            accountType: accountType,
            businessCategory: businessCategory,
            riskScore: 500, // Default medium risk
            isBlocked: false
        });
        
        // Update mappings
        stripeIdToAddress[stripeAccountId] = msg.sender;
        addressToStripeId[msg.sender] = stripeAccountId;
        
        emit StripeAccountLinked(msg.sender, stripeAccountId);
    }
    
    /**
     * @dev Verify Stripe account (admin only)
     * @param userAddress User's Ethereum address
     * @param isVerified Verification status
     * @param riskScore Risk score (0-1000)
     */
    function verifyStripeAccount(
        address userAddress,
        bool isVerified,
        uint256 riskScore
    ) external onlyOwner {
        require(bytes(addressToStripeId[userAddress]).length > 0, "No linked Stripe account");
        require(riskScore <= 1000, "Invalid risk score");
        
        StripeAccountData storage account = stripeAccounts[userAddress];
        account.isVerified = isVerified;
        account.riskScore = riskScore;
        
        emit AccountVerificationStatusChanged(userAddress, isVerified);
    }
    
    // ============ Transaction Processing ============
    
    /**
     * @dev Process Stripe webhook for transaction events
     * @param eventId Stripe event ID
     * @param eventType Stripe event type
     * @param eventData Encoded event data
     * @param signature Webhook signature
     */
    function processStripeWebhook(
        string calldata eventId,
        string calldata eventType,
        bytes calldata eventData,
        bytes calldata signature
    ) external nonReentrant whenNotPaused {
        require(bytes(eventId).length > 0, "Invalid event ID");
        require(!processedEvents[eventId], "Event already processed");
        
        // Verify webhook signature
        bytes32 messageHash = keccak256(abi.encodePacked(eventId, eventType, eventData));
        bytes32 ethSignedMessageHash = messageHash.toEthSignedMessageHash();
        address signer = ethSignedMessageHash.recover(signature);
        require(signer == stripeWebhookSigner, "Invalid webhook signature");
        
        // Process based on event type
        bool success = false;
        if (keccak256(bytes(eventType)) == keccak256("payment_intent.succeeded")) {
            success = _processPaymentSuccess(eventData);
        } else if (keccak256(bytes(eventType)) == keccak256("payment_intent.payment_failed")) {
            success = _processPaymentFailure(eventData);
        } else if (keccak256(bytes(eventType)) == keccak256("payout.paid")) {
            success = _processPayoutPaid(eventData);
        } else if (keccak256(bytes(eventType)) == keccak256("account.updated")) {
            success = _processAccountUpdate(eventData);
        } else {
            // Unknown event type, mark as processed but don't fail
            success = true;
        }
        
        if (success) {
            processedEvents[eventId] = true;
        } else {
            // Store for retry
            webhookEvents[eventId] = WebhookData({
                eventId: eventId,
                eventType: eventType,
                eventData: eventData,
                timestamp: block.timestamp,
                signature: signature,
                processed: false,
                retryCount: 1
            });
            pendingWebhooks.push(eventId);
        }
        
        emit WebhookProcessed(eventId, eventType, success);
    }
    
    /**
     * @dev Record fiat transaction manually (for testing or backup)
     * @param transaction Transaction data
     */
    function recordFiatTransaction(
        FiatTransaction calldata transaction
    ) external {
        require(authorizedValidators[msg.sender] || msg.sender == owner(), "Unauthorized");
        require(bytes(transaction.transactionId).length > 0, "Invalid transaction ID");
        
        // Calculate transaction hash
        bytes32 hash = keccak256(abi.encode(
            transaction.transactionId,
            transaction.userAddress,
            transaction.amount,
            transaction.currency,
            transaction.transactionType,
            transaction.timestamp
        ));
        
        require(hash == transaction.hash, "Transaction hash mismatch");
        
        // Store transaction
        fiatTransactions[transaction.transactionId] = transaction;
        userTransactions[transaction.userAddress].push(transaction.transactionId);
        
        // Update user activity summary
        _updateActivitySummary(transaction.userAddress, transaction.amount);
        
        // Update Stripe account data
        StripeAccountData storage account = stripeAccounts[transaction.userAddress];
        account.totalVolume += transaction.amount;
        account.lastActivity = block.timestamp;
        
        // Notify economic engine of fiat activity
        if (transaction.status == TransactionStatus.SUCCEEDED && transaction.amount >= MIN_FIAT_ACTIVITY_FOR_DISCOUNT) {
            economicEngine.recordFiatActivity(
                transaction.userAddress,
                transaction.amount,
                uint8(transaction.transactionType)
            );
        }
        
        emit FiatTransactionProcessed(transaction.transactionId, transaction.userAddress, transaction.amount, transaction.transactionType);
    }
    
    // ============ Backing Validation ============
    
    /**
     * @dev Validate fiat backing with signed data from authorized validator
     * @param newBackingData Backing validation data
     * @param signature Validator signature
     */
    function validateFiatBacking(
        BackingValidationData calldata newBackingData,
        bytes calldata signature
    ) external nonReentrant whenNotPaused {
        require(newBackingData.lastValidation > lastBackingCheck, "Stale backing data");
        require(newBackingData.validationNonce > backingData.validationNonce, "Invalid nonce");
        
        // Verify signature
        bytes32 messageHash = keccak256(abi.encode(
            newBackingData.totalUSDCReserves,
            newBackingData.totalStripeBalance,
            newBackingData.pendingTransactions,
            newBackingData.reservedFunds,
            newBackingData.lastValidation,
            newBackingData.validationNonce
        ));
        bytes32 ethSignedMessageHash = messageHash.toEthSignedMessageHash();
        address signer = ethSignedMessageHash.recover(signature);
        require(authorizedValidators[signer], "Unauthorized validator");
        
        // Calculate available funds
        uint256 availableFunds = newBackingData.totalUSDCReserves + 
                                 newBackingData.totalStripeBalance - 
                                 newBackingData.pendingTransactions - 
                                 newBackingData.reservedFunds;
        
        // Get total supply from economic engine (would need to add this function)
        uint256 requiredBacking = _getTotalGateSupply();
        
        // Check if backing is sufficient (with tolerance)
        uint256 minimumRequired = (requiredBacking * (10000 - BACKING_VALIDATION_TOLERANCE)) / 10000;
        bool isValid = availableFunds >= minimumRequired;
        
        // Update backing data
        backingData.totalUSDCReserves = newBackingData.totalUSDCReserves;
        backingData.totalStripeBalance = newBackingData.totalStripeBalance;
        backingData.pendingTransactions = newBackingData.pendingTransactions;
        backingData.reservedFunds = newBackingData.reservedFunds;
        backingData.availableFunds = availableFunds;
        backingData.lastValidation = newBackingData.lastValidation;
        backingData.isValid = isValid;
        backingData.validationNonce = newBackingData.validationNonce;
        
        lastBackingCheck = block.timestamp;
        
        emit BackingValidated(availableFunds, requiredBacking, isValid);
        
        // Trigger emergency mode if backing is invalid
        if (!isValid && !emergencyMode) {
            _activateEmergencyMode("Insufficient fiat backing detected");
        }
    }
    
    /**
     * @dev Calculate fiat activity discount for user
     * @param userAddress User address
     * @return discountPercentage Discount percentage (0-5000 basis points)
     */
    function calculateFiatDiscount(address userAddress) external returns (uint256 discountPercentage) {
        require(
            msg.sender == address(economicEngine) || msg.sender == owner(),
            "Unauthorized"
        );
        
        FiatActivitySummary storage summary = activitySummaries[userAddress];
        StripeAccountData memory account = stripeAccounts[userAddress];
        
        // Check if user has verified Stripe account
        if (!account.isVerified || account.isBlocked) {
            return 0;
        }
        
        // Update activity summary if needed
        if (block.timestamp > summary.lastUpdate + 1 hours) {
            _updateActivitySummary(userAddress, 0); // Update without adding new transaction
        }
        
        // Calculate discount based on activity levels
        uint256 discount = 0;
        
        // Base discount for having fiat activity
        if (summary.volume24h > 0) {
            discount = 500; // 5% base discount
        }
        
        // Additional discount for high activity
        if (summary.volume24h >= 1000e18) { // $1000+
            discount += 1000; // +10%
        } else if (summary.volume24h >= 500e18) { // $500+
            discount += 500; // +5%
        }
        
        // Frequency bonus
        if (summary.transactionCount24h >= 5) {
            discount += 500; // +5% for frequent usage
        } else if (summary.transactionCount24h >= 3) {
            discount += 250; // +2.5%
        }
        
        // Weekly volume bonus
        if (summary.volume7d >= 5000e18) { // $5000+ weekly
            discount += 1500; // +15%
        } else if (summary.volume7d >= 2000e18) { // $2000+ weekly
            discount += 750; // +7.5%
        }
        
        // Risk adjustment (lower risk = higher discount)
        if (account.riskScore <= 300) { // Low risk
            discount = (discount * 120) / 100; // +20% bonus
        } else if (account.riskScore >= 700) { // High risk
            discount = (discount * 80) / 100; // -20% penalty
        }
        
        // Cap at maximum discount
        discount = discount > MAX_DISCOUNT_PERCENTAGE ? MAX_DISCOUNT_PERCENTAGE : discount;
        
        // Store calculated discount
        fiatDiscountPercentages[userAddress] = discount;
        lastDiscountCalculation[userAddress] = block.timestamp;
        
        emit FiatDiscountCalculated(userAddress, discount);
        return discount;
    }
    
    // ============ Internal Functions ============
    
    function _processPaymentSuccess(bytes calldata eventData) internal returns (bool) {
        // Decode payment success data and process
        // Implementation depends on Stripe webhook format
        return true; // Placeholder
    }
    
    function _processPaymentFailure(bytes calldata eventData) internal returns (bool) {
        // Process payment failure
        return true; // Placeholder
    }
    
    function _processPayoutPaid(bytes calldata eventData) internal returns (bool) {
        // Process payout completion
        return true; // Placeholder
    }
    
    function _processAccountUpdate(bytes calldata eventData) internal returns (bool) {
        // Process Stripe account updates
        return true; // Placeholder
    }
    
    function _updateActivitySummary(address userAddress, uint256 newTransactionAmount) internal {
        FiatActivitySummary storage summary = activitySummaries[userAddress];
        uint256 currentTime = block.timestamp;
        uint256 timeSinceUpdate = currentTime - summary.lastUpdate;
        
        // Apply time decay to existing activity
        if (timeSinceUpdate > 0) {
            // Decay 24h volume
            if (timeSinceUpdate >= 24 hours) {
                summary.volume24h = 0;
                summary.transactionCount24h = 0;
            } else {
                uint256 decayFactor = (24 hours - timeSinceUpdate) * 100 / 24 hours;
                summary.volume24h = (summary.volume24h * decayFactor) / 100;
                summary.transactionCount24h = (summary.transactionCount24h * decayFactor) / 100;
            }
            
            // Decay 7d volume
            if (timeSinceUpdate >= 7 days) {
                summary.volume7d = 0;
            } else {
                uint256 decayFactor = (7 days - timeSinceUpdate) * 100 / 7 days;
                summary.volume7d = (summary.volume7d * decayFactor) / 100;
            }
            
            // Decay 30d volume
            if (timeSinceUpdate >= 30 days) {
                summary.volume30d = 0;
            } else {
                uint256 decayFactor = (30 days - timeSinceUpdate) * 100 / 30 days;
                summary.volume30d = (summary.volume30d * decayFactor) / 100;
            }
        }
        
        // Add new transaction
        if (newTransactionAmount > 0) {
            summary.volume24h += newTransactionAmount;
            summary.volume7d += newTransactionAmount;
            summary.volume30d += newTransactionAmount;
            summary.transactionCount24h++;
        }
        
        // Update average transaction size
        if (summary.transactionCount24h > 0) {
            summary.averageTransactionSize = summary.volume24h / summary.transactionCount24h;
        }
        
        summary.lastUpdate = currentTime;
    }
    
    function _getTotalGateSupply() internal view returns (uint256) {
        // This would query the main Caesar Token contract for total supply
        // Placeholder implementation
        return 1000000 * 1e18;
    }
    
    function _activateEmergencyMode(string memory reason) internal {
        emergencyMode = true;
        emergencyActivatedAt = block.timestamp;
        economicEngine.activateEmergencyMode(reason);
        emit EmergencyModeActivated(msg.sender, reason);
    }
    
    // ============ Administrative Functions ============
    
    /**
     * @dev Update Stripe configuration
     * @param _publishableKey New Stripe publishable key
     * @param _webhookSigner New webhook signer address
     */
    function updateStripeConfiguration(
        string calldata _publishableKey,
        address _webhookSigner
    ) external onlyOwner {
        stripePublishableKey = _publishableKey;
        stripeWebhookSigner = _webhookSigner;
        emit StripeConfigurationUpdated(_publishableKey);
    }
    
    /**
     * @dev Add authorized backing validator
     * @param validator Validator address
     */
    function addAuthorizedValidator(address validator) external onlyOwner {
        authorizedValidators[validator] = true;
    }
    
    /**
     * @dev Remove authorized backing validator
     * @param validator Validator address
     */
    function removeAuthorizedValidator(address validator) external onlyOwner {
        authorizedValidators[validator] = false;
    }
    
    /**
     * @dev Add emergency operator
     * @param operator Operator address
     */
    function addEmergencyOperator(address operator) external onlyOwner {
        emergencyOperators[operator] = true;
    }
    
    /**
     * @dev Block Stripe account
     * @param userAddress User address to block
     * @param isBlocked Block status
     */
    function blockStripeAccount(address userAddress, bool isBlocked) external onlyOwner {
        require(bytes(addressToStripeId[userAddress]).length > 0, "No linked account");
        stripeAccounts[userAddress].isBlocked = isBlocked;
    }
    
    // ============ View Functions ============
    
    function getStripeAccountData(address userAddress) external view returns (StripeAccountData memory) {
        return stripeAccounts[userAddress];
    }
    
    function getFiatTransaction(string calldata transactionId) external view returns (FiatTransaction memory) {
        return fiatTransactions[transactionId];
    }
    
    function getUserTransactions(address userAddress) external view returns (string[] memory) {
        return userTransactions[userAddress];
    }
    
    function getActivitySummary(address userAddress) external view returns (FiatActivitySummary memory) {
        return activitySummaries[userAddress];
    }
    
    function getBackingData() external view returns (BackingValidationData memory) {
        return backingData;
    }
    
    function getFiatDiscount(address userAddress) external view returns (uint256) {
        return fiatDiscountPercentages[userAddress];
    }
    
    function isBackingValid() external view returns (bool) {
        return backingData.isValid && (block.timestamp - backingData.lastValidation <= 1 hours);
    }
    
    // ============ Emergency Functions ============
    
    /**
     * @dev Manual emergency mode activation
     * @param reason Emergency reason
     */
    function activateEmergencyMode(string calldata reason) external {
        require(emergencyOperators[msg.sender] || msg.sender == owner(), "Unauthorized");
        _activateEmergencyMode(reason);
    }
    
    /**
     * @dev Deactivate emergency mode
     */
    function deactivateEmergencyMode() external onlyOwner {
        emergencyMode = false;
        emergencyActivatedAt = 0;
    }
    
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
    
    // ============ Mock Methods for Testing ============
    
    /**
     * @dev Set CSR token address (for testing)
     * @param _csrToken CSR token address
     */
    function setCsrToken(address _csrToken) external onlyOwner {
        require(_csrToken != address(0), "Invalid token address");
        csrToken = IERC20(_csrToken);
    }
    
    /**
     * @dev Redeem CSR tokens for fiat (stub for testing)
     * @param amount Amount of CSR to redeem
     */
    function redeemForFiat(uint256 amount) external whenNotPaused nonReentrant {
        require(amount > 0, "Amount must be positive");
        require(address(csrToken) != address(0), "CSR token not set");
        
        // Transfer CSR tokens from user to this contract (lock them)
        require(csrToken.transferFrom(msg.sender, address(this), amount), "Transfer failed");
        
        // Record the redemption request
        string memory txId = string(abi.encodePacked("REDEEM_", block.timestamp, "_", msg.sender));
        
        // Store transaction directly to avoid stack issues
        fiatTransactions[txId].transactionId = txId;
        fiatTransactions[txId].stripePaymentId = "";
        fiatTransactions[txId].userAddress = msg.sender;
        fiatTransactions[txId].amount = amount;
        fiatTransactions[txId].currency = "USD";
        fiatTransactions[txId].transactionType = TransactionType.REDEMPTION;
        fiatTransactions[txId].status = TransactionStatus.PENDING;
        fiatTransactions[txId].timestamp = block.timestamp;
        fiatTransactions[txId].description = "CSR redemption";
        fiatTransactions[txId].hash = keccak256(abi.encodePacked(txId, msg.sender, amount));
        
        userTransactions[msg.sender].push(txId);
        
        // Update activity summary
        activitySummaries[msg.sender].volume24h += amount;
        activitySummaries[msg.sender].transactionCount24h++;
        activitySummaries[msg.sender].lastUpdate = block.timestamp;
        
        emit FiatTransactionProcessed(txId, msg.sender, amount, TransactionType.REDEMPTION);
    }
    
    /**
     * @dev Get user redemption count (stub for testing)
     * @param userAddress User address to check
     * @return count Number of redemptions
     */
    function getUserRedemptionCount(address userAddress) external view returns (uint256 count) {
        string[] memory transactions = userTransactions[userAddress];
        for (uint256 i = 0; i < transactions.length; i++) {
            if (fiatTransactions[transactions[i]].transactionType == TransactionType.REDEMPTION) {
                count++;
            }
        }
        return count;
    }
    
    /**
     * @dev Purchase CSR tokens with fiat (stub for testing)
     * @param amount Amount to purchase
     * @param paymentMethodId Payment method identifier
     */
    function purchaseWithFiat(uint256 amount, string calldata paymentMethodId) external whenNotPaused nonReentrant {
        require(amount > 0, "Amount must be positive");
        require(bytes(paymentMethodId).length > 0, "Payment method required");
        
        string memory txId = string(abi.encodePacked("PURCHASE_", block.timestamp, "_", msg.sender));
        
        // Store transaction directly to avoid stack issues
        fiatTransactions[txId].transactionId = txId;
        fiatTransactions[txId].stripePaymentId = paymentMethodId;
        fiatTransactions[txId].userAddress = msg.sender;
        fiatTransactions[txId].amount = amount;
        fiatTransactions[txId].currency = "USD";
        fiatTransactions[txId].transactionType = TransactionType.PURCHASE;
        fiatTransactions[txId].status = TransactionStatus.PENDING;
        fiatTransactions[txId].timestamp = block.timestamp;
        fiatTransactions[txId].description = "CSR purchase";
        fiatTransactions[txId].hash = keccak256(abi.encodePacked(txId, msg.sender, amount));
        
        userTransactions[msg.sender].push(txId);
        
        emit FiatTransactionProcessed(txId, msg.sender, amount, TransactionType.PURCHASE);
    }
}