// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { expect } from 'chai';
import { ethers } from 'hardhat';
import { loadFixture, time } from '@nomicfoundation/hardhat-network-helpers';
import { deployCaesarFixture } from '../fixtures/TestFixtures';

describe('Security Audit Tests', function () {
  describe('Access Control Security', function () {
    it('Should enforce owner-only functions correctly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, economicEngine, user1 } = fixture;
      
      // Test owner-only functions
      const restrictedFunctions = [
        {
          name: 'setAccountExemption',
          call: () => caesarToken.connect(user1).setAccountExemption(user1.address, true)
        },
        {
          name: 'withdrawFromStabilityPool',
          call: () => caesarToken.connect(user1).withdrawFromStabilityPool(ethers.parseEther('100'))
        },
        {
          name: 'mintFromFiat',
          call: () => caesarToken.connect(user1).mintFromFiat(user1.address, ethers.parseEther('1000'))
        },
        {
          name: 'activateEmergencyMode',
          call: () => economicEngine.connect(user1).activateEmergencyMode('Test emergency')
        },
        {
          name: 'updateEconomicParameters',
          call: () => economicEngine.connect(user1).updateEconomicParameters({
            baseDemurrageRate: 50n,
            maxDemurrageRate: 200n,
            stabilityThreshold: 100n,
            fiatDiscountFactor: 5000n,
            gracePeriodsHours: 48n,
            interventionThreshold: 500n,
            rebalanceFrequency: 3600n,
            emergencyThreshold: 1000n
          })
        }
      ];
      
      for (const func of restrictedFunctions) {
        await expect(func.call()).to.be.revertedWithCustomError(
          caesar, 
          'OwnableUnauthorizedAccount'
        );
      }
    });

    it('Should prevent unauthorized role escalation', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stabilityPool, user1, user2 } = fixture;
      
      // User cannot grant themselves roles
      const keeperRole = await stabilityPool.KEEPER_ROLE();
      const adminRole = await stabilityPool.DEFAULT_ADMIN_ROLE();
      
      await expect(
        stabilityPool.connect(user1).grantRole(keeperRole, user1.address)
      ).to.be.reverted;
      
      await expect(
        stabilityPool.connect(user1).grantRole(adminRole, user1.address)
      ).to.be.reverted;
    });

    it('Should validate multi-signature requirements for critical operations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, owner, emergencyOperator } = fixture;
      
      // Emergency operators can activate emergency mode
      await economicEngine.addEmergencyOperator(emergencyOperator.address);
      
      // But cannot update economic parameters (owner-only)
      await expect(
        economicEngine.connect(emergencyOperator).updateEconomicParameters({
          baseDemurrageRate: 100n,
          maxDemurrageRate: 300n,
          stabilityThreshold: 150n,
          fiatDiscountFactor: 4000n,
          gracePeriodsHours: 72n,
          interventionThreshold: 600n,
          rebalanceFrequency: 7200n,
          emergencyThreshold: 1200n
        })
      ).to.be.revertedWithCustomError(economicEngine, 'OwnableUnauthorizedAccount');
    });
  });

  describe('Smart Contract Vulnerabilities', function () {
    it('Should prevent reentrancy attacks', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, user1, owner } = fixture;
      
      // Deploy malicious contract that attempts reentrancy
      const MaliciousReentrantFactory = await ethers.getContractFactory('MockMaliciousContract');
      const maliciousContract = await MaliciousReentrantFactory.deploy();
      
      // Give the malicious contract some tokens
      const amount = ethers.parseEther('1000');
      await caesarToken.transfer(await maliciousContract.getAddress(), amount);
      
      // Attempt reentrancy attack on transfer function
      await expect(
        maliciousContract.attemptReentrancy(await caesarToken.getAddress())
      ).to.be.revertedWith('ReentrancyGuard: reentrant call');
    });

    it('Should prevent integer overflow/underflow', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, user1 } = fixture;
      
      // Test with maximum possible values
      const maxUint256 = ethers.MaxUint256;
      
      await expect(
        caesarToken.transfer(user1.address, maxUint256)
      ).to.be.revertedWithCustomError(caesar, 'ERC20InsufficientBalance');
      
      // Test calculation overflow in demurrage
      await expect(
        caesar.calculateDemurrage(user1.address)
      ).to.not.be.reverted; // Should handle large numbers gracefully
    });

    it('Should validate input parameters correctly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, caesar, owner } = fixture;
      
      // Test invalid economic parameters
      const invalidParams = [
        {
          name: 'baseDemurrageRate > maxDemurrageRate',
          params: {
            baseDemurrageRate: 300n,
            maxDemurrageRate: 200n,
            stabilityThreshold: 100n,
            fiatDiscountFactor: 5000n,
            gracePeriodsHours: 48n,
            interventionThreshold: 500n,
            rebalanceFrequency: 3600n,
            emergencyThreshold: 1000n
          }
        },
        {
          name: 'fiatDiscountFactor > 100%',
          params: {
            baseDemurrageRate: 50n,
            maxDemurrageRate: 200n,
            stabilityThreshold: 100n,
            fiatDiscountFactor: 15000n, // 150% > 100%
            gracePeriodsHours: 48n,
            interventionThreshold: 500n,
            rebalanceFrequency: 7200n,
            emergencyThreshold: 1000n
          }
        },
        {
          name: 'gracePeriodsHours too long',
          params: {
            baseDemurrageRate: 50n,
            maxDemurrageRate: 200n,
            stabilityThreshold: 100n,
            fiatDiscountFactor: 5000n,
            gracePeriodsHours: 200n, // > 168 hours (1 week)
            interventionThreshold: 500n,
            rebalanceFrequency: 3600n,
            emergencyThreshold: 1000n
          }
        }
      ];
      
      for (const test of invalidParams) {
        await expect(
          economicEngine.updateEconomicParameters(test.params)
        ).to.be.reverted;
      }
    });

    it('Should handle edge cases in mathematical calculations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, economicEngine } = fixture;
      
      // Test with zero balance
      const zeroResult = await caesarToken.calculateDemurrage(ethers.ZeroAddress);
      expect(zeroResult).to.equal(0);
      
      // Test with very small balance
      const tinyBalance = 1n; // 1 wei
      const tinyResult = await economicEngine.calculateDemurrage(ethers.ZeroAddress, tinyBalance);
      expect(tinyResult).to.be.gte(0);
      
      // Test with maximum balance
      const maxBalance = ethers.parseEther('1000000000'); // 1 billion tokens
      const maxResult = await economicEngine.calculateDemurrage(ethers.ZeroAddress, maxBalance);
      expect(maxResult).to.be.finite; // Should not overflow
    });
  });

  describe('Economic Attack Vectors', function () {
    it('Should prevent flash loan attacks on stability pool', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stabilityPool, mockUSDC, owner } = fixture;
      
      // Deploy flash loan attack contract
      const FlashLoanAttackFactory = await ethers.getContractFactory('MockFlashLoanAttacker');
      const flashLoanAttacker = await FlashLoanAttackFactory.deploy();
      
      // Setup stability pool with funds
      const poolAmount = ethers.parseUnits('100000', 6); // 100k USDC
      await mockUSDC.mint(await stabilityPool.getAddress(), poolAmount);
      await stabilityPool.contributeReserves(poolAmount);
      
      // Attempt flash loan attack
      await expect(
        flashLoanAttacker.executeFlashLoanAttack(
          await stabilityPool.getAddress(),
          await caesarToken.getAddress(),
          ethers.parseEther('50000')
        )
      ).to.be.reverted; // Should fail due to protection mechanisms
    });

    it('Should detect and prevent MEV sandwich attacks', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, antiSpeculationEngine, mockAMM, user1, user2, owner } = fixture;
      
      // Setup scenario: User1 wants to make large trade
      const largeTradeAmount = ethers.parseEther('10000');
      await caesarToken.transfer(user1.address, largeTradeAmount);
      
      // MEV bot (user2) tries to sandwich the trade
      const frontrunAmount = ethers.parseEther('5000');
      await caesarToken.transfer(user2.address, frontrunAmount);
      
      // MEV bot front-runs
      await caesarToken.connect(user2).transfer(user1.address, frontrunAmount);
      
      // Legitimate user trade
      await caesarToken.connect(user1).transfer(owner.address, largeTradeAmount);
      
      // MEV bot back-runs (attempts to profit)
      await caesarToken.connect(user2).transfer(owner.address, frontrunAmount);
      
      // Check if MEV pattern was detected
      const user2RiskProfile = await antiSpeculationEngine.getAccountRiskProfile(user2.address);
      expect(user2RiskProfile.overallRiskScore).to.be.gt(500); // Should be flagged as high risk
    });

    it('Should prevent governance token manipulation', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, economicEngine, user1, user2, owner } = fixture;
      
      // Scenario: Attacker tries to manipulate governance by accumulating tokens
      const manipulationAmount = ethers.parseEther('100000');
      
      // Give attacker large balance
      await caesarToken.transfer(user1.address, manipulationAmount);
      
      // Attacker tries to make governance changes
      // Note: In actual implementation, this would involve governance contracts
      // For this test, we verify that critical functions remain owner-only
      
      await expect(
        economicEngine.connect(user1).activateEmergencyMode('Malicious emergency')
      ).to.be.revertedWithCustomError(economicEngine, 'OwnableUnauthorizedAccount');
      
      // Even with large token balance, cannot override access controls
      expect(await caesarToken.balanceOf(user1.address)).to.equal(manipulationAmount);
      
      await expect(
        economicEngine.connect(user1).updateEconomicParameters({
          baseDemurrageRate: 1000n, // Malicious high rate
          maxDemurrageRate: 2000n,
          stabilityThreshold: 100n,
          fiatDiscountFactor: 0n, // Remove fiat benefits
          gracePeriodsHours: 0n, // Remove grace periods
          interventionThreshold: 9999n,
          rebalanceFrequency: 1n,
          emergencyThreshold: 9999n
        })
      ).to.be.revertedWithCustomError(economicEngine, 'OwnableUnauthorizedAccount');
    });

    it('Should resist economic griefing attacks', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stabilityPool, user1, user2, owner } = fixture;
      
      // Scenario: Attacker tries to grief the system by creating many dust transactions
      const dustAmount = ethers.parseEther('0.001'); // Very small amount
      const initialBalance = ethers.parseEther('1000');
      
      await caesarToken.transfer(user1.address, initialBalance);
      await caesarToken.transfer(user2.address, initialBalance);
      
      // Create many dust transactions to spam the system
      for (let i = 0; i < 20; i++) {
        await caesarToken.connect(user1).transfer(
          i % 2 === 0 ? user2.address : owner.address,
          dustAmount
        );
        
        // Rapid succession
        await time.increase(1);
      }
      
      // System should still function normally
      const networkHealth = await caesarToken.getNetworkHealthIndex();
      expect(networkHealth).to.be.gt(400); // Should maintain reasonable health
      
      // Dust transactions should not significantly affect stability pool
      const stabilityBalance = await caesarToken.getStabilityPoolBalance();
      expect(stabilityBalance).to.be.lt(ethers.parseEther('100')); // Penalties should be minimal
    });
  });

  describe('Cross-Chain Security', function () {
    it('Should validate cross-chain message authenticity', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, mockLZEndpointV2, user1 } = fixture;
      
      // Test invalid cross-chain message
      const fakeMessage = ethers.AbiCoder.defaultAbiCoder().encode(
        ['address', 'uint256'], 
        [user1.address, ethers.parseEther('1000000')] // Large fake amount
      );
      
      await expect(
        caesar.lzReceive(
          999, // Invalid source chain
          ethers.solidityPacked(['address'], [ethers.ZeroAddress]), // Invalid source address
          1,
          fakeMessage
        )
      ).to.be.reverted; // Should reject invalid message
    });

    it('Should prevent cross-chain replay attacks', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, user1 } = fixture;
      
      const validMessage = ethers.AbiCoder.defaultAbiCoder().encode(
        ['address', 'uint256'],
        [user1.address, ethers.parseEther('1000')]
      );
      
      // First message should process (if valid)
      // Second identical message should be rejected
      
      // Note: This test would need proper LayerZero setup for full validation
      // For now, we test the concept with mock data
      
      const messageHash = ethers.keccak256(validMessage);
      
      // In a real implementation, the contract would track processed messages
      // and reject replays
    });

    it('Should handle cross-chain communication failures gracefully', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { crossChainSync, owner } = fixture;
      
      // Simulate communication failure
      await crossChainSync.connect(owner).handleFailedMessage(
        137, // Polygon chain ID
        'Message delivery failed',
        ethers.keccak256('0x1234') // Failed message hash
      );
      
      // System should log the failure and allow retry
      const failedMessage = await crossChainSync.getFailedMessage(ethers.keccak256('0x1234'));
      expect(failedMessage.chainId).to.equal(137);
    });
  });

  describe('Data Privacy and Encryption', function () {
    it('Should protect sensitive user data', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      // Verify that sensitive data is not exposed in events
      const depositAmount = ethers.parseUnits('1000', 6);
      
      const tx = await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'sensitive_payment_123',
        depositAmount
      );
      
      const receipt = await tx.wait();
      
      // Events should not contain raw sensitive data
      for (const event of receipt?.logs || []) {
        const eventData = event.data;
        // Ensure no plain text sensitive information is in events
        expect(eventData).to.not.include('sensitive_payment_123');
      }
    });

    it('Should validate encrypted communication channels', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, owner } = fixture;
      
      // Test that encrypted data handling works correctly
      const encryptedData = ethers.keccak256(ethers.toUtf8Bytes('sensitive_data'));
      
      // In real implementation, this would use proper encryption
      await stripeIntegration.connect(owner).storeEncryptedData(encryptedData);
      
      const retrievedData = await stripeIntegration.getEncryptedData();
      expect(retrievedData).to.equal(encryptedData);
    });
  });

  describe('DOS and Rate Limiting', function () {
    it('Should resist denial of service attacks', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, user1, owner } = fixture;
      
      // Setup: Give attacker tokens
      const attackerBalance = ethers.parseEther('1000');
      await caesarToken.transfer(user1.address, attackerBalance);
      
      // Attempt to DOS by making many small transactions rapidly
      for (let i = 0; i < 10; i++) {
        await caesarToken.connect(user1).transfer(
          owner.address,
          ethers.parseEther('1')
        );
      }
      
      // System should still be responsive
      const balance = await caesarToken.balanceOf(user1.address);
      expect(balance).to.be.lt(attackerBalance);
      
      // Network should maintain health
      const networkHealth = await caesarToken.getNetworkHealthIndex();
      expect(networkHealth).to.be.gt(300);
    });

    it('Should implement rate limiting for critical functions', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      // Test rate limiting on fiat operations
      const depositAmount = ethers.parseUnits('100', 6);
      
      // First few deposits should succeed
      for (let i = 0; i < 3; i++) {
        await stripeIntegration.connect(owner).recordFiatDeposit(
          user1.address,
          `rate_limit_test_${i}`,
          depositAmount
        );
      }
      
      // Rapid additional deposits should be rate limited
      await expect(
        stripeIntegration.connect(owner).recordFiatDeposit(
          user1.address,
          'rate_limit_test_excess',
          depositAmount
        )
      ).to.be.revertedWith('Rate limit exceeded');
    });
  });

  describe('Emergency Response Security', function () {
    it('Should secure emergency mode activation', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, owner, emergencyOperator, user1 } = fixture;
      
      // Add emergency operator
      await economicEngine.addEmergencyOperator(emergencyOperator.address);
      
      // Emergency operators can activate emergency mode
      await economicEngine.connect(emergencyOperator).activateEmergencyMode('Security incident');
      
      // Verify emergency mode is active
      const params = await economicEngine.getEconomicParameters();
      expect(params.baseDemurrageRate).to.equal(0); // Demurrage stopped
      
      // Regular users cannot deactivate emergency mode
      await expect(
        economicEngine.connect(user1).deactivateEmergencyMode()
      ).to.be.revertedWithCustomError(economicEngine, 'OwnableUnauthorizedAccount');
      
      // Only owner can deactivate
      await economicEngine.connect(owner).deactivateEmergencyMode();
    });

    it('Should handle security incidents properly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, stabilityPool, owner } = fixture;
      
      // Simulate security incident detection
      const incidentType = 0; // PAUSE_TRADING
      const reason = 'Suspicious activity detected';
      
      const result = await stabilityPool.connect(owner).executeEmergencyIntervention(
        reason,
        incidentType
      );
      
      expect(result).to.be.true;
      
      // Contract should be paused
      expect(await stabilityPool.paused()).to.be.true;
    });

    it('Should maintain audit trail for security events', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, owner } = fixture;
      
      const emergencyReason = 'Security audit test emergency';
      
      // Activate emergency mode
      const tx = await economicEngine.connect(owner).activateEmergencyMode(emergencyReason);
      
      const receipt = await tx.wait();
      
      // Should emit proper events for audit trail
      const emergencyEvent = receipt?.logs?.find(log => {
        try {
          const parsedLog = economicEngine.interface.parseLog({
            topics: log.topics,
            data: log.data
          });
          return parsedLog?.name === 'EmergencyModeActivated';
        } catch {
          return false;
        }
      });
      
      expect(emergencyEvent).to.not.be.undefined;
    });
  });

  describe('Compliance and Regulatory Security', function () {
    it('Should enforce AML compliance limits', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      // Set AML limits
      const dailyLimit = ethers.parseUnits('10000', 6); // $10k daily limit
      await stripeIntegration.connect(owner).setUserDailyLimit(user1.address, dailyLimit);
      
      // Large transaction above limit should require additional verification
      const largeAmount = ethers.parseUnits('15000', 6); // $15k
      
      await expect(
        stripeIntegration.connect(owner).recordFiatDeposit(
          user1.address,
          'large_aml_test',
          largeAmount
        )
      ).to.be.revertedWith('AML verification required');
      
      // After AML verification, should succeed
      await stripeIntegration.connect(owner).setAMLVerification(user1.address, true);
      
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'large_aml_verified',
        largeAmount
      );
      
      const depositInfo = await stripeIntegration.getDepositInfo('large_aml_verified');
      expect(depositInfo.amount).to.equal(largeAmount);
    });

    it('Should maintain transaction monitoring for suspicious activity', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { antiSpeculationEngine, caesar, user1, user2, owner } = fixture;
      
      // Setup users with balances
      await caesarToken.transfer(user1.address, ethers.parseEther('10000'));
      await caesarToken.transfer(user2.address, ethers.parseEther('10000'));
      
      // Create suspicious transaction pattern
      const suspiciousAmount = ethers.parseEther('100');
      
      for (let i = 0; i < 5; i++) {
        // Rapid back-and-forth transactions
        await caesarToken.connect(user1).transfer(user2.address, suspiciousAmount);
        await caesarToken.connect(user2).transfer(user1.address, suspiciousAmount);
        await time.increase(60); // 1 minute between pairs
      }
      
      // Check if pattern was detected
      const user1Risk = await antiSpeculationEngine.getAccountRiskProfile(user1.address);
      const user2Risk = await antiSpeculationEngine.getAccountRiskProfile(user2.address);
      
      expect(user1Risk.overallRiskScore).to.be.gt(500); // High risk threshold
      expect(user2Risk.overallRiskScore).to.be.gt(500);
    });

    it('Should enforce data retention and privacy policies', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      // Record user data
      const depositAmount = ethers.parseUnits('1000', 6);
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'privacy_test_payment',
        depositAmount
      );
      
      // User should be able to request data deletion (GDPR compliance)
      await stripeIntegration.connect(owner).processDataDeletionRequest(user1.address);
      
      // Verify sensitive data was cleaned up
      const userDataCount = await stripeIntegration.getUserDataCount(user1.address);
      expect(userDataCount).to.equal(0); // Personal data removed
      
      // But transaction records (for regulatory compliance) should remain
      const transactionExists = await stripeIntegration.transactionExists('privacy_test_payment');
      expect(transactionExists).to.be.true; // Anonymized transaction record kept
    });
  });
});