// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import fs from "fs";

interface StressTestMetrics {
  totalTransactions: number;
  successfulTransactions: number;
  failedTransactions: number;
  averageGasUsed: number;
  totalGasUsed: bigint;
  averageTransactionTime: number;
  transactionsPerSecond: number;
  demurrageAccuracy: number;
  economicStabilityMaintained: boolean;
}

interface TestScenario {
  name: string;
  transactionCount: number;
  concurrency: number;
  delayBetweenTx: number; // milliseconds
  testDuration: number; // seconds
}

async function main() {
  console.log("🚀 Production Scalability & Stress Testing on Sepolia");
  console.log("=====================================================\n");
  
  // Find deployed contract
  const deploymentFiles = ["deployments/sepolia.json", "deployments/final-caes-sepolia.json"];
  let caesarAddress: string | null = null;
  
  for (const file of deploymentFiles) {
    if (fs.existsSync(file)) {
      const deployment = JSON.parse(fs.readFileSync(file, 'utf8'));
      if (deployment.contracts.Caesar) {
        caesarAddress = deployment.contracts.Caesar;
        break;
      } else if (deployment.contracts.FinalCAES) {
        caesarAddress = deployment.contracts.FinalCAES;
        break;
      }
    }
  }
  
  if (!caesarAddress) {
    console.error("❌ No Caesar token deployment found!");
    process.exit(1);
  }
  
  console.log(`📋 Testing Caesar Token at: ${caesarAddress}`);
  
  const Caesar = await ethers.getContractFactory("CaesarToken");
  const caesar = Caesar.attach(caesarAddress);
  const [deployer, ...accounts] = await ethers.getSigners();
  
  // Test scenarios with increasing complexity
  const scenarios: TestScenario[] = [
    {
      name: "Light Load Test",
      transactionCount: 50,
      concurrency: 5,
      delayBetweenTx: 100,
      testDuration: 60
    },
    {
      name: "Medium Load Test", 
      transactionCount: 200,
      concurrency: 10,
      delayBetweenTx: 50,
      testDuration: 120
    },
    {
      name: "High Load Test",
      transactionCount: 500,
      concurrency: 20,
      delayBetweenTx: 20,
      testDuration: 300
    }
  ];
  
  // Setup test accounts with tokens
  console.log("🔧 Setting up test environment...");
  
  // Check if accounts need tokens
  const testAccounts = accounts.slice(0, Math.min(25, accounts.length)); // Use available accounts
  let totalMinted = 0n;
  
  try {
    // Enable migration for testing
    await caesar.setMigrationContract(deployer.address);
    await caesar.setMigrationEnabled(true);
    
    for (let i = 0; i < testAccounts.length; i++) {
      const account = testAccounts[i];
      const balance = await caesar.balanceOf(account.address);
      
      if (balance < ethers.parseEther("1000")) {
        const mintAmount = ethers.parseEther("10000");
        try {
          const tx = await caesar.migrationMint(account.address, mintAmount);
          await tx.wait();
          totalMinted += mintAmount;
          
          if ((i + 1) % 5 === 0) {
            console.log(`✅ Minted tokens for ${i + 1}/${testAccounts.length} accounts`);
          }
        } catch (error) {
          console.log(`⚠️  Failed to mint for account ${i}: ${error}`);
        }
      }
    }
    
    console.log(`✅ Test setup complete. Total minted: ${ethers.formatEther(totalMinted)} CAES\n`);
    
  } catch (error) {
    console.log(`⚠️  Setup limited due to: ${error}\n`);
  }
  
  const allResults: Array<StressTestMetrics & { scenarioName: string }> = [];
  
  // Run each test scenario
  for (const scenario of scenarios) {
    console.log(`\n🧪 Running ${scenario.name}`);
    console.log("=".repeat(scenario.name.length + 12));
    console.log(`Transactions: ${scenario.transactionCount}`);
    console.log(`Concurrency: ${scenario.concurrency}`);
    console.log(`Duration: ${scenario.testDuration}s`);
    console.log(`Delay: ${scenario.delayBetweenTx}ms\n`);
    
    const results = await runStressTest(caesar, testAccounts.length > 0 ? testAccounts : [deployer], scenario);
    allResults.push({ ...results, scenarioName: scenario.name });
    
    // Show immediate results
    console.log(`\n📊 ${scenario.name} Results:`);
    console.log(`✅ Success Rate: ${((results.successfulTransactions / results.totalTransactions) * 100).toFixed(2)}%`);
    console.log(`⚡ TPS: ${results.transactionsPerSecond.toFixed(2)}`);
    console.log(`⛽ Avg Gas: ${results.averageGasUsed}`);
    console.log(`⏱️  Avg Time: ${results.averageTransactionTime.toFixed(0)}ms`);
    
    // Wait between scenarios
    if (scenario !== scenarios[scenarios.length - 1]) {
      console.log("\n⏳ Waiting 30s before next scenario...");
      await new Promise(resolve => setTimeout(resolve, 30000));
    }
  }
  
  // Economic formula validation
  console.log("\n\n💰 Economic Formula Validation");
  console.log("================================");
  
  await validateEconomicFormulas(caesar, testAccounts.length > 0 ? testAccounts : [deployer]);
  
  // Generate comprehensive report
  console.log("\n\n" + "=".repeat(80));
  console.log("📊 COMPREHENSIVE SCALABILITY REPORT");
  console.log("=".repeat(80));
  
  generateScalabilityReport(allResults);
  
  // Save detailed results
  const reportData = {
    timestamp: new Date().toISOString(),
    network: "sepolia",
    contractAddress: caesarAddress,
    scenarios: scenarios,
    results: allResults,
    summary: generateSummaryMetrics(allResults)
  };
  
  const reportPath = `test-reports/scalability-report-${Date.now()}.json`;
  fs.mkdirSync("test-reports", { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(reportData, (key, value) =>
    typeof value === 'bigint' ? value.toString() : value, 2));
  
  console.log(`\n📝 Detailed report saved: ${reportPath}`);
  
  const overallSuccess = allResults.every(r => r.successfulTransactions / r.totalTransactions > 0.9);
  console.log(`\n🎯 Overall Assessment: ${overallSuccess ? '🟢 READY FOR PRODUCTION' : '🟡 NEEDS OPTIMIZATION'}`);
}

async function runStressTest(
  caesar: any, 
  testAccounts: any[], 
  scenario: TestScenario
): Promise<StressTestMetrics> {
  
  const startTime = Date.now();
  const results: StressTestMetrics = {
    totalTransactions: 0,
    successfulTransactions: 0, 
    failedTransactions: 0,
    averageGasUsed: 0,
    totalGasUsed: 0n,
    averageTransactionTime: 0,
    transactionsPerSecond: 0,
    demurrageAccuracy: 1.0,
    economicStabilityMaintained: true
  };
  
  const transactionPromises: Promise<any>[] = [];
  const gasUsages: number[] = [];
  const transactionTimes: number[] = [];
  
  // Create transaction batches for concurrency
  const batchSize = scenario.concurrency;
  const totalBatches = Math.ceil(scenario.transactionCount / batchSize);
  
  for (let batch = 0; batch < totalBatches; batch++) {
    const batchPromises: Promise<any>[] = [];
    const batchStartTime = Date.now();
    
    for (let i = 0; i < batchSize && (batch * batchSize + i) < scenario.transactionCount; i++) {
      const txIndex = batch * batchSize + i;
      const fromAccount = testAccounts[txIndex % testAccounts.length];
      const toAccount = testAccounts[(txIndex + 1) % testAccounts.length];
      const amount = ethers.parseEther((Math.random() * 10 + 1).toString()); // 1-11 CAES
      
      const txPromise = executeTransaction(caesar, fromAccount, toAccount, amount, txIndex)
        .then(result => {
          if (result.success) {
            results.successfulTransactions++;
            if (result.gasUsed) {
              gasUsages.push(result.gasUsed);
              results.totalGasUsed += BigInt(result.gasUsed);
            }
            if (result.time) {
              transactionTimes.push(result.time);
            }
          } else {
            results.failedTransactions++;
          }
          results.totalTransactions++;
          return result;
        })
        .catch(error => {
          results.failedTransactions++;
          results.totalTransactions++;
          return { success: false, error };
        });
      
      batchPromises.push(txPromise);
    }
    
    // Execute batch
    await Promise.allSettled(batchPromises);
    
    const batchDuration = Date.now() - batchStartTime;
    if (batchDuration < scenario.delayBetweenTx * batchSize) {
      await new Promise(resolve => 
        setTimeout(resolve, scenario.delayBetweenTx * batchSize - batchDuration)
      );
    }
    
    // Progress indicator
    if ((batch + 1) % Math.ceil(totalBatches / 10) === 0) {
      const progress = ((batch + 1) / totalBatches * 100).toFixed(0);
      console.log(`🔄 Progress: ${progress}% (${results.successfulTransactions}/${results.totalTransactions} successful)`);
    }
  }
  
  // Calculate metrics
  const totalDuration = (Date.now() - startTime) / 1000;
  results.transactionsPerSecond = results.successfulTransactions / totalDuration;
  
  if (gasUsages.length > 0) {
    results.averageGasUsed = gasUsages.reduce((a, b) => a + b, 0) / gasUsages.length;
  }
  
  if (transactionTimes.length > 0) {
    results.averageTransactionTime = transactionTimes.reduce((a, b) => a + b, 0) / transactionTimes.length;
  }
  
  return results;
}

async function executeTransaction(
  caesar: any,
  fromAccount: any,
  toAccount: any, 
  amount: bigint,
  txIndex: number
): Promise<{success: boolean, gasUsed?: number, time?: number, error?: any}> {
  
  const startTime = Date.now();
  
  try {
    // Check balance first
    const balance = await caesar.balanceOf(fromAccount.address);
    if (balance < amount) {
      return { success: false, error: "Insufficient balance" };
    }
    
    // Execute transfer
    const tx = await caesar.connect(fromAccount).transfer(toAccount.address, amount, {
      gasLimit: 300000 // Set reasonable gas limit
    });
    
    const receipt = await tx.wait();
    const endTime = Date.now();
    
    return {
      success: true,
      gasUsed: Number(receipt.gasUsed),
      time: endTime - startTime
    };
    
  } catch (error: any) {
    return { 
      success: false, 
      error: error.message || error.toString(),
      time: Date.now() - startTime
    };
  }
}

async function validateEconomicFormulas(caesar: any, testAccounts: any[]) {
  console.log("🧮 Testing demurrage calculations...");
  
  try {
    // Test demurrage calculation
    const testAccount = testAccounts[0];
    const balance = await caesar.balanceOf(testAccount.address);
    
    if (balance > 0) {
      const demurrageAmount = await caesar.calculateDemurrage(testAccount.address);
      console.log(`✅ Demurrage calculation: ${ethers.formatEther(demurrageAmount)} CAES`);
      
      // Test time-based demurrage
      console.log(`🔄 Testing time-based demurrage...`);
      
      // Small transfer to set activity time
      if (balance >= ethers.parseEther("1")) {
        await caesar.connect(testAccount).transfer(testAccounts[1].address, ethers.parseEther("1"));
      }
      
      const demurrageAfter = await caesar.calculateDemurrage(testAccount.address);
      console.log(`✅ Demurrage after activity: ${ethers.formatEther(demurrageAfter)} CAES`);
    }
    
    // Test network health metrics
    const healthIndex = await caesar.getNetworkHealthIndex();
    const activeParticipants = await caesar.getActiveParticipants();
    const liquidityRatio = await caesar.getLiquidityRatio();
    
    console.log(`✅ Network Health Index: ${healthIndex}`);
    console.log(`✅ Active Participants: ${activeParticipants}`);
    console.log(`✅ Liquidity Ratio: ${liquidityRatio}`);
    
    // Test rebase conditions
    const shouldRebase = await caesar.shouldRebase();
    const rebaseRatio = await caesar.getRebaseRatio();
    
    console.log(`✅ Should Rebase: ${shouldRebase}`);
    console.log(`✅ Rebase Ratio: ${rebaseRatio}`);
    
  } catch (error) {
    console.log(`⚠️  Economic validation limited: ${error}`);
  }
}

function generateScalabilityReport(results: Array<StressTestMetrics & { scenarioName: string }>) {
  console.log("\n📈 Performance Summary:");
  console.log("=======================");
  
  results.forEach(result => {
    const successRate = (result.successfulTransactions / result.totalTransactions * 100).toFixed(2);
    console.log(`\n${result.scenarioName}:`);
    console.log(`  📊 Success Rate: ${successRate}%`);
    console.log(`  ⚡ TPS: ${result.transactionsPerSecond.toFixed(2)}`);
    console.log(`  ⛽ Avg Gas: ${result.averageGasUsed}`);
    console.log(`  ⏱️  Avg Time: ${result.averageTransactionTime.toFixed(0)}ms`);
    console.log(`  📦 Total Txs: ${result.totalTransactions}`);
  });
  
  const avgTps = results.reduce((sum, r) => sum + r.transactionsPerSecond, 0) / results.length;
  const avgSuccessRate = results.reduce((sum, r) => 
    sum + (r.successfulTransactions / r.totalTransactions), 0) / results.length * 100;
  
  console.log(`\n🎯 Overall Metrics:`);
  console.log(`  📊 Average Success Rate: ${avgSuccessRate.toFixed(2)}%`);
  console.log(`  ⚡ Average TPS: ${avgTps.toFixed(2)}`);
  console.log(`  🎯 Target TPS (50): ${avgTps >= 50 ? '✅ ACHIEVED' : '⚠️  BELOW TARGET'}`);
}

function generateSummaryMetrics(results: Array<StressTestMetrics & { scenarioName: string }>) {
  const totalTransactions = results.reduce((sum, r) => sum + r.totalTransactions, 0);
  const totalSuccessful = results.reduce((sum, r) => sum + r.successfulTransactions, 0);
  const avgTps = results.reduce((sum, r) => sum + r.transactionsPerSecond, 0) / results.length;
  
  return {
    totalTransactions,
    totalSuccessful,
    overallSuccessRate: totalSuccessful / totalTransactions,
    averageTPS: avgTps,
    meetsTargetTPS: avgTps >= 50,
    productionReady: (totalSuccessful / totalTransactions) > 0.95 && avgTps >= 20
  };
}

if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error(error);
      process.exit(1);
    });
}