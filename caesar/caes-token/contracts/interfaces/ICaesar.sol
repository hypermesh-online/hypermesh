// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import { IOFT, SendParam, MessagingFee } from "@layerzerolabs/oft-evm/contracts/interfaces/IOFT.sol";

/**
 * @title ICaesar
 * @dev Interface for Caesar with demurrage and anti-speculation mechanisms
 */
interface ICaesar is IOFT {
    // Events
    event DemurrageApplied(address indexed account, uint256 amount, uint256 timestamp);
    event AntiSpeculationPenalty(address indexed account, uint256 penalty, uint256 timestamp);
    event TensorEpochUpdate(uint256 indexed epoch, uint256 timestamp);
    event RebaseOccurred(uint256 indexed epoch, uint256 rebaseRatio, uint256 newTotalSupply);
    event StabilityPoolContribution(uint256 amount, uint256 timestamp);
    event ParticipationScoreUpdate(address indexed account, uint256 score);
    
    // Structs
    struct AccountInfo {
        uint256 balance;
        uint256 lastActivity;
        uint256 participationScore;
        uint256 holdingStartTime;
        bool isExempt;
    }
    
    struct DemurrageConfig {
        uint256 baseRate;       // Base demurrage rate (basis points)
        uint256 maxRate;        // Maximum demurrage rate (basis points)
        uint256 decayInterval;  // Time interval for decay calculation
        uint256 stabilityThreshold; // Price stability threshold
    }
    
    struct AntiSpeculationConfig {
        uint256 maxHoldingPeriod;  // Maximum beneficial holding period
        uint256 penaltyRate;       // Penalty rate for excessive holding
        uint256 rapidTradePenalty; // Penalty for rapid trading
        uint256 minTransactionGap; // Minimum time between transactions
    }
    
    // Demurrage Functions
    function applyDemurrage(address account) external returns (uint256 appliedAmount);
    function calculateDemurrage(address account) external view returns (uint256 demurrageAmount);
    function getCurrentDecayRate() external view returns (uint256 decayRate);
    function setDemurrageConfig(DemurrageConfig calldata config) external;
    
    // Anti-Speculation Functions
    function calculateSpeculationPenalty(address account) external view returns (uint256 penalty);
    function updateParticipationScore(address account, uint256 transactionVolume) external;
    function setAntiSpeculationConfig(AntiSpeculationConfig calldata config) external;
    function isAccountExempt(address account) external view returns (bool exempt);
    
    // Tensor Epoch Management
    function getCurrentEpoch() external view returns (uint256 epoch);
    function advanceEpoch() external;
    function getEpochDuration() external view returns (uint256 duration);
    
    // Rebase Functions
    function rebase() external returns (uint256 newSupply);
    function getRebaseRatio() external view returns (uint256 ratio);
    function shouldRebase() external view returns (bool shouldRebase);
    
    // Stability Pool Functions
    function contributeToStabilityPool(uint256 amount) external;
    function getStabilityPoolBalance() external view returns (uint256 balance);
    function withdrawFromStabilityPool(uint256 amount) external;
    
    // Network Health
    function getNetworkHealthIndex() external view returns (uint256 healthIndex);
    function getLiquidityRatio() external view returns (uint256 liquidityRatio);
    function getActiveParticipants() external view returns (uint256 activeCount);
    
    // Account Management
    function getAccountInfo(address account) external view returns (AccountInfo memory info);
    function setAccountExemption(address account, bool exempt) external;
    function updateAccountActivity(address account) external;
    
    // Cross-chain specific
    function bridgeWithDecay(
        SendParam calldata sendParam,
        MessagingFee calldata fee,
        address refundAddress
    ) external payable returns (MessagingFee memory messagingFee);
    
    function quoteBridgeWithDecay(
        SendParam calldata sendParam,
        bool payInLzToken
    ) external view returns (MessagingFee memory messagingFee);
}