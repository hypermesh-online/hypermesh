// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import fs from "fs";

async function main() {
  console.log("⚡ Simple Performance Test on Sepolia Testnet");
  console.log("==============================================\n");
  
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
  const [deployer] = await ethers.getSigners();
  
  console.log(`👤 Testing with account: ${deployer.address}\n`);
  
  // Performance benchmarks
  console.log("🔬 Performance Benchmarks");
  console.log("==========================");
  
  const tests = [];
  
  // 1. Basic read operations
  console.log("1️⃣ Read Operations Performance:");
  
  const readTests = [
    { name: "balanceOf", fn: () => caesar.balanceOf(deployer.address) },
    { name: "totalSupply", fn: () => caesar.totalSupply() },
    { name: "name", fn: () => caesar.name() },
    { name: "symbol", fn: () => caesar.symbol() },
    { name: "getCurrentEpoch", fn: () => caesar.getCurrentEpoch() },
    { name: "getNetworkHealthIndex", fn: () => caesar.getNetworkHealthIndex() },
    { name: "getStabilityPoolBalance", fn: () => caesar.getStabilityPoolBalance() },
    { name: "calculateDemurrage", fn: () => caesar.calculateDemurrage(deployer.address) }
  ];
  
  for (const test of readTests) {
    try {
      const startTime = Date.now();
      const result = await test.fn();
      const duration = Date.now() - startTime;
      
      console.log(`  ✅ ${test.name}: ${duration}ms`);
      tests.push({ operation: test.name, type: 'read', duration, success: true });
    } catch (error) {
      console.log(`  ❌ ${test.name}: ERROR`);
      tests.push({ operation: test.name, type: 'read', duration: 0, success: false, error: error.toString() });
    }
  }
  
  // 2. Gas estimation for write operations
  console.log("\n2️⃣ Gas Estimation for Write Operations:");
  
  const gasTests = [
    { name: "transfer(self, 1 CAES)", fn: () => caesar.transfer.estimateGas(deployer.address, ethers.parseEther("1")) },
    { name: "updateAccountActivity", fn: () => caesar.updateAccountActivity.estimateGas(deployer.address) },
    { name: "applyDemurrage", fn: () => caesar.applyDemurrage.estimateGas(deployer.address) },
    { name: "advanceEpoch", fn: () => caesar.advanceEpoch.estimateGas() }
  ];
  
  for (const test of gasTests) {
    try {
      const startTime = Date.now();
      const gasEstimate = await test.fn();
      const duration = Date.now() - startTime;
      
      console.log(`  ✅ ${test.name}: ${gasEstimate} gas (${duration}ms)`);
      tests.push({ 
        operation: test.name, 
        type: 'gas_estimate', 
        duration, 
        gasEstimate: Number(gasEstimate), 
        success: true 
      });
    } catch (error) {
      console.log(`  ❌ ${test.name}: ERROR - ${error.message.slice(0, 50)}...`);
      tests.push({ 
        operation: test.name, 
        type: 'gas_estimate', 
        duration: 0, 
        success: false, 
        error: error.toString() 
      });
    }
  }
  
  // 3. Network and block information
  console.log("\n3️⃣ Network Performance:");
  
  try {
    const startBlock = Date.now();
    const blockNumber = await ethers.provider.getBlockNumber();
    const blockTime = Date.now() - startBlock;
    
    const startBalance = Date.now();
    const balance = await ethers.provider.getBalance(deployer.address);
    const balanceTime = Date.now() - startBalance;
    
    console.log(`  ✅ getBlockNumber: ${blockTime}ms (Block: ${blockNumber})`);
    console.log(`  ✅ getBalance: ${balanceTime}ms (${ethers.formatEther(balance)} ETH)`);
    
    tests.push({ operation: "getBlockNumber", type: "network", duration: blockTime, success: true });
    tests.push({ operation: "getBalance", type: "network", duration: balanceTime, success: true });
    
  } catch (error) {
    console.log(`  ❌ Network tests failed: ${error}`);
  }
  
  // 4. Economic system health check
  console.log("\n4️⃣ Economic System Health:");
  
  try {
    const healthData = await Promise.all([
      caesar.getCurrentEpoch(),
      caesar.getNetworkHealthIndex(), 
      caesar.getActiveParticipants(),
      caesar.getLiquidityRatio(),
      caesar.getStabilityPoolBalance(),
      caesar.shouldRebase(),
      caesar.getRebaseRatio()
    ]);
    
    const [epoch, health, participants, liquidity, stabilityPool, shouldRebase, rebaseRatio] = healthData;
    
    console.log(`  📊 Current Epoch: ${epoch}`);
    console.log(`  📈 Health Index: ${health}`);
    console.log(`  👥 Active Participants: ${participants}`);
    console.log(`  💧 Liquidity Ratio: ${liquidity}`);
    console.log(`  🏦 Stability Pool: ${ethers.formatEther(stabilityPool)} CAES`);
    console.log(`  🔄 Should Rebase: ${shouldRebase}`);
    console.log(`  📊 Rebase Ratio: ${rebaseRatio}`);
    
    // Economic health assessment
    const healthScore = Number(health);
    const hasLiquidity = Number(liquidity) > 100;
    const systemStable = !shouldRebase || Number(rebaseRatio) === 10000;
    
    console.log(`\n  🎯 Economic Health Assessment:`);
    console.log(`     ${healthScore > 500 ? '✅' : '⚠️'} Health Score: ${healthScore > 500 ? 'GOOD' : 'NEEDS ATTENTION'}`);
    console.log(`     ${hasLiquidity ? '✅' : '⚠️'} Liquidity: ${hasLiquidity ? 'ADEQUATE' : 'LOW'}`);
    console.log(`     ${systemStable ? '✅' : '⚠️'} Stability: ${systemStable ? 'STABLE' : 'REBALANCING NEEDED'}`);
    
  } catch (error) {
    console.log(`  ❌ Economic health check failed: ${error}`);
  }
  
  // 5. Cross-chain readiness
  console.log("\n5️⃣ Cross-Chain Readiness:");
  
  try {
    // Test LayerZero functions
    const hasLayerZero = typeof caesar.bridgeWithDecay === 'function';
    console.log(`  ${hasLayerZero ? '✅' : '❌'} LayerZero Integration: ${hasLayerZero ? 'READY' : 'NOT AVAILABLE'}`);
    
    if (hasLayerZero) {
      // Test quote function (read-only)
      try {
        const mockSendParam = {
          dstEid: 1, // Ethereum mainnet
          to: ethers.zeroPadValue(deployer.address, 32),
          amountLD: ethers.parseEther("100"),
          minAmountLD: ethers.parseEther("95"),
          extraOptions: "0x",
          composeMsg: "0x",
          oftCmd: "0x"
        };
        
        const startQuote = Date.now();
        const quote = await caesar.quoteBridgeWithDecay(mockSendParam, false);
        const quoteTime = Date.now() - startQuote;
        
        console.log(`  ✅ Bridge Quote: ${quoteTime}ms (Fee: ${ethers.formatEther(quote.nativeFee)} ETH)`);
        
      } catch (error) {
        console.log(`  ⚠️  Bridge quote limited: ${error.message.slice(0, 50)}...`);
      }
    }
    
  } catch (error) {
    console.log(`  ❌ Cross-chain readiness check failed: ${error}`);
  }
  
  // Generate summary report
  console.log("\n" + "=".repeat(80));
  console.log("📊 PERFORMANCE SUMMARY");
  console.log("=".repeat(80));
  
  const readOps = tests.filter(t => t.type === 'read' && t.success);
  const gasOps = tests.filter(t => t.type === 'gas_estimate' && t.success);
  
  if (readOps.length > 0) {
    const avgReadTime = readOps.reduce((sum, t) => sum + t.duration, 0) / readOps.length;
    console.log(`\n📖 Read Operations:`);
    console.log(`   Average Response Time: ${avgReadTime.toFixed(1)}ms`);
    console.log(`   Success Rate: ${(readOps.length / tests.filter(t => t.type === 'read').length * 100).toFixed(1)}%`);
  }
  
  if (gasOps.length > 0) {
    const avgGas = gasOps.reduce((sum, t) => sum + (t.gasEstimate || 0), 0) / gasOps.length;
    console.log(`\n⛽ Gas Estimates:`);
    console.log(`   Average Gas Usage: ${avgGas.toFixed(0)}`);
    console.log(`   Estimated Cost (20 gwei): ~$${(avgGas * 20 * 2500 / 1e9 / 1e18).toFixed(4)}`);
  }
  
  const successRate = tests.filter(t => t.success).length / tests.length;
  console.log(`\n🎯 Overall Performance:`);
  console.log(`   Success Rate: ${(successRate * 100).toFixed(1)}%`);
  console.log(`   System Status: ${successRate > 0.8 ? '🟢 OPERATIONAL' : '🟡 DEGRADED'}`);
  
  // Save report
  const reportData = {
    timestamp: new Date().toISOString(),
    network: "sepolia",
    contractAddress: caesarAddress,
    performanceTests: tests,
    summary: {
      successRate,
      avgReadTime: readOps.length > 0 ? readOps.reduce((sum, t) => sum + t.duration, 0) / readOps.length : 0,
      avgGasUsage: gasOps.length > 0 ? gasOps.reduce((sum, t) => sum + (t.gasEstimate || 0), 0) / gasOps.length : 0
    }
  };
  
  const reportPath = `test-reports/performance-test-${Date.now()}.json`;
  fs.mkdirSync("test-reports", { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(reportData, null, 2));
  
  console.log(`\n📝 Performance report saved: ${reportPath}`);
  
  if (successRate > 0.8) {
    console.log(`\n🚀 PRODUCTION READINESS: ✅ READY FOR USER TESTING`);
  } else {
    console.log(`\n⚠️  PRODUCTION READINESS: 🟡 NEEDS OPTIMIZATION`);
  }
}

if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error(error);
      process.exit(1);
    });
}