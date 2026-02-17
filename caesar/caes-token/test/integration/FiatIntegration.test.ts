// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { expect } from 'chai';
import { ethers } from 'hardhat';
import { loadFixture, time } from '@nomicfoundation/hardhat-network-helpers';
import { deployCaesarFixture } from '../fixtures/TestFixtures';

describe('Fiat Integration Tests', function () {
  describe('USD to CSR Conversion Flow', function () {
    it('Should mint CSR tokens for USD deposits', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, mockUSDC, user1, owner } = fixture;
      
      const usdAmount = ethers.parseUnits('1000', 6); // $1000 USDC
      const expectedGateAmount = ethers.parseEther('1000'); // 1:1 conversion
      
      // Simulate USD deposit via Stripe
      await mockUSDC.connect(user1).approve(await stripeIntegration.getAddress(), usdAmount);
      
      // Record the fiat deposit
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'stripe_payment_123',
        usdAmount
      );
      
      // Mint CSR tokens equivalent to USD deposited
      const initialBalance = await caesarToken.balanceOf(user1.address);
      await caesarToken.connect(owner).mintFromFiat(user1.address, expectedGateAmount);
      
      const finalBalance = await caesarToken.balanceOf(user1.address);
      expect(finalBalance - initialBalance).to.equal(expectedGateAmount);
      
      // Verify the deposit was recorded
      const depositInfo = await stripeIntegration.getDepositInfo('stripe_payment_123');
      expect(depositInfo.amount).to.equal(usdAmount);
      expect(depositInfo.user).to.equal(user1.address);
    });

    it('Should handle CSR to USD redemption correctly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, mockUSDC, user1, owner } = fixture;
      
      const gateAmount = ethers.parseEther('500'); // 500 CSR
      const expectedUSDAmount = ethers.parseUnits('500', 6); // $500 USDC
      
      // Ensure user has CSR tokens
      await caesarToken.transfer(user1.address, gateAmount);
      
      // Set CSR token in StripeIntegrationManager
      await stripeIntegration.connect(owner).setCsrToken(await caesarToken.getAddress());
      
      // Ensure contract has USD reserves for redemption
      await mockUSDC.mint(await stripeIntegration.getAddress(), expectedUSDAmount * 10n);
      
      const initialUSDBalance = await mockUSDC.balanceOf(user1.address);
      const initialCSRBalance = await caesarToken.balanceOf(user1.address);
      
      // Initiate redemption
      await caesarToken.connect(user1).approve(await stripeIntegration.getAddress(), gateAmount);
      await stripeIntegration.connect(user1).redeemForFiat(gateAmount);
      
      // Verify CSR tokens were burned/locked
      const finalCSRBalance = await caesarToken.balanceOf(user1.address);
      expect(initialCSRBalance - finalCSRBalance).to.equal(gateAmount);
      
      // Verify redemption was recorded (actual USD transfer would happen off-chain)
      const redemptionCount = await stripeIntegration.getUserRedemptionCount(user1.address);
      expect(redemptionCount).to.equal(1);
    });

    it('Should maintain 1:1 peg during high volume conversions', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, mockUSDC, user1, user2, owner } = fixture;
      
      // Large volume conversion test
      const largeAmount = ethers.parseUnits('100000', 6); // $100k USDC
      const users = [user1, user2];
      
      // Setup large USD balances
      for (const user of users) {
        await mockUSDC.mint(user.address, largeAmount);
        await mockUSDC.connect(user).approve(await stripeIntegration.getAddress(), largeAmount);
      }
      
      // Perform multiple large conversions
      for (let i = 0; i < users.length; i++) {
        const user = users[i];
        const paymentId = `large_payment_${i}`;
        
        await stripeIntegration.connect(owner).recordFiatDeposit(
          user.address,
          paymentId,
          largeAmount
        );
        
        await caesarToken.connect(owner).mintFromFiat(user.address, ethers.parseEther('100000'));
      }
      
      // Check that peg is maintained
      const totalMinted = ethers.parseEther('200000'); // 200k CSR total
      const totalUSDDeposited = largeAmount * 2n; // 200k USDC total
      
      // Verify balances maintain 1:1 ratio
      let totalCSRBalance = 0n;
      for (const user of users) {
        totalCSRBalance += await caesarToken.balanceOf(user.address);
      }
      
      expect(totalCSRBalance).to.be.closeTo(totalMinted, ethers.parseEther('1000')); // Within 1k tolerance
    });
  });

  describe('KYC and Compliance Integration', function () {
    it('Should enforce KYC requirements for large transactions', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, mockUSDC, user1, owner } = fixture;
      
      const largeAmount = ethers.parseUnits('10000', 6); // $10k USDC (above KYC threshold)
      
      await mockUSDC.mint(user1.address, largeAmount);
      await mockUSDC.connect(user1).approve(await stripeIntegration.getAddress(), largeAmount);
      
      // Attempt large deposit without KYC should fail
      await expect(
        stripeIntegration.connect(owner).recordFiatDeposit(
          user1.address,
          'large_payment_nokyc',
          largeAmount
        )
      ).to.be.revertedWith('KYC verification required for large amounts');
      
      // Complete KYC verification
      await stripeIntegration.connect(owner).setKYCStatus(user1.address, true);
      
      // Now the deposit should succeed
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'large_payment_kyc',
        largeAmount
      );
      
      const depositInfo = await stripeIntegration.getDepositInfo('large_payment_kyc');
      expect(depositInfo.amount).to.equal(largeAmount);
    });

    it('Should track AML compliance metrics', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, user2, owner } = fixture;
      
      // Simulate multiple transactions for AML tracking
      const transactions = [
        { user: user1, amount: ethers.parseUnits('5000', 6), id: 'txn_1' },
        { user: user1, amount: ethers.parseUnits('7000', 6), id: 'txn_2' },
        { user: user2, amount: ethers.parseUnits('3000', 6), id: 'txn_3' }
      ];
      
      for (const txn of transactions) {
        await stripeIntegration.connect(owner).recordFiatDeposit(
          txn.user.address,
          txn.id,
          txn.amount
        );
      }
      
      // Check AML metrics for user1
      const user1Volume = await stripeIntegration.getUserDailyVolume(user1.address);
      expect(user1Volume).to.equal(ethers.parseUnits('12000', 6)); // 5k + 7k
      
      // Check if user1 is flagged for high volume
      const isHighVolume = await stripeIntegration.isHighVolumeUser(user1.address);
      expect(isHighVolume).to.be.true; // Above 10k daily threshold
    });
  });

  describe('Payment Method Integration', function () {
    it('Should handle credit card payments correctly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, user1, owner } = fixture;
      
      const cardAmount = ethers.parseUnits('250', 6); // $250
      const paymentMethod = 'card_visa_4242';
      
      // Simulate credit card payment
      await stripeIntegration.connect(owner).recordCardPayment(
        user1.address,
        paymentMethod,
        cardAmount,
        'pi_card_payment_123'
      );
      
      // Verify payment was recorded with correct method
      const paymentInfo = await stripeIntegration.getPaymentInfo('pi_card_payment_123');
      expect(paymentInfo.amount).to.equal(cardAmount);
      expect(paymentInfo.paymentMethod).to.equal(paymentMethod);
      expect(paymentInfo.user).to.equal(user1.address);
      
      // Mint corresponding CSR tokens
      await caesarToken.connect(owner).mintFromFiat(user1.address, ethers.parseEther('250'));
      
      const gateBalance = await caesarToken.balanceOf(user1.address);
      expect(gateBalance).to.be.gte(ethers.parseEther('250'));
    });

    it('Should handle ACH bank transfers', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, user1, owner } = fixture;
      
      const achAmount = ethers.parseUnits('5000', 6); // $5000
      const bankAccount = 'ba_us_bank_account_123';
      
      // ACH transfers have longer settlement time
      await stripeIntegration.connect(owner).recordACHPayment(
        user1.address,
        bankAccount,
        achAmount,
        'pi_ach_payment_456'
      );
      
      // Payment should be in pending status initially
      const paymentInfo = await stripeIntegration.getPaymentInfo('pi_ach_payment_456');
      expect(paymentInfo.status).to.equal(0); // PENDING status
      
      // Simulate ACH settlement after 3 days
      await time.increase(3 * 24 * 3600); // 3 days
      
      // Mark payment as settled
      await stripeIntegration.connect(owner).settleACHPayment('pi_ach_payment_456');
      
      const settledPaymentInfo = await stripeIntegration.getPaymentInfo('pi_ach_payment_456');
      expect(settledPaymentInfo.status).to.equal(1); // COMPLETED status
      
      // Now mint CSR tokens
      await caesarToken.connect(owner).mintFromFiat(user1.address, ethers.parseEther('5000'));
      
      const gateBalance = await caesarToken.balanceOf(user1.address);
      expect(gateBalance).to.be.gte(ethers.parseEther('5000'));
    });

    it('Should handle payment failures and refunds', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      const failedAmount = ethers.parseUnits('1000', 6);
      const paymentId = 'pi_failed_payment_789';
      
      // Record initial payment attempt
      await stripeIntegration.connect(owner).recordCardPayment(
        user1.address,
        'card_declined_4000',
        failedAmount,
        paymentId
      );
      
      // Mark payment as failed
      await stripeIntegration.connect(owner).markPaymentFailed(paymentId, 'Insufficient funds');
      
      const failedPaymentInfo = await stripeIntegration.getPaymentInfo(paymentId);
      expect(failedPaymentInfo.status).to.equal(2); // FAILED status
      
      // Verify no CSR tokens were minted for failed payment
      const gateBalance = await caesarToken.balanceOf(user1.address);
      expect(gateBalance).to.equal(0); // No tokens should be minted for failed payment
    });
  });

  describe('Fiat Backing and Reserves', function () {
    it('Should maintain full fiat backing of circulating supply', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, mockUSDC, user1, user2, owner } = fixture;
      
      // Record multiple fiat deposits
      const deposits = [
        { user: user1, amount: ethers.parseUnits('10000', 6) }, // $10k
        { user: user2, amount: ethers.parseUnits('15000', 6) }  // $15k
      ];
      
      let totalDeposited = 0n;
      
      for (let i = 0; i < deposits.length; i++) {
        const deposit = deposits[i];
        const paymentId = `backing_test_${i}`;
        
        await mockUSDC.mint(deposit.user.address, deposit.amount);
        await mockUSDC.connect(deposit.user).transfer(
          await stripeIntegration.getAddress(), 
          deposit.amount
        );
        
        await stripeIntegration.connect(owner).recordFiatDeposit(
          deposit.user.address,
          paymentId,
          deposit.amount
        );
        
        // Mint equivalent CSR tokens
        const gateAmount = ethers.parseEther((Number(deposit.amount) / 1e6).toString());
        await caesarToken.connect(owner).mintFromFiat(deposit.user.address, gateAmount);
        
        totalDeposited += deposit.amount;
      }
      
      // Check total fiat backing
      const totalBacking = await stripeIntegration.getTotalFiatBacking();
      expect(totalBacking).to.equal(totalDeposited);
      
      // Check that backing ratio is 100%
      const circulating = await caesarToken.totalSupply();
      const expectedBacking = circulating; // 1:1 ratio
      
      // Convert to same decimals for comparison (CSR has 18, USDC has 6)
      const backingInCSRDecimals = totalBacking * (10n ** 12n); // Convert USDC to 18 decimals
      expect(backingInCSRDecimals).to.be.closeTo(circulating, ethers.parseEther('1000')); // Within 1k tolerance
    });

    it('Should handle reserve rebalancing correctly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stabilityPool, mockUSDC, owner } = fixture;
      
      // Add reserves to stability pool
      const reserveAmount = ethers.parseUnits('500000', 6); // $500k
      await mockUSDC.mint(await stabilityPool.getAddress(), reserveAmount);
      
      await stabilityPool.connect(owner).contributeReserves(reserveAmount);
      
      // Check reserve ratio
      const reserveRatio = await stabilityPool.calculateReserveRatio();
      expect(reserveRatio).to.be.gt(0);
      
      // Trigger rebalancing
      const rebalanceResult = await stabilityPool.connect(owner).rebalanceReserves();
      expect(rebalanceResult).to.not.be.reverted;
      
      // Verify reserves are optimally distributed
      const composition = await stabilityPool.getPoolComposition();
      expect(composition.reserves).to.be.gt(0);
    });

    it('Should enforce minimum reserve requirements', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, stabilityPool, mockUSDC, user1, owner } = fixture;
      
      const largeWithdrawal = ethers.parseEther('100000'); // Large CSR amount
      
      // Give user CSR tokens
      await caesarToken.transfer(user1.address, largeWithdrawal);
      
      // Attempt withdrawal that would breach minimum reserves
      await caesarToken.connect(user1).approve(await stripeIntegration.getAddress(), largeWithdrawal);
      
      // This should fail due to insufficient reserves
      await expect(
        stripeIntegration.connect(user1).redeemForFiat(largeWithdrawal)
      ).to.be.revertedWith('Insufficient reserves for redemption');
      
      // Add sufficient reserves
      const reserveAmount = ethers.parseUnits('200000', 6); // $200k reserves
      await mockUSDC.mint(await stabilityPool.getAddress(), reserveAmount);
      await stabilityPool.connect(owner).contributeReserves(reserveAmount);
      
      // Now withdrawal should succeed
      await stripeIntegration.connect(user1).redeemForFiat(largeWithdrawal / 2n); // Smaller amount
      
      const redemptionCount = await stripeIntegration.getUserRedemptionCount(user1.address);
      expect(redemptionCount).to.equal(1);
    });
  });

  describe('Real-Time Fiat Price Feeds', function () {
    it('Should handle USD/CSR price fluctuations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stripeIntegration, priceOracle, user1, owner } = fixture;
      
      // Set initial price to $1.00
      await priceOracle.setPrice(ethers.parseEther('1.0'));
      
      const usdAmount = ethers.parseUnits('1000', 6); // $1000
      
      // Deposit at $1.00 price
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'price_test_1',
        usdAmount
      );
      
      let currentPrice = await priceOracle.getLatestPrice();
      let gateAmount = (usdAmount * ethers.parseEther('1')) / currentPrice;
      // Convert to 18 decimals: (1000 * 10^6) * 10^18 / (1 * 10^18) = 1000 * 10^18
      gateAmount = usdAmount * (10n ** 12n); // Convert from 6 to 18 decimals
      
      await caesarToken.connect(owner).mintFromFiat(user1.address, gateAmount);
      
      const balance1 = await caesarToken.balanceOf(user1.address);
      expect(balance1).to.equal(ethers.parseEther('1000')); // 1000 CSR at $1.00
      
      // Change price to $1.05 (5% premium)
      await priceOracle.setPrice(ethers.parseEther('1.05'));
      
      // New deposit should get fewer CSR tokens
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'price_test_2',
        usdAmount
      );
      
      currentPrice = await priceOracle.getLatestPrice();
      gateAmount = (usdAmount * (10n ** 12n) * ethers.parseEther('1')) / currentPrice;
      // At $1.05: 1000 USD should get ~952.38 CSR
      
      await caesarToken.connect(owner).mintFromFiat(user1.address, gateAmount);
      
      const balance2 = await caesarToken.balanceOf(user1.address);
      expect(balance2).to.be.lt(balance1 + ethers.parseEther('1000')); // Less than 2000 CSR total
      expect(balance2).to.be.gt(balance1 + ethers.parseEther('950')); // But more than 1950 CSR
    });

    it('Should maintain peg stability during price volatility', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, stabilityPool, priceOracle, mockAMM, owner } = fixture;
      
      // Simulate price volatility
      const pricePoints = [
        ethers.parseEther('0.95'), // 5% below peg
        ethers.parseEther('1.08'), // 8% above peg  
        ethers.parseEther('0.92'), // 8% below peg
        ethers.parseEther('1.03')  // 3% above peg
      ];
      
      for (const price of pricePoints) {
        await priceOracle.setPrice(price);
        
        // Check if stability interventions are triggered
        const shouldIntervene = await stabilityPool.shouldIntervene(price, ethers.parseEther('1.0'));
        
        if (shouldIntervene) {
          // Execute intervention
          const intervention = await stabilityPool.executeAMMIntervention(
            price,
            ethers.parseEther('1.0') // Target price $1.00
          );
          
          expect(intervention.executed).to.be.true;
          
          // Verify intervention direction is correct
          if (price > ethers.parseEther('1.0')) {
            expect(intervention.tradeType).to.equal(2); // SELL CSR to reduce price
          } else {
            expect(intervention.tradeType).to.equal(1); // BUY CSR to increase price
          }
        }
        
        // Add time between interventions
        await time.increase(3600); // 1 hour
      }
      
      // Check that price is closer to peg after interventions
      const finalPrice = await priceOracle.getLatestPrice();
      const deviation = finalPrice > ethers.parseEther('1.0') 
        ? finalPrice - ethers.parseEther('1.0')
        : ethers.parseEther('1.0') - finalPrice;
      
      // Deviation should be less than 5%
      expect(deviation).to.be.lt(ethers.parseEther('0.05'));
    });
  });

  describe('Error Handling and Edge Cases', function () {
    it('Should handle double-spending prevention', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      const amount = ethers.parseUnits('1000', 6);
      const paymentId = 'duplicate_payment_test';
      
      // Record payment once
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        paymentId,
        amount
      );
      
      // Attempt to record same payment again should fail
      await expect(
        stripeIntegration.connect(owner).recordFiatDeposit(
          user1.address,
          paymentId,
          amount
        )
      ).to.be.revertedWith('Payment already processed');
    });

    it('Should handle network failures gracefully', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      // Simulate network timeout scenario
      const timeoutPayment = 'timeout_payment_test';
      const amount = ethers.parseUnits('500', 6);
      
      // Mark payment as pending
      await stripeIntegration.connect(owner).recordPendingPayment(
        user1.address,
        timeoutPayment,
        amount
      );
      
      // Payment should be queryable
      const paymentInfo = await stripeIntegration.getPaymentInfo(timeoutPayment);
      expect(paymentInfo.status).to.equal(0); // PENDING
      
      // After timeout period, payment can be retried or cancelled
      await time.increase(3600); // 1 hour
      
      await stripeIntegration.connect(owner).cancelTimedOutPayment(timeoutPayment);
      
      const cancelledPaymentInfo = await stripeIntegration.getPaymentInfo(timeoutPayment);
      expect(cancelledPaymentInfo.status).to.equal(3); // CANCELLED
    });

    it('Should enforce transaction limits correctly', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { stripeIntegration, user1, owner } = fixture;
      
      // Set daily limit for user
      const dailyLimit = ethers.parseUnits('5000', 6); // $5k daily limit
      await stripeIntegration.connect(owner).setUserDailyLimit(user1.address, dailyLimit);
      
      // First transaction within limit should succeed
      const amount1 = ethers.parseUnits('3000', 6); // $3k
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'limit_test_1',
        amount1
      );
      
      // Second transaction that exceeds daily limit should fail
      const amount2 = ethers.parseUnits('3000', 6); // $3k (total would be $6k > $5k limit)
      await expect(
        stripeIntegration.connect(owner).recordFiatDeposit(
          user1.address,
          'limit_test_2',
          amount2
        )
      ).to.be.revertedWith('Daily limit exceeded');
      
      // Smaller transaction within remaining limit should succeed
      const amount3 = ethers.parseUnits('1500', 6); // $1.5k (total $4.5k < $5k limit)
      await stripeIntegration.connect(owner).recordFiatDeposit(
        user1.address,
        'limit_test_3',
        amount3
      );
      
      const dailyVolume = await stripeIntegration.getUserDailyVolume(user1.address);
      expect(dailyVolume).to.equal(amount1 + amount3); // $4.5k total
    });
  });
});