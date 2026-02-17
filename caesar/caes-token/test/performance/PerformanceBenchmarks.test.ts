// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { expect } from 'chai';
import { ethers } from 'hardhat';
import { loadFixture, time } from '@nomicfoundation/hardhat-network-helpers';
import { deployCaesarFixture, deployStressTestFixture, deployPerformanceTestFixture } from '../fixtures/TestFixtures';

describe('Performance Benchmarks', function () {
  // Extend timeout for performance tests
  this.timeout(300000); // 5 minutes
  
  describe('Transaction Throughput Tests', function () {
    it('Should handle high-volume sequential transactions', async function () {
      const fixture = await loadFixture(deployStressTestFixture);
      const { caesarToken, testUsers, owner } = fixture;
      
      console.log('📊 Testing sequential transaction throughput...');
      
      const transactionCount = 100;
      const transferAmount = ethers.parseEther('10');
      
      const startTime = Date.now();
      const gasUsed = [];
      
      // Execute sequential transactions
      for (let i = 0; i < transactionCount; i++) {
        const fromUser = testUsers[i % testUsers.length];
        const toUser = testUsers[(i + 1) % testUsers.length];
        
        const tx = await caesarToken.connect(fromUser).transfer(toUser.address, transferAmount);
        const receipt = await tx.wait();
        
        gasUsed.push(receipt!.gasUsed);
        
        if (i % 20 === 0) {
          console.log(`  Completed ${i + 1}/${transactionCount} transactions...`);
        }
      }
      
      const endTime = Date.now();
      const totalTime = endTime - startTime;
      const tps = (transactionCount * 1000) / totalTime;
      
      console.log(`📈 Results:`);
      console.log(`  Total time: ${totalTime}ms`);
      console.log(`  Transactions per second: ${tps.toFixed(2)} TPS`);
      console.log(`  Average gas per transaction: ${(gasUsed.reduce((a, b) => a + b, 0n) / BigInt(gasUsed.length)).toString()}`);
      console.log(`  Min gas: ${Math.min(...gasUsed.map(g => Number(g)))}`);
      console.log(`  Max gas: ${Math.max(...gasUsed.map(g => Number(g)))}`);
      
      // Performance targets
      expect(tps).to.be.gte(10); // Minimum 10 TPS for sequential transactions
      
      // Gas efficiency targets
      const avgGas = gasUsed.reduce((a, b) => a + b, 0n) / BigInt(gasUsed.length);
      expect(Number(avgGas)).to.be.lte(100000); // Max 100k gas per transaction
    });

    it('Should maintain performance under concurrent load simulation', async function () {
      const fixture = await loadFixture(deployStressTestFixture);
      const { caesarToken, testUsers } = fixture;
      
      console.log('🔄 Testing concurrent transaction simulation...');
      
      const batchSize = 10;
      const batchCount = 5;
      const transferAmount = ethers.parseEther('5');
      
      const startTime = Date.now();
      const allGasUsed = [];
      
      // Execute batches of concurrent transactions
      for (let batch = 0; batch < batchCount; batch++) {
        console.log(`  Processing batch ${batch + 1}/${batchCount}...`);
        
        const batchPromises = [];
        
        for (let i = 0; i < batchSize; i++) {
          const fromUser = testUsers[(batch * batchSize + i) % testUsers.length];
          const toUser = testUsers[(batch * batchSize + i + 1) % testUsers.length];
          
          batchPromises.push(
            caesarToken.connect(fromUser).transfer(toUser.address, transferAmount)
          );
        }
        
        // Wait for all transactions in batch to complete
        const batchTxs = await Promise.all(batchPromises);
        const batchReceipts = await Promise.all(batchTxs.map(tx => tx.wait()));
        
        // Collect gas usage
        batchReceipts.forEach(receipt => {
          if (receipt) allGasUsed.push(receipt.gasUsed);
        });
        
        // Small delay between batches
        await time.increase(1);
      }
      
      const endTime = Date.now();
      const totalTime = endTime - startTime;
      const totalTransactions = batchSize * batchCount;
      const tps = (totalTransactions * 1000) / totalTime;
      
      console.log(`📈 Concurrent Load Results:`);
      console.log(`  Total transactions: ${totalTransactions}`);
      console.log(`  Total time: ${totalTime}ms`);
      console.log(`  Effective TPS: ${tps.toFixed(2)}`);
      console.log(`  Average gas: ${(allGasUsed.reduce((a, b) => a + b, 0n) / BigInt(allGasUsed.length)).toString()}`);
      
      // Performance targets for concurrent load
      expect(tps).to.be.gte(5); // Lower threshold for concurrent simulation
      
      const avgGas = allGasUsed.reduce((a, b) => a + b, 0n) / BigInt(allGasUsed.length);
      expect(Number(avgGas)).to.be.lte(120000); // Slightly higher gas limit for concurrent operations
    });

    it('Should optimize gas usage for complex economic operations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, economicEngine, antiSpeculationEngine, user1, user2, owner } = fixture;
      
      console.log('⛽ Testing gas optimization for complex operations...');
      
      // Setup users with balances
      await caesarToken.transfer(user1.address, ethers.parseEther('50000'));
      await caesarToken.transfer(user2.address, ethers.parseEther('50000'));
      
      const operations = [
        {
          name: 'Simple Transfer',
          operation: () => caesarToken.connect(user1).transfer(user2.address, ethers.parseEther('100'))
        },
        {
          name: 'Transfer with Demurrage Calculation',
          operation: async () => {
            // Move time forward to trigger demurrage
            await time.increase(86400);
            return caesarToken.connect(user1).transfer(user2.address, ethers.parseEther('100'));
          }
        },
        {
          name: 'Calculate Demurrage Only',
          operation: () => caesar.calculateDemurrage(user1.address)
        },
        {
          name: 'Calculate Speculation Penalty',
          operation: () => caesar.calculateSpeculationPenalty(user1.address)
        },
        {
          name: 'Economic Health Check',
          operation: () => economicEngine.monitorEconomicHealth()
        },
        {
          name: 'Risk Profile Analysis',
          operation: () => antiSpeculationEngine.getAccountRiskProfile(user1.address)
        }
      ];
      
      const gasResults = [];
      
      for (const op of operations) {
        console.log(`  Testing ${op.name}...`);
        
        const tx = await op.operation();
        let gasUsed: bigint;
        
        if (tx && typeof tx.wait === 'function') {
          const receipt = await tx.wait();
          gasUsed = receipt!.gasUsed;
        } else {
          // For view functions, estimate gas
          gasUsed = 0n; // View functions don't consume gas
        }
        
        gasResults.push({
          operation: op.name,
          gasUsed: gasUsed
        });
        
        console.log(`    Gas used: ${gasUsed.toString()}`);
      }
      
      console.log(`📊 Gas Usage Summary:`);
      gasResults.forEach(result => {
        console.log(`  ${result.operation}: ${result.gasUsed} gas`);
      });
      
      // Gas efficiency targets
      const simpleTransfer = gasResults.find(r => r.operation === 'Simple Transfer');
      if (simpleTransfer) {
        expect(Number(simpleTransfer.gasUsed)).to.be.lte(80000); // Simple transfer should be under 80k gas
      }
      
      const complexTransfer = gasResults.find(r => r.operation === 'Transfer with Demurrage Calculation');
      if (complexTransfer) {
        expect(Number(complexTransfer.gasUsed)).to.be.lte(200000); // Complex transfer under 200k gas
      }
    });
  });

  describe('Memory and State Management', function () {
    it('Should efficiently handle large-scale state updates', async function () {
      const fixture = await loadFixture(deployStressTestFixture);
      const { caesarToken, economicEngine, testUsers, owner } = fixture;
      
      console.log('💾 Testing large-scale state management...');
      
      const userCount = Math.min(testUsers.length, 50); // Limit for test efficiency
      const operationsPerUser = 5;
      
      console.log(`  Processing ${userCount} users with ${operationsPerUser} operations each...`);
      
      const startTime = Date.now();
      
      // Create activity for many users
      for (let i = 0; i < userCount; i++) {
        const user = testUsers[i];
        
        for (let j = 0; j < operationsPerUser; j++) {
          const targetUser = testUsers[(i + j + 1) % userCount];
          await caesarToken.connect(user).transfer(targetUser.address, ethers.parseEther('10'));
        }
        
        if (i % 10 === 0) {
          console.log(`    Processed ${i}/${userCount} users...`);
        }
      }
      
      console.log('  Calculating network health metrics...');
      
      // Test state query performance with large dataset
      const healthCheckStart = Date.now();
      const networkHealth = await caesarToken.getNetworkHealthIndex();
      const healthCheckTime = Date.now() - healthCheckStart;
      
      const activeParticipants = await caesarToken.getActiveParticipants();
      const liquidityRatio = await caesarToken.getLiquidityRatio();
      
      const totalTime = Date.now() - startTime;
      
      console.log(`📈 State Management Results:`);
      console.log(`  Total operations: ${userCount * operationsPerUser}`);
      console.log(`  Total time: ${totalTime}ms`);
      console.log(`  Health check time: ${healthCheckTime}ms`);
      console.log(`  Network health: ${networkHealth}`);
      console.log(`  Active participants: ${activeParticipants}`);
      console.log(`  Liquidity ratio: ${liquidityRatio}`);
      
      // Performance targets
      expect(totalTime).to.be.lte(60000); // Should complete within 60 seconds
      expect(healthCheckTime).to.be.lte(5000); // Health check under 5 seconds
      expect(networkHealth).to.be.gt(0); // Should maintain positive health
    });

    it('Should optimize storage for economic parameter updates', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, owner } = fixture;
      
      console.log('🔧 Testing economic parameter update efficiency...');
      
      const parameterSets = [
        {
          name: 'Conservative',
          params: {
            baseDemurrageRate: 25n,
            maxDemurrageRate: 100n,
            stabilityThreshold: 50n,
            fiatDiscountFactor: 3000n,
            gracePeriodsHours: 72n,
            interventionThreshold: 300n,
            rebalanceFrequency: 7200n,
            emergencyThreshold: 800n
          }
        },
        {
          name: 'Moderate',
          params: {
            baseDemurrageRate: 50n,
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
          name: 'Aggressive',
          params: {
            baseDemurrageRate: 100n,
            maxDemurrageRate: 400n,
            stabilityThreshold: 200n,
            fiatDiscountFactor: 7500n,
            gracePeriodsHours: 24n,
            interventionThreshold: 800n,
            rebalanceFrequency: 1800n,
            emergencyThreshold: 1500n
          }
        }
      ];
      
      const gasUsageResults = [];
      
      for (const paramSet of parameterSets) {
        console.log(`  Testing ${paramSet.name} parameters...`);
        
        const tx = await economicEngine.connect(owner).updateEconomicParameters(paramSet.params);
        const receipt = await tx.wait();
        
        gasUsageResults.push({
          name: paramSet.name,
          gasUsed: receipt!.gasUsed
        });
        
        // Verify parameters were set correctly
        const updatedParams = await economicEngine.getEconomicParameters();
        expect(updatedParams.baseDemurrageRate).to.equal(paramSet.params.baseDemurrageRate);
        
        console.log(`    Gas used: ${receipt!.gasUsed}`);
      }
      
      console.log(`📊 Parameter Update Gas Usage:`);
      gasUsageResults.forEach(result => {
        console.log(`  ${result.name}: ${result.gasUsed} gas`);
      });
      
      // All parameter updates should use similar gas
      const maxGas = Math.max(...gasUsageResults.map(r => Number(r.gasUsed)));
      const minGas = Math.min(...gasUsageResults.map(r => Number(r.gasUsed)));
      
      expect(maxGas).to.be.lte(100000); // Parameter updates should be under 100k gas
      expect(maxGas - minGas).to.be.lte(20000); // Gas usage should be consistent (within 20k)
    });
  });

  describe('Cross-Chain Performance', function () {
    it('Should benchmark cross-chain message processing', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, crossChainSync, user1, owner } = fixture;
      
      console.log('🌐 Testing cross-chain message performance...');
      
      const messageCount = 10;
      const messageData = ethers.AbiCoder.defaultAbiCoder().encode(
        ['address', 'uint256', 'uint256'],
        [user1.address, ethers.parseEther('1000'), 1] // address, amount, operation type
      );
      
      const processingTimes = [];
      const gasUsed = [];
      
      for (let i = 0; i < messageCount; i++) {
        console.log(`  Processing cross-chain message ${i + 1}/${messageCount}...`);
        
        const startTime = Date.now();
        
        const tx = await crossChainSync.connect(owner).processInboundMessage(
          137, // Polygon chain ID
          messageData,
          i + 1 // nonce
        );
        
        const receipt = await tx.wait();
        const processingTime = Date.now() - startTime;
        
        processingTimes.push(processingTime);
        gasUsed.push(receipt!.gasUsed);
      }
      
      const avgProcessingTime = processingTimes.reduce((a, b) => a + b, 0) / processingTimes.length;
      const avgGasUsed = gasUsed.reduce((a, b) => a + b, 0n) / BigInt(gasUsed.length);
      
      console.log(`📈 Cross-Chain Performance Results:`);
      console.log(`  Messages processed: ${messageCount}`);
      console.log(`  Average processing time: ${avgProcessingTime.toFixed(2)}ms`);
      console.log(`  Average gas used: ${avgGasUsed.toString()}`);
      console.log(`  Min processing time: ${Math.min(...processingTimes)}ms`);
      console.log(`  Max processing time: ${Math.max(...processingTimes)}ms`);
      
      // Performance targets for cross-chain operations
      expect(avgProcessingTime).to.be.lte(3000); // Average under 3 seconds
      expect(Number(avgGasUsed)).to.be.lte(150000); // Average gas under 150k
    });

    it('Should handle cross-chain parameter synchronization efficiently', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { economicEngine, crossChainSync, owner } = fixture;
      
      console.log('🔄 Testing cross-chain parameter sync performance...');
      
      const chainIds = [1, 137, 42161, 10, 56]; // Ethereum, Polygon, Arbitrum, Optimism, BSC
      const newParams = {
        baseDemurrageRate: 75n,
        maxDemurrageRate: 250n,
        stabilityThreshold: 125n,
        fiatDiscountFactor: 6000n,
        gracePeriodsHours: 60n,
        interventionThreshold: 600n,
        rebalanceFrequency: 2400n,
        emergencyThreshold: 1200n
      };
      
      console.log(`  Syncing parameters to ${chainIds.length} chains...`);
      
      const syncStart = Date.now();
      const syncGasUsed = [];
      
      // Update parameters locally first
      const updateTx = await economicEngine.connect(owner).updateEconomicParameters(newParams);
      const updateReceipt = await updateTx.wait();
      
      console.log(`  Local update gas used: ${updateReceipt!.gasUsed}`);
      
      // Simulate cross-chain sync to each chain
      for (let i = 0; i < chainIds.length; i++) {
        const chainId = chainIds[i];
        console.log(`    Syncing to chain ${chainId}...`);
        
        const syncTx = await crossChainSync.connect(owner).syncEconomicParameters(
          chainId,
          newParams
        );
        
        const syncReceipt = await syncTx.wait();
        syncGasUsed.push(syncReceipt!.gasUsed);
      }
      
      const totalSyncTime = Date.now() - syncStart;
      const avgSyncGas = syncGasUsed.reduce((a, b) => a + b, 0n) / BigInt(syncGasUsed.length);
      
      console.log(`📊 Cross-Chain Sync Results:`);
      console.log(`  Chains synced: ${chainIds.length}`);
      console.log(`  Total sync time: ${totalSyncTime}ms`);
      console.log(`  Average sync gas per chain: ${avgSyncGas.toString()}`);
      console.log(`  Total gas for all chains: ${syncGasUsed.reduce((a, b) => a + b, 0n).toString()}`);
      
      // Performance targets
      expect(totalSyncTime).to.be.lte(30000); // All syncs under 30 seconds
      expect(Number(avgSyncGas)).to.be.lte(200000); // Sync gas under 200k per chain
    });
  });

  describe('Load Testing and Scalability', function () {
    it('Should handle peak usage scenarios', async function () {
      const fixture = await loadFixture(deployStressTestFixture);
      const { caesarToken, stripeIntegration, mockUSDC, testUsers, owner } = fixture;
      
      console.log('🚀 Testing peak usage scalability...');
      
      const peakUsers = Math.min(testUsers.length, 20); // Limit for test efficiency
      const transactionsPerUser = 10;
      const fiatOperationsPerUser = 3;
      
      console.log(`  Simulating peak load: ${peakUsers} users, ${transactionsPerUser + fiatOperationsPerUser} ops each...`);
      
      const startTime = Date.now();
      const allOperations = [];
      
      // Create mixed workload of crypto and fiat operations
      for (let i = 0; i < peakUsers; i++) {
        const user = testUsers[i];
        
        // Crypto operations
        for (let j = 0; j < transactionsPerUser; j++) {
          const targetUser = testUsers[(i + j + 1) % peakUsers];
          allOperations.push(
            caesarToken.connect(user).transfer(targetUser.address, ethers.parseEther('25'))
          );
        }
        
        // Fiat operations
        for (let k = 0; k < fiatOperationsPerUser; k++) {
          const fiatAmount = ethers.parseUnits('100', 6);
          await mockUSDC.mint(user.address, fiatAmount);
          
          allOperations.push(
            stripeIntegration.connect(owner).recordFiatDeposit(
              user.address,
              `peak_load_${i}_${k}`,
              fiatAmount
            )
          );
        }
        
        if (i % 5 === 0) {
          console.log(`    Queued operations for ${i}/${peakUsers} users...`);
        }
      }
      
      console.log(`  Executing ${allOperations.length} operations...`);
      
      // Execute all operations
      const operationPromises = allOperations.map(async (op, index) => {
        try {
          const tx = await op;
          const receipt = await tx.wait();
          return { success: true, gasUsed: receipt!.gasUsed, index };
        } catch (error) {
          console.error(`Operation ${index} failed:`, error);
          return { success: false, gasUsed: 0n, index };
        }
      });
      
      const results = await Promise.allSettled(operationPromises);
      
      const totalTime = Date.now() - startTime;
      const successfulOps = results.filter(r => r.status === 'fulfilled' && r.value.success).length;
      const totalGasUsed = results
        .filter(r => r.status === 'fulfilled' && r.value.success)
        .reduce((sum, r) => sum + (r.value as any).gasUsed, 0n);
      
      const effectiveTps = (successfulOps * 1000) / totalTime;
      
      console.log(`📈 Peak Load Results:`);
      console.log(`  Total operations: ${allOperations.length}`);
      console.log(`  Successful operations: ${successfulOps}`);
      console.log(`  Success rate: ${((successfulOps / allOperations.length) * 100).toFixed(2)}%`);
      console.log(`  Total time: ${totalTime}ms`);
      console.log(`  Effective TPS: ${effectiveTps.toFixed(2)}`);
      console.log(`  Total gas used: ${totalGasUsed.toString()}`);
      console.log(`  Average gas per operation: ${(totalGasUsed / BigInt(successfulOps)).toString()}`);
      
      // Scalability targets
      expect(successfulOps / allOperations.length).to.be.gte(0.95); // 95% success rate
      expect(effectiveTps).to.be.gte(3); // Minimum 3 effective TPS under peak load
      expect(totalTime).to.be.lte(120000); // Complete within 2 minutes
    });

    it('Should maintain consistent performance under sustained load', async function () {
      const fixture = await loadFixture(deployPerformanceTestFixture);
      const { caesarToken, user1, user2, user3 } = fixture;
      
      console.log('⏱️  Testing sustained load performance...');
      
      const users = [user1, user2, user3];
      const rounds = 5;
      const transactionsPerRound = 20;
      const transferAmount = ethers.parseEther('50');
      
      const roundResults = [];
      
      for (let round = 0; round < rounds; round++) {
        console.log(`  Round ${round + 1}/${rounds}...`);
        
        const roundStart = Date.now();
        const roundGasUsed = [];
        
        for (let i = 0; i < transactionsPerRound; i++) {
          const fromUser = users[i % users.length];
          const toUser = users[(i + 1) % users.length];
          
          const tx = await caesarToken.connect(fromUser).transfer(toUser.address, transferAmount);
          const receipt = await tx.wait();
          
          roundGasUsed.push(receipt!.gasUsed);
        }
        
        const roundTime = Date.now() - roundStart;
        const roundTps = (transactionsPerRound * 1000) / roundTime;
        const avgGas = roundGasUsed.reduce((a, b) => a + b, 0n) / BigInt(roundGasUsed.length);
        
        roundResults.push({
          round: round + 1,
          time: roundTime,
          tps: roundTps,
          avgGas: Number(avgGas)
        });
        
        console.log(`    Round ${round + 1}: ${roundTps.toFixed(2)} TPS, ${avgGas} avg gas`);
        
        // Short break between rounds
        await time.increase(60); // 1 minute
      }
      
      console.log(`📊 Sustained Load Analysis:`);
      
      const avgTps = roundResults.reduce((sum, r) => sum + r.tps, 0) / roundResults.length;
      const tpsVariance = roundResults.reduce((sum, r) => sum + Math.pow(r.tps - avgTps, 2), 0) / roundResults.length;
      const tpsStdDev = Math.sqrt(tpsVariance);
      
      const avgGas = roundResults.reduce((sum, r) => sum + r.avgGas, 0) / roundResults.length;
      const gasVariance = roundResults.reduce((sum, r) => sum + Math.pow(r.avgGas - avgGas, 2), 0) / roundResults.length;
      const gasStdDev = Math.sqrt(gasVariance);
      
      console.log(`  Average TPS: ${avgTps.toFixed(2)} ± ${tpsStdDev.toFixed(2)}`);
      console.log(`  Average Gas: ${avgGas.toFixed(0)} ± ${gasStdDev.toFixed(0)}`);
      console.log(`  TPS Coefficient of Variation: ${((tpsStdDev / avgTps) * 100).toFixed(2)}%`);
      console.log(`  Gas Coefficient of Variation: ${((gasStdDev / avgGas) * 100).toFixed(2)}%`);
      
      // Performance consistency targets
      expect(avgTps).to.be.gte(8); // Minimum 8 TPS average
      expect(tpsStdDev / avgTps).to.be.lte(0.3); // TPS variation under 30%
      expect(gasStdDev / avgGas).to.be.lte(0.2); // Gas variation under 20%
    });
  });

  describe('Economic Model Performance', function () {
    it('Should efficiently process complex economic calculations', async function () {
      const fixture = await loadFixture(deployCaesarFixture);
      const { caesarToken, economicEngine, antiSpeculationEngine, demurrageManager, user1 } = fixture;
      
      console.log('💰 Testing economic calculation performance...');
      
      // Setup complex scenario
      await caesarToken.transfer(user1.address, ethers.parseEther('100000'));
      
      // Create transaction history for complex calculations
      for (let i = 0; i < 20; i++) {
        await caesarToken.connect(user1).transfer(user1.address, ethers.parseEther('100'));
        await time.increase(3600); // 1 hour between transactions
      }
      
      const calculations = [
        {
          name: 'Basic Demurrage',
          operation: () => caesar.calculateDemurrage(user1.address)
        },
        {
          name: 'Advanced Demurrage with History',
          operation: () => economicEngine.calculateDemurrage(user1.address, ethers.parseEther('50000'))
        },
        {
          name: 'Risk Profile Analysis',
          operation: () => antiSpeculationEngine.getAccountRiskProfile(user1.address)
        },
        {
          name: 'Network Health Index',
          operation: () => caesar.getNetworkHealthIndex()
        },
        {
          name: 'Comprehensive Health Monitoring',
          operation: () => economicEngine.monitorEconomicHealth()
        },
        {
          name: 'Fiat Activity Analysis',
          operation: () => demurrageManager.getFiatActivityData(user1.address)
        }
      ];
      
      const calculationResults = [];
      
      for (const calc of calculations) {
        console.log(`  Testing ${calc.name}...`);
        
        const iterations = 5;
        const times = [];
        
        for (let i = 0; i < iterations; i++) {
          const start = Date.now();
          await calc.operation();
          const time = Date.now() - start;
          times.push(time);
        }
        
        const avgTime = times.reduce((a, b) => a + b, 0) / times.length;
        const minTime = Math.min(...times);
        const maxTime = Math.max(...times);
        
        calculationResults.push({
          name: calc.name,
          avgTime,
          minTime,
          maxTime
        });
        
        console.log(`    Average: ${avgTime.toFixed(2)}ms (${minTime}-${maxTime}ms)`);
      }
      
      console.log(`📊 Economic Calculation Performance:`);
      calculationResults.forEach(result => {
        console.log(`  ${result.name}: ${result.avgTime.toFixed(2)}ms average`);
      });
      
      // Performance targets for economic calculations
      calculationResults.forEach(result => {
        expect(result.avgTime).to.be.lte(1000); // All calculations under 1 second
        expect(result.maxTime).to.be.lte(2000); // Worst case under 2 seconds
      });
      
      // Complex calculations should still be reasonable
      const complexCalcs = calculationResults.filter(r => 
        r.name.includes('Advanced') || r.name.includes('Comprehensive')
      );
      
      complexCalcs.forEach(result => {
        expect(result.avgTime).to.be.lte(500); // Complex calculations under 500ms
      });
    });
  });
});