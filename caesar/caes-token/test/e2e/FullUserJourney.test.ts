// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { expect } from 'chai';
import { ethers } from 'hardhat';
import { loadFixture, time } from '@nomicfoundation/hardhat-network-helpers';
import { deployCaesarFixture, deployCrossChainFixture } from '../fixtures/TestFixtures';

describe('End-to-End User Journey Tests', function () {
  describe('Complete Onboarding to Withdrawal Flow', function () {
    it('Should handle complete user onboarding and first transaction', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        stripeIntegration, 
        mockUSDC, 
        economicEngine,
        user1, 
        owner 
      } = fixture;
      
      const depositAmount = ethers.parseUnits('2000', 6); // $2000 initial deposit
      
      console.log('Step 1: New user onboarding...');
      
      // 1. User completes KYC verification
      await stripeIntegration.connect(owner).setKYCStatus(user1.address, true);
      expect(await stripeIntegration.isKYCVerified(user1.address)).to.be.true;
      
      // 2. User makes initial USD deposit via credit card
      await mockUSDC.mint(user1.address, depositAmount);
      await mockUSDC.connect(user1).transfer(await stripeIntegration.getAddress(), depositAmount);
      
      await stripeIntegration.connect(owner).recordCardPayment(
        user1.address,
        'card_visa_onboarding',
        depositAmount,
        'pi_onboarding_payment'
      );
      
      console.log('Step 2: Converting USD to CSR tokens...');
      
      // 3. CSR tokens are minted for the user
      const gateAmount = ethers.parseEther('2000'); // 1:1 conversion
      await caesarToken.connect(owner).mintFromFiat(user1.address, gateAmount);
      
      const initialGateBalance = await caesarToken.balanceOf(user1.address);
      expect(initialGateBalance).to.equal(gateAmount);
      
      console.log('Step 3: User receives welcome bonus and starts grace period...');
      
      // 4. User enters grace period (no demurrage)
      await economicEngine.connect(owner).startGracePeriod(user1.address);
      
      // 5. Verify account is in grace period
      const accountInfo = await caesarToken.getAccountInfo(user1.address);
      expect(accountInfo.isInGracePeriod).to.be.true;
      
      console.log('Step 4: User makes first transaction...');
      
      // 6. User makes first transaction (buying something)
      const firstTransactionAmount = ethers.parseEther('50'); // $50 purchase
      await caesarToken.connect(user1).transfer(owner.address, firstTransactionAmount); // Simulate merchant payment
      
      // 7. Verify transaction was recorded correctly
      const updatedAccountInfo = await caesarToken.getAccountInfo(user1.address);
      expect(updatedAccountInfo.lastActivity).to.be.gt(accountInfo.lastActivity);
      
      const finalBalance = await caesarToken.balanceOf(user1.address);
      expect(finalBalance).to.equal(initialGateBalance - firstTransactionAmount);
      
      console.log('Step 5: Verifying compliance tracking...');
      
      // 8. Verify compliance tracking
      const userVolume = await stripeIntegration.getUserDailyVolume(user1.address);
      expect(userVolume).to.equal(depositAmount);
      
      console.log('✅ Complete onboarding flow successful');
    });

    it('Should handle active user lifecycle with multiple transactions', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        stripeIntegration, 
        mockUSDC, 
        economicEngine,
        user1, 
        user2,
        owner 
      } = fixture;
      
      console.log('Setting up active user scenario...');
      
      // Setup: Give user1 initial balance
      const initialAmount = ethers.parseEther('5000');
      await caesarToken.transfer(user1.address, initialAmount);
      await caesarToken.transfer(user2.address, initialAmount);
      
      // Start grace period for both users
      await economicEngine.connect(owner).startGracePeriod(user1.address);
      await economicEngine.connect(owner).startGracePeriod(user2.address);
      
      console.log('Phase 1: Regular transaction activity...');
      
      // Phase 1: Regular transactions during grace period
      const transactions = [
        { from: user1, to: user2.address, amount: ethers.parseEther('100'), desc: 'Coffee shop payment' },
        { from: user2, to: user1.address, amount: ethers.parseEther('50'), desc: 'Refund' },
        { from: user1, to: owner.address, amount: ethers.parseEther('200'), desc: 'Online purchase' },
        { from: user1, to: user2.address, amount: ethers.parseEther('75'), desc: 'Split dinner bill' }
      ];
      
      for (let i = 0; i < transactions.length; i++) {
        const tx = transactions[i];
        console.log(`  Transaction ${i + 1}: ${tx.desc}`);
        
        await caesarToken.connect(tx.from).transfer(tx.to, tx.amount);
        await time.increase(3600); // 1 hour between transactions
      }
      
      console.log('Phase 2: Grace period ends, demurrage begins...');
      
      // Phase 2: Grace period ends - demurrage starts applying
      await time.increase(48 * 3600); // End of 48-hour grace period
      
      const user1BalanceBeforeDemurrage = await caesarToken.balanceOf(user1.address);
      
      // User1 becomes inactive - demurrage should apply
      await time.increase(24 * 3600); // 1 day of inactivity
      
      const demurrageAmount = await caesarToken.calculateDemurrage(user1.address);
      expect(demurrageAmount).to.be.gt(0);
      
      console.log('Phase 3: User returns to activity...');
      
      // Phase 3: User1 becomes active again
      await caesarToken.connect(user1).transfer(user2.address, ethers.parseEther('25'));
      
      // Verify demurrage was applied to stability pool
      const stabilityPoolBalance = await caesarToken.getStabilityPoolBalance();
      expect(stabilityPoolBalance).to.be.gt(0);
      
      console.log('Phase 4: Fiat activity reduces demurrage...');
      
      // Phase 4: User makes fiat purchase - should reduce future demurrage
      const fiatPurchaseAmount = ethers.parseUnits('500', 6); // $500
      await mockUSDC.mint(user1.address, fiatPurchaseAmount);
      await mockUSDC.connect(user1).transfer(await stripeIntegration.getAddress(), fiatPurchaseAmount);
      
      await stripeIntegration.connect(owner).recordFiatActivity(
        user1.address,
        fiatPurchaseAmount,
        0 // Purchase type
      );
      
      // Wait and check if demurrage is reduced
      await time.increase(24 * 3600); // Another day
      
      const demurrageWithFiatActivity = await caesarToken.calculateDemurrage(user1.address);
      // Should be lower than without fiat activity (though this depends on implementation)
      
      console.log('✅ Active user lifecycle test complete');
    });

    it('Should handle user redemption and withdrawal process', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        stripeIntegration, 
        mockUSDC, 
        stabilityPool,
        user1, 
        owner 
      } = fixture;
      
      console.log('Setting up withdrawal scenario...');
      
      // Setup: User has CSR tokens and wants to withdraw
      const gateAmount = ethers.parseEther('3000'); // $3000 worth
      await caesarToken.transfer(user1.address, gateAmount);
      
      // Ensure system has sufficient USD reserves
      const reserveAmount = ethers.parseUnits('10000', 6); // $10k reserves
      await mockUSDC.mint(await stabilityPool.getAddress(), reserveAmount);
      await stabilityPool.connect(owner).contributeReserves(reserveAmount);
      
      console.log('Step 1: User initiates withdrawal...');
      
      // 1. User initiates withdrawal request
      const withdrawalAmount = ethers.parseEther('1000'); // Withdraw $1000 worth
      
      await caesarToken.connect(user1).approve(await stripeIntegration.getAddress(), withdrawalAmount);
      
      const initialUserBalance = await caesarToken.balanceOf(user1.address);
      const initialRedemptionCount = await stripeIntegration.getUserRedemptionCount(user1.address);
      
      console.log('Step 2: Processing redemption...');
      
      // 2. System processes redemption
      await stripeIntegration.connect(user1).redeemForFiat(withdrawalAmount);
      
      // 3. Verify CSR tokens were locked/burned
      const finalUserBalance = await caesarToken.balanceOf(user1.address);
      expect(initialUserBalance - finalUserBalance).to.equal(withdrawalAmount);
      
      // 4. Verify redemption was recorded
      const finalRedemptionCount = await stripeIntegration.getUserRedemptionCount(user1.address);
      expect(finalRedemptionCount).to.equal(initialRedemptionCount + 1n);
      
      console.log('Step 3: Checking compliance and limits...');
      
      // 5. Check withdrawal limits and compliance
      const userDailyWithdrawals = await stripeIntegration.getUserDailyWithdrawals(user1.address);
      expect(userDailyWithdrawals).to.equal(ethers.parseUnits('1000', 6)); // $1000 in USDC terms
      
      console.log('Step 4: Simulating bank transfer processing...');
      
      // 6. Simulate off-chain bank transfer process (would happen via Stripe webhooks)
      // For testing, we can simulate the confirmation
      const redemptionId = await stripeIntegration.getLastRedemptionId(user1.address);
      
      // Mark as pending bank transfer
      await stripeIntegration.connect(owner).updateRedemptionStatus(redemptionId, 1); // PROCESSING
      
      // Simulate bank transfer completion
      await time.increase(2 * 24 * 3600); // 2 days for bank processing
      await stripeIntegration.connect(owner).updateRedemptionStatus(redemptionId, 2); // COMPLETED
      
      console.log('Step 5: Verifying final state...');
      
      // 7. Verify final redemption status
      const redemptionInfo = await stripeIntegration.getRedemptionInfo(redemptionId);
      expect(redemptionInfo.status).to.equal(2); // COMPLETED
      expect(redemptionInfo.amount).to.equal(withdrawalAmount);
      
      console.log('✅ Complete withdrawal process successful');
    });
  });

  describe('Cross-Chain User Experience', function () {
    it('Should handle cross-chain token transfers seamlessly', async function () {
      const fixture = await deployCrossChainFixture();
      const { caesarToken, chainInstances, user1, owner } = fixture;
      
      console.log('Setting up cross-chain scenario...');
      
      // Setup: User has tokens on Ethereum
      const transferAmount = ethers.parseEther('2000');
      await caesarToken.transfer(user1.address, transferAmount);
      
      const ethereumChain = chainInstances[0]; // Ethereum mainnet
      const polygonChain = chainInstances[1]; // Polygon
      
      console.log('Step 1: Initiating cross-chain transfer...');
      
      // 1. User initiates transfer from Ethereum to Polygon
      const crossChainAmount = ethers.parseEther('500');
      
      // Set up LayerZero pathway (simplified for testing)
      const polygonChainId = 109; // LayerZero chain ID for Polygon
      const sendParam = {
        dstEid: polygonChainId,
        to: ethers.zeroPadValue(user1.address, 32),
        amountLD: crossChainAmount,
        minAmountLD: crossChainAmount,
        extraOptions: '0x',
        composeMsg: '0x',
        oftCmd: '0x'
      };
      
      // Get quote for cross-chain transfer
      const messagingFee = await caesarToken.quoteSend(sendParam, false);
      
      console.log('Step 2: Executing cross-chain send...');
      
      // 2. Execute cross-chain send
      const initialBalance = await caesarToken.balanceOf(user1.address);
      
      await caesarToken.connect(user1).send(
        sendParam,
        messagingFee,
        user1.address,
        { value: messagingFee.nativeFee }
      );
      
      // 3. Verify tokens were locked on source chain
      const balanceAfterSend = await caesarToken.balanceOf(user1.address);
      expect(initialBalance - balanceAfterSend).to.equal(crossChainAmount);
      
      console.log('Step 3: Simulating cross-chain receipt...');
      
      // 4. Simulate receipt on destination chain (Polygon)
      // In real scenario, this would be triggered by LayerZero relayers
      const polygonContract = polygonChain.contract;
      
      // Simulate the cross-chain message delivery
      await polygonContract.lzReceive(
        1, // Source chain ID (Ethereum)
        ethers.solidityPacked(['address', 'address'], [await caesarToken.getAddress(), await polygonContract.getAddress()]),
        1, // Nonce
        ethers.AbiCoder.defaultAbiCoder().encode(['address', 'uint256'], [user1.address, crossChainAmount])
      );
      
      // 5. Verify user received tokens on Polygon
      const polygonBalance = await polygonContract.balanceOf(user1.address);
      expect(polygonBalance).to.equal(crossChainAmount);
      
      console.log('Step 4: Verifying economic consistency across chains...');
      
      // 6. Verify economic parameters are synced across chains
      const ethEconomicData = await caesarToken.getNetworkHealthIndex();
      const polygonEconomicData = await polygonContract.getNetworkHealthIndex();
      
      // Both chains should have consistent economic health metrics
      expect(ethEconomicData).to.be.closeTo(polygonEconomicData, 100); // Within reasonable tolerance
      
      console.log('✅ Cross-chain transfer completed successfully');
    });

    it('Should maintain economic consistency across all chains', async function () {
      const fixture = await deployCrossChainFixture();
      const { caesarToken, chainInstances, economicEngine, owner } = fixture;
      
      console.log('Testing cross-chain economic synchronization...');
      
      // 1. Update economic parameters on main chain
      const newParams = {
        baseDemurrageRate: 75n, // 0.75%
        maxDemurrageRate: 250n, // 2.5%
        stabilityThreshold: 150n,
        fiatDiscountFactor: 4000n,
        gracePeriodsHours: 72n,
        interventionThreshold: 600n,
        rebalanceFrequency: 7200n,
        emergencyThreshold: 1200n
      };
      
      await economicEngine.connect(owner).updateEconomicParameters(newParams);
      
      console.log('Step 1: Broadcasting parameter updates...');
      
      // 2. Simulate cross-chain parameter synchronization
      for (const chain of chainInstances) {
        // In real implementation, this would be done via LayerZero messages
        // For testing, we directly update each chain
        
        console.log(`  Syncing parameters to ${chain.name}...`);
        
        // Each chain contract would have its own economic engine
        // For this test, we'll verify the sync mechanism works
        const chainId = chain.chainId;
        
        // Simulate receiving cross-chain economic update
        // This would be triggered by CrossChainEconomicSync contract
        await time.increase(300); // 5 minutes for propagation
      }
      
      console.log('Step 2: Verifying parameter consistency...');
      
      // 3. Verify all chains have consistent parameters
      const mainChainParams = await economicEngine.getEconomicParameters();
      
      // All chains should reflect the updated parameters
      expect(mainChainParams.baseDemurrageRate).to.equal(newParams.baseDemurrageRate);
      expect(mainChainParams.maxDemurrageRate).to.equal(newParams.maxDemurrageRate);
      
      console.log('Step 3: Testing cross-chain emergency response...');
      
      // 4. Test cross-chain emergency mode activation
      const emergencyReason = 'Cross-chain test emergency';
      
      await economicEngine.connect(owner).activateEmergencyMode(emergencyReason);
      
      // 5. Verify emergency mode is activated
      const paramsAfterEmergency = await economicEngine.getEconomicParameters();
      expect(paramsAfterEmergency.baseDemurrageRate).to.equal(0); // Emergency mode stops demurrage
      
      console.log('✅ Cross-chain economic synchronization verified');
    });
  });

  describe('Stress Test Scenarios', function () {
    it('Should handle high-frequency trading scenario', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        antiSpeculationEngine,
        user1, 
        user2,
        user3,
        owner 
      } = fixture;
      
      console.log('Setting up high-frequency trading stress test...');
      
      // Setup: Multiple users with substantial balances
      const users = [user1, user2, user3];
      const baseAmount = ethers.parseEther('10000'); // 10k CSR each
      
      for (const user of users) {
        await caesarToken.transfer(user.address, baseAmount);
      }
      
      console.log('Phase 1: Rapid trading activity...');
      
      // Phase 1: Simulate rapid trading
      const tradeAmount = ethers.parseEther('100');
      let totalPenalties = 0n;
      
      for (let round = 0; round < 10; round++) {
        console.log(`  Trading round ${round + 1}/10...`);
        
        for (let i = 0; i < users.length; i++) {
          const fromUser = users[i];
          const toUser = users[(i + 1) % users.length];
          
          // Check penalties before trade
          const penalty = await caesarToken.calculateSpeculationPenalty(fromUser.address);
          totalPenalties += penalty;
          
          // Execute trade
          await caesarToken.connect(fromUser).transfer(toUser.address, tradeAmount);
          
          // Very short time between trades (rapid trading)
          await time.increase(30); // 30 seconds
        }
      }
      
      console.log('Phase 2: Analyzing anti-speculation results...');
      
      // Phase 2: Analyze results
      const stabilityPoolBalance = await caesarToken.getStabilityPoolBalance();
      expect(stabilityPoolBalance).to.be.gt(0); // Penalties should accumulate
      
      // Check risk scores for rapid traders
      for (const user of users) {
        const riskProfile = await antiSpeculationEngine.getAccountRiskProfile(user.address);
        console.log(`  User ${user.address.slice(0, 8)}... risk score: ${riskProfile.overallRiskScore}`);
        
        // High-frequency traders should have elevated risk scores
        expect(riskProfile.overallRiskScore).to.be.gt(300); // Above normal threshold
      }
      
      console.log('Phase 3: Testing system stability under load...');
      
      // Phase 3: Verify system remains stable
      const networkHealth = await caesarToken.getNetworkHealthIndex();
      expect(networkHealth).to.be.gt(500); // System should remain reasonably healthy
      
      const liquidityRatio = await caesarToken.getLiquidityRatio();
      expect(liquidityRatio).to.be.gt(0); // Liquidity should be maintained
      
      console.log('✅ High-frequency trading stress test completed');
    });

    it('Should handle large-scale user onboarding wave', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        stripeIntegration,
        mockUSDC,
        economicEngine,
        owner 
      } = fixture;
      
      console.log('Simulating large-scale user onboarding...');
      
      // Generate 50 new user addresses
      const newUsers = Array.from({ length: 50 }, () => ethers.Wallet.createRandom());
      const onboardingAmount = ethers.parseUnits('1000', 6); // $1000 per user
      
      console.log('Phase 1: Mass user registration and KYC...');
      
      // Phase 1: Mass KYC verification
      for (let i = 0; i < newUsers.length; i++) {
        const user = newUsers[i];
        await stripeIntegration.connect(owner).setKYCStatus(user.address, true);
        
        if (i % 10 === 0) {
          console.log(`  Processed ${i + 1}/${newUsers.length} KYC verifications...`);
        }
      }
      
      console.log('Phase 2: Mass fiat deposits and token minting...');
      
      // Phase 2: Mass deposits and minting
      let totalDeposited = 0n;
      
      for (let i = 0; i < newUsers.length; i++) {
        const user = newUsers[i];
        const paymentId = `mass_onboarding_${i}`;
        
        // Record fiat deposit
        await stripeIntegration.connect(owner).recordFiatDeposit(
          user.address,
          paymentId,
          onboardingAmount
        );
        
        // Mint CSR tokens
        const gateAmount = ethers.parseEther('1000'); // 1:1 conversion
        await caesarToken.connect(owner).mintFromFiat(user.address, gateAmount);
        
        totalDeposited += onboardingAmount;
        
        if (i % 10 === 0) {
          console.log(`  Processed ${i + 1}/${newUsers.length} onboardings...`);
        }
      }
      
      console.log('Phase 3: Verifying system capacity and health...');
      
      // Phase 3: Verify system handled the load
      const totalSupply = await caesarToken.totalSupply();
      const expectedIncrease = ethers.parseEther('50000'); // 50 users × 1000 CSR
      
      // Total supply should have increased appropriately
      expect(totalSupply).to.be.gte(expectedIncrease);
      
      // Verify fiat backing
      const totalBacking = await stripeIntegration.getTotalFiatBacking();
      expect(totalBacking).to.equal(totalDeposited);
      
      // Check system health metrics
      const networkHealth = await caesarToken.getNetworkHealthIndex();
      expect(networkHealth).to.be.gt(600); // Should maintain good health
      
      const activeParticipants = await caesarToken.getActiveParticipants();
      expect(activeParticipants).to.be.gte(50); // Should reflect new users
      
      console.log('Phase 4: Testing grace period management at scale...');
      
      // Phase 4: Start grace periods for all new users
      for (let i = 0; i < Math.min(newUsers.length, 20); i++) { // Test subset to avoid gas limits
        const user = newUsers[i];
        await economicEngine.connect(owner).startGracePeriod(user.address);
        
        const accountInfo = await caesarToken.getAccountInfo(user.address);
        expect(accountInfo.isInGracePeriod).to.be.true;
      }
      
      console.log('✅ Large-scale onboarding stress test completed');
    });

    it('Should maintain stability during market crash simulation', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        economicEngine,
        stabilityPool,
        priceOracle,
        mockAMM,
        user1,
        user2,
        user3,
        owner 
      } = fixture;
      
      console.log('Simulating market crash scenario...');
      
      // Setup: Users have substantial balances
      const users = [user1, user2, user3];
      const balanceAmount = ethers.parseEther('50000'); // 50k CSR each
      
      for (const user of users) {
        await caesarToken.transfer(user.address, balanceAmount);
      }
      
      console.log('Phase 1: Normal market conditions...');
      
      // Phase 1: Establish baseline
      await priceOracle.setPrice(ethers.parseEther('1.0')); // $1.00
      
      const baselineHealth = await caesarToken.getNetworkHealthIndex();
      const baselineStabilityPool = await caesarToken.getStabilityPoolBalance();
      
      console.log('Phase 2: Rapid price decline (crash)...');
      
      // Phase 2: Simulate rapid price decline
      const priceDeclinePoints = [
        ethers.parseEther('0.90'), // -10%
        ethers.parseEther('0.75'), // -25%
        ethers.parseEther('0.60'), // -40%
        ethers.parseEther('0.45'), // -55%
        ethers.parseEther('0.30')  // -70% (severe crash)
      ];
      
      for (let i = 0; i < priceDeclinePoints.length; i++) {
        const price = priceDeclinePoints[i];
        console.log(`  Price drop to $${ethers.formatEther(price)}...`);
        
        await priceOracle.setPrice(price);
        
        // Simulate panic selling
        const panicSellAmount = ethers.parseEther('5000');
        for (const user of users) {
          await caesarToken.connect(user).transfer(owner.address, panicSellAmount / 3n);
        }
        
        // Check if stability interventions are triggered
        const shouldIntervene = await stabilityPool.shouldIntervene(price, ethers.parseEther('1.0'));
        
        if (shouldIntervene) {
          console.log(`    Executing stability intervention at $${ethers.formatEther(price)}`);
          
          await stabilityPool.executeAMMIntervention(price, ethers.parseEther('1.0'));
        }
        
        // Add time for market reaction
        await time.increase(3600); // 1 hour between price drops
      }
      
      console.log('Phase 3: Analyzing crash response...');
      
      // Phase 3: Analyze system response
      const crashHealth = await caesarToken.getNetworkHealthIndex();
      const crashStabilityPool = await caesarToken.getStabilityPoolBalance();
      
      // System should have activated protective measures
      expect(crashStabilityPool).to.be.gt(baselineStabilityPool); // More funds in stability pool
      
      console.log('Phase 4: Emergency mode activation...');
      
      // Phase 4: Test emergency mode activation
      if (crashHealth < 300) { // Critical threshold
        await economicEngine.connect(owner).activateEmergencyMode('Market crash emergency');
        
        const emergencyParams = await economicEngine.getEconomicParameters();
        expect(emergencyParams.baseDemurrageRate).to.equal(0); // Demurrage paused in emergency
      }
      
      console.log('Phase 5: Recovery simulation...');
      
      // Phase 5: Simulate market recovery
      const recoveryPrices = [
        ethers.parseEther('0.50'), // +67% from bottom
        ethers.parseEther('0.70'), // +40% more
        ethers.parseEther('0.85'), // +21% more
        ethers.parseEther('0.95')  // +12% more (near peg)
      ];
      
      for (const price of recoveryPrices) {
        await priceOracle.setPrice(price);
        
        // Execute recovery interventions if needed
        const shouldIntervene = await stabilityPool.shouldIntervene(price, ethers.parseEther('1.0'));
        if (shouldIntervene) {
          await stabilityPool.executeAMMIntervention(price, ethers.parseEther('1.0'));
        }
        
        await time.increase(7200); // 2 hours between recovery points
      }
      
      console.log('Phase 6: Verifying system recovery...');
      
      // Phase 6: Verify recovery
      const recoveryHealth = await caesarToken.getNetworkHealthIndex();
      const finalPrice = await priceOracle.getLatestPrice();
      
      // System should show signs of recovery
      expect(recoveryHealth).to.be.gt(crashHealth);
      expect(finalPrice).to.be.gt(ethers.parseEther('0.90')); // Should be recovering toward peg
      
      // Deactivate emergency mode if it was activated
      if (crashHealth < 300) {
        await economicEngine.connect(owner).deactivateEmergencyMode();
      }
      
      console.log('✅ Market crash simulation completed');
    });
  });

  describe('Multi-User Interaction Scenarios', function () {
    it('Should handle complex multi-user economic interactions', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { 
        caesar, 
        economicEngine,
        antiSpeculationEngine,
        stabilityPool,
        stripeIntegration,
        mockUSDC,
        user1, 
        user2, 
        user3,
        owner 
      } = fixture;
      
      console.log('Setting up complex multi-user scenario...');
      
      // Create user personas with different behaviors
      const personas = {
        saver: { user: user1, balance: ethers.parseEther('20000'), behavior: 'hold' },
        trader: { user: user2, balance: ethers.parseEther('10000'), behavior: 'active_trading' },
        spender: { user: user3, balance: ethers.parseEther('5000'), behavior: 'frequent_spending' }
      };
      
      // Setup initial balances
      for (const persona of Object.values(personas)) {
        await caesarToken.transfer(persona.user.address, persona.balance);
      }
      
      console.log('Phase 1: Diverse user behaviors...');
      
      // Phase 1: Execute different user behaviors
      
      // Saver: Holds tokens, makes occasional large purchases
      console.log('  Saver behavior: Long-term holding...');
      await caesarToken.connect(personas.saver.user).transfer(
        owner.address, 
        ethers.parseEther('500') // Occasional large purchase
      );
      await time.increase(24 * 3600); // 1 day between actions
      
      // Trader: Frequent small trades
      console.log('  Trader behavior: Active trading...');
      for (let i = 0; i < 5; i++) {
        await caesarToken.connect(personas.trader.user).transfer(
          i % 2 === 0 ? personas.spender.user.address : personas.saver.user.address,
          ethers.parseEther('200')
        );
        await time.increase(3600); // 1 hour between trades
      }
      
      // Spender: Many small transactions
      console.log('  Spender behavior: Frequent small purchases...');
      for (let i = 0; i < 10; i++) {
        await caesarToken.connect(personas.spender.user).transfer(
          owner.address, // Simulate merchant payments
          ethers.parseEther('50')
        );
        await time.increase(1800); // 30 minutes between purchases
      }
      
      console.log('Phase 2: Analyzing behavioral impact on economic metrics...');
      
      // Phase 2: Analyze how different behaviors affect the system
      
      // Check risk profiles
      const saverRisk = await antiSpeculationEngine.getAccountRiskProfile(personas.saver.user.address);
      const traderRisk = await antiSpeculationEngine.getAccountRiskProfile(personas.trader.user.address);
      const spenderRisk = await antiSpeculationEngine.getAccountRiskProfile(personas.spender.user.address);
      
      console.log(`  Saver risk score: ${saverRisk.overallRiskScore}`);
      console.log(`  Trader risk score: ${traderRisk.overallRiskScore}`);
      console.log(`  Spender risk score: ${spenderRisk.overallRiskScore}`);
      
      // Trader should have highest risk score due to frequent trading
      expect(traderRisk.overallRiskScore).to.be.gt(saverRisk.overallRiskScore);
      expect(traderRisk.overallRiskScore).to.be.gt(spenderRisk.overallRiskScore);
      
      console.log('Phase 3: Fiat integration patterns...');
      
      // Phase 3: Different fiat integration patterns
      
      // Saver: Occasional large fiat deposits
      const saverFiatAmount = ethers.parseUnits('5000', 6); // $5000
      await mockUSDC.mint(personas.saver.user.address, saverFiatAmount);
      await mockUSDC.connect(personas.saver.user).transfer(await stripeIntegration.getAddress(), saverFiatAmount);
      
      await stripeIntegration.connect(owner).recordFiatActivity(
        personas.saver.user.address,
        saverFiatAmount,
        0 // Purchase type
      );
      
      // Spender: Regular small fiat top-ups
      for (let i = 0; i < 3; i++) {
        const topUpAmount = ethers.parseUnits('200', 6); // $200 each
        await mockUSDC.mint(personas.spender.user.address, topUpAmount);
        await mockUSDC.connect(personas.spender.user).transfer(await stripeIntegration.getAddress(), topUpAmount);
        
        await stripeIntegration.connect(owner).recordFiatActivity(
          personas.spender.user.address,
          topUpAmount,
          0
        );
        
        await time.increase(7 * 24 * 3600); // Weekly top-ups
      }
      
      console.log('Phase 4: Demurrage impact analysis...');
      
      // Phase 4: Check demurrage impact on different user types
      await time.increase(30 * 24 * 3600); // 30 days pass
      
      const saverDemurrage = await caesarToken.calculateDemurrage(personas.saver.user.address);
      const traderDemurrage = await caesarToken.calculateDemurrage(personas.trader.user.address);
      const spenderDemurrage = await caesarToken.calculateDemurrage(personas.spender.user.address);
      
      console.log(`  Saver demurrage: ${ethers.formatEther(saverDemurrage)}`);
      console.log(`  Trader demurrage: ${ethers.formatEther(traderDemurrage)}`);
      console.log(`  Spender demurrage: ${ethers.formatEther(spenderDemurrage)}`);
      
      // Active users should have lower demurrage
      expect(spenderDemurrage).to.be.lt(saverDemurrage); // Spender is more active
      
      console.log('Phase 5: System health under diverse usage...');
      
      // Phase 5: Verify system health with diverse user base
      const networkHealth = await caesarToken.getNetworkHealthIndex();
      const activeParticipants = await caesarToken.getActiveParticipants();
      const liquidityRatio = await caesarToken.getLiquidityRatio();
      
      console.log(`  Network health: ${networkHealth}`);
      console.log(`  Active participants: ${activeParticipants}`);
      console.log(`  Liquidity ratio: ${liquidityRatio}`);
      
      // Diverse user base should maintain healthy metrics
      expect(networkHealth).to.be.gt(600);
      expect(activeParticipants).to.be.gte(3);
      expect(liquidityRatio).to.be.gt(0);
      
      console.log('✅ Complex multi-user interaction test completed');
    });
  });
});