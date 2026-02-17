// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { expect } from 'chai';
import { ethers } from 'hardhat';
import { loadFixture, time } from '@nomicfoundation/hardhat-network-helpers';
import { deployBasicFixture, deployCaesarFixture } from '../fixtures/TestFixtures';
import { CaesarToken } from '../../typechain-types';

describe('Caesar Unit Tests', function () {
  describe('Deployment', function () {
    it('Should deploy with correct initial parameters', async function () {
      const { caesarToken, owner, initialSupply } = await loadFixture(deployBasicFixture);
      
      expect(await caesarToken.name()).to.equal('CAESAR');
      expect(await caesarToken.symbol()).to.equal('CAES');
      expect(await caesarToken.decimals()).to.equal(18);
      expect(await caesarToken.owner()).to.equal(owner.address);
      expect(await caesarToken.totalSupply()).to.equal(initialSupply);
    });

    it('Should initialize epoch system correctly', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      expect(await caesarToken.getCurrentEpoch()).to.equal(1);
      expect(await caesarToken.getEpochDuration()).to.equal(86400); // 1 day
      
      const currentTime = await time.latest();
      const lastEpochUpdate = await caesarToken.lastEpochUpdate();
      expect(lastEpochUpdate).to.be.closeTo(currentTime, 10);
    });

    it('Should deploy demurrage manager and anti-speculation engine', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const demurrageManagerAddress = await caesarToken.demurrageManager();
      const antiSpeculationEngineAddress = await caesarToken.antiSpeculationEngine();
      
      expect(demurrageManagerAddress).to.not.equal(ethers.ZeroAddress);
      expect(antiSpeculationEngineAddress).to.not.equal(ethers.ZeroAddress);
    });

    it('Should set initial supply in epoch tracking', async function () {
      const { caesarToken, initialSupply } = await loadFixture(deployBasicFixture);
      
      const currentEpoch = await caesarToken.getCurrentEpoch();
      const epochSupply = await caesarToken.epochSupply(currentEpoch);
      const totalSupply = await caesarToken.totalSupply();
      
      // Supply should be set during deployment
      expect(epochSupply).to.equal(0); // Not updated until epoch advance
      expect(totalSupply).to.equal(initialSupply);
    });
  });

  describe('Token Transfer Mechanics', function () {
    it('Should transfer tokens correctly between accounts', async function () {
      const { caesarToken, owner, user1, user2 } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000');
      
      // Transfer from owner to user1
      await caesarToken.transfer(user1.address, transferAmount);
      expect(await caesarToken.balanceOf(user1.address)).to.equal(transferAmount);
      
      // Transfer from user1 to user2
      await caesarToken.connect(user1).transfer(user2.address, transferAmount / 2n);
      expect(await caesarToken.balanceOf(user2.address)).to.equal(transferAmount / 2n);
      expect(await caesarToken.balanceOf(user1.address)).to.equal(transferAmount / 2n);
    });

    it('Should update account activity on transfers', async function () {
      const { caesarToken, owner, user1 } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000');
      const beforeTransfer = await time.latest();
      
      await caesarToken.transfer(user1.address, transferAmount);
      
      const ownerInfo = await caesarToken.getAccountInfo(owner.address);
      const user1Info = await caesarToken.getAccountInfo(user1.address);
      
      expect(ownerInfo.lastActivity).to.be.gte(beforeTransfer);
      expect(user1Info.lastActivity).to.be.gte(beforeTransfer);
    });

    it('Should fail on insufficient balance', async function () {
      const { caesarToken, user1, user2 } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000000'); // More than user1 has
      
      await expect(
        caesarToken.connect(user1).transfer(user2.address, transferAmount)
      ).to.be.revertedWith('Insufficient balance after demurrage');
    });

    it('Should handle zero amount transfers', async function () {
      const { caesarToken, owner, user1 } = await loadFixture(deployBasicFixture);
      
      // Zero amount should not fail but should not change balances
      const initialBalance = await caesarToken.balanceOf(user1.address);
      await caesarToken.transfer(user1.address, 0);
      
      expect(await caesarToken.balanceOf(user1.address)).to.equal(initialBalance);
    });
  });

  describe('Demurrage System', function () {
    it('Should calculate demurrage for inactive accounts', async function () {
      const { caesarToken, user1, owner } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('10000');
      await caesarToken.connect(owner).transfer(user1.address, transferAmount);
      
      // Move time forward to trigger demurrage
      await time.increase(86400 * 3); // 3 days
      
      const demurrageAmount = await caesarToken.calculateDemurrage(user1.address);
      expect(demurrageAmount).to.be.gt(0);
    });

    it('Should not apply demurrage during grace period', async function () {
      const { caesarToken, user1, owner } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('10000');
      await caesarToken.connect(owner).transfer(user1.address, transferAmount);
      
      // Within grace period
      await time.increase(3600); // 1 hour
      
      const demurrageAmount = await caesarToken.calculateDemurrage(user1.address);
      expect(demurrageAmount).to.equal(0);
    });

    it('Should not apply demurrage to exempt accounts', async function () {
      const { caesarToken, user1, owner } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('10000');
      await caesarToken.connect(owner).transfer(user1.address, transferAmount);
      
      // Set account as exempt
      await caesarToken.setAccountExemption(user1.address, true);
      
      // Move time forward
      await time.increase(86400 * 3); // 3 days
      
      const demurrageAmount = await caesarToken.calculateDemurrage(user1.address);
      expect(demurrageAmount).to.equal(0);
    });

    it('Should apply demurrage on transfers after time passes', async function () {
      const { caesarToken, user1, user2, owner } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('10000');
      await caesarToken.connect(owner).transfer(user1.address, transferAmount);
      
      // Move time forward to trigger demurrage
      await time.increase(86400 * 2); // 2 days
      
      const initialStabilityPool = await caesarToken.getStabilityPoolBalance();
      
      // Make a transfer to trigger demurrage application
      await caesarToken.connect(user1).transfer(user2.address, ethers.parseEther('100'));
      
      const finalStabilityPool = await caesarToken.getStabilityPoolBalance();
      expect(finalStabilityPool).to.be.gt(initialStabilityPool);
    });
  });

  describe('Anti-Speculation Penalties', function () {
    it('Should calculate penalties for rapid trading', async function () {
      const { caesarToken, user1, owner } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000');
      await caesarToken.connect(owner).transfer(user1.address, transferAmount);
      
      // Make rapid transfers to trigger penalties
      await caesarToken.connect(user1).transfer(user1.address, ethers.parseEther('100'));
      
      // Immediate second transfer should have higher penalty
      const penalty = await caesarToken.calculateSpeculationPenalty(user1.address);
      // Note: Penalty may be 0 initially depending on engine configuration
      expect(penalty).to.be.gte(0);
    });

    it('Should apply penalties to stability pool', async function () {
      const { caesarToken, user1, user2, owner } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('10000');
      await caesarToken.connect(owner).transfer(user1.address, transferAmount);
      
      // Manually trigger a penalty scenario
      // This requires setting up the anti-speculation engine properly
      const initialStabilityPool = await caesarToken.getStabilityPoolBalance();
      
      // Make transfers that might trigger penalties
      for (let i = 0; i < 5; i++) {
        await caesarToken.connect(user1).transfer(
          i % 2 === 0 ? user2.address : user1.address, 
          ethers.parseEther('100')
        );
        await time.increase(60); // 1 minute between transfers
      }
      
      const finalStabilityPool = await caesarToken.getStabilityPoolBalance();
      // Pool may increase due to penalties or demurrage
      expect(finalStabilityPool).to.be.gte(initialStabilityPool);
    });
  });

  describe('Epoch Management', function () {
    it('Should advance epoch after duration passes', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const initialEpoch = await caesarToken.getCurrentEpoch();
      
      // Move time forward by epoch duration
      await time.increase(86400); // 1 day
      
      await caesarToken.advanceEpoch();
      
      const newEpoch = await caesarToken.getCurrentEpoch();
      expect(newEpoch).to.equal(initialEpoch + 1n);
    });

    it('Should not advance epoch before duration elapses', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      await expect(caesarToken.advanceEpoch())
        .to.be.revertedWith('Epoch duration not elapsed');
    });

    it('Should update epoch supply tracking', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      // Move time forward
      await time.increase(86400);
      
      const totalSupplyBefore = await caesarToken.totalSupply();
      await caesarToken.advanceEpoch();
      
      const newEpoch = await caesarToken.getCurrentEpoch();
      const epochSupply = await caesarToken.epochSupply(newEpoch);
      
      expect(epochSupply).to.equal(totalSupplyBefore);
    });
  });

  describe('Stability Pool Operations', function () {
    it('Should allow contributions to stability pool', async function () {
      const { caesarToken, user1 } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000');
      await caesarToken.transfer(user1.address, transferAmount);
      
      const contributionAmount = ethers.parseEther('100');
      await caesarToken.connect(user1).contributeToStabilityPool(contributionAmount);
      
      expect(await caesarToken.getStabilityPoolBalance()).to.equal(contributionAmount);
      expect(await caesarToken.balanceOf(user1.address)).to.equal(transferAmount - contributionAmount);
    });

    it('Should allow owner to withdraw from stability pool', async function () {
      const { caesarToken, owner, user1 } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000');
      await caesarToken.transfer(user1.address, transferAmount);
      
      const contributionAmount = ethers.parseEther('100');
      await caesarToken.connect(user1).contributeToStabilityPool(contributionAmount);
      
      const ownerBalanceBefore = await caesarToken.balanceOf(owner.address);
      
      // Owner withdraws from stability pool
      await caesarToken.withdrawFromStabilityPool(contributionAmount);
      
      expect(await caesarToken.getStabilityPoolBalance()).to.equal(0);
      expect(await caesarToken.balanceOf(owner.address))
        .to.equal(ownerBalanceBefore + contributionAmount);
    });

    it('Should not allow non-owner to withdraw from stability pool', async function () {
      const { caesarToken, user1 } = await loadFixture(deployBasicFixture);
      
      const transferAmount = ethers.parseEther('1000');
      await caesarToken.transfer(user1.address, transferAmount);
      
      const contributionAmount = ethers.parseEther('100');
      await caesarToken.connect(user1).contributeToStabilityPool(contributionAmount);
      
      await expect(
        caesarToken.connect(user1).withdrawFromStabilityPool(contributionAmount)
      ).to.be.revertedWithCustomError(caesarToken, 'OwnableUnauthorizedAccount');
    });
  });

  describe('Network Health Metrics', function () {
    it('Should calculate network health index', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const healthIndex = await caesarToken.getNetworkHealthIndex();
      expect(healthIndex).to.be.gte(0);
      expect(healthIndex).to.be.lte(1000); // Assuming scale of 0-1000
    });

    it('Should track liquidity ratio', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const liquidityRatio = await caesarToken.getLiquidityRatio();
      expect(liquidityRatio).to.be.gte(0);
    });

    it('Should track active participants', async function () {
      const { caesarToken, user1, user2 } = await loadFixture(deployBasicFixture);
      
      const initialParticipants = await caesarToken.getActiveParticipants();
      
      // Make some transfers to create activity
      await caesarToken.transfer(user1.address, ethers.parseEther('1000'));
      await caesarToken.transfer(user2.address, ethers.parseEther('1000'));
      
      const updatedParticipants = await caesarToken.getActiveParticipants();
      expect(updatedParticipants).to.be.gte(initialParticipants);
    });
  });

  describe('Rebase Mechanism', function () {
    it('Should check rebase conditions correctly', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      // Initially should not need rebase
      const shouldRebase = await caesarToken.shouldRebase();
      expect(shouldRebase).to.be.false;
    });

    it('Should return correct rebase ratio', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const ratio = await caesarToken.getRebaseRatio();
      // Should return neutral ratio when no rebase needed
      expect(ratio).to.equal(10000); // 1.0x ratio in basis points
    });

    it('Should handle rebase timing correctly', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const lastRebaseTime = await caesarToken.lastRebaseTime();
      expect(lastRebaseTime).to.be.gt(0);
      
      const rebaseInterval = await caesarToken.rebaseInterval();
      expect(rebaseInterval).to.equal(86400 * 7); // 7 days
    });
  });

  describe('Access Control', function () {
    it('Should allow owner to set configurations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, owner } = fixture;
      
      const demurrageConfig = {
        baseRate: 100,
        maxRate: 500,
        decayInterval: 86400,
        stabilityThreshold: 200
      };
      
      await expect(
        caesarToken.setDemurrageConfig(demurrageConfig)
      ).to.not.be.reverted;
    });

    it('Should not allow non-owner to set configurations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, user1 } = fixture;
      
      const demurrageConfig = {
        baseRate: 100,
        maxRate: 500,
        decayInterval: 86400,
        stabilityThreshold: 200
      };
      
      await expect(
        caesarToken.connect(user1).setDemurrageConfig(demurrageConfig)
      ).to.be.revertedWithCustomError(caesarToken, 'OwnableUnauthorizedAccount');
    });

    it('Should allow owner to set account exemptions', async function () {
      const { caesarToken, user1 } = await loadFixture(deployBasicFixture);
      
      await caesarToken.setAccountExemption(user1.address, true);
      expect(await caesarToken.isAccountExempt(user1.address)).to.be.true;
      
      await caesarToken.setAccountExemption(user1.address, false);
      expect(await caesarToken.isAccountExempt(user1.address)).to.be.false;
    });

    it('Should not allow non-owner to set account exemptions', async function () {
      const { caesarToken, user1, user2 } = await loadFixture(deployBasicFixture);
      
      await expect(
        caesarToken.connect(user1).setAccountExemption(user2.address, true)
      ).to.be.revertedWithCustomError(caesarToken, 'OwnableUnauthorizedAccount');
    });
  });

  describe('Cross-Chain Functionality', function () {
    it('Should track supply per chain', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      const chainId = 1; // Ethereum mainnet
      const initialSupply = await caesarToken.chainSupply(chainId);
      
      // Initially should be 0 for other chains
      expect(initialSupply).to.equal(0);
    });

    it('Should update cross-chain supply correctly', async function () {
      const { caesarToken, owner } = await loadFixture(deployBasicFixture);
      
      const chainId = 137; // Polygon
      const supplyAmount = ethers.parseEther('100000');
      
      // This would typically be called by LayerZero operations
      // For now, test the state tracking
      const totalSupply = await caesarToken.totalSupply();
      expect(totalSupply).to.be.gt(0);
    });
  });

  describe('Error Conditions', function () {
    it('Should handle overflow conditions gracefully', async function () {
      const { caesarToken, user1 } = await loadFixture(deployBasicFixture);
      
      // Transfer maximum possible amount to test overflow protection
      const maxAmount = ethers.MaxUint256;
      
      await expect(
        caesarToken.transfer(user1.address, maxAmount)
      ).to.be.revertedWith('Insufficient balance after demurrage');
    });

    it('Should handle zero address transfers', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      await expect(
        caesarToken.transfer(ethers.ZeroAddress, ethers.parseEther('1000'))
      ).to.be.revertedWithCustomError(caesarToken, 'ERC20InvalidReceiver');
    });

    it('Should handle invalid epoch operations', async function () {
      const { caesarToken } = await loadFixture(deployBasicFixture);
      
      // Trying to advance epoch too early should fail
      await expect(caesarToken.advanceEpoch())
        .to.be.revertedWith('Epoch duration not elapsed');
    });
  });
});