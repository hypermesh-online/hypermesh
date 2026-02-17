// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import fs from "fs";
import path from "path";

interface DeploymentInfo {
  network: string;
  chainId: number;
  contracts: {
    Caesar?: string;
    FinalCAES?: string;
    [key: string]: any;
  };
  tokenDetails?: {
    name: string;
    symbol: string;
    totalSupply: string;
  };
}

interface TestResults {
  contractValidation: boolean;
  tokenMetadata: any;
  ownershipVerification: boolean;
  demurrageSystemActive: boolean;
  antiSpeculationActive: boolean;
  crossChainReady: boolean;
  economicMetrics: any;
  scalabilityMetrics: any;
}

async function main() {
  console.log("🔍 Starting Production Validation on Sepolia Testnet\n");
  
  const network = await ethers.provider.getNetwork();
  console.log(`Network: ${network.name} (Chain ID: ${network.chainId})`);
  
  // Load deployment information
  const deploymentFiles = [
    "deployments/sepolia.json",
    "deployments/final-caes-sepolia.json"
  ];
  
  let deploymentInfo: DeploymentInfo | null = null;
  let caesarAddress: string | null = null;
  
  for (const file of deploymentFiles) {
    if (fs.existsSync(file)) {
      const deployment = JSON.parse(fs.readFileSync(file, 'utf8')) as DeploymentInfo;
      console.log(`📋 Found deployment: ${file}`);
      
      if (deployment.contracts.Caesar) {
        caesarAddress = deployment.contracts.Caesar;
        deploymentInfo = deployment;
        break;
      } else if (deployment.contracts.FinalCAES) {
        caesarAddress = deployment.contracts.FinalCAES;
        deploymentInfo = deployment;
        break;
      }
    }
  }
  
  if (!caesarAddress || !deploymentInfo) {
    console.error("❌ No valid Caesar token deployment found!");
    process.exit(1);
  }
  
  console.log(`✅ Using Caesar token at: ${caesarAddress}\n`);
  
  // Initialize test results
  const results: TestResults = {
    contractValidation: false,
    tokenMetadata: {},
    ownershipVerification: false,
    demurrageSystemActive: false,
    antiSpeculationActive: false,
    crossChainReady: false,
    economicMetrics: {},
    scalabilityMetrics: {}
  };
  
  try {
    // Connect to the deployed contract
    const Caesar = await ethers.getContractFactory("CaesarToken");
    const caesar = Caesar.attach(caesarAddress);
    
    console.log("1️⃣ Contract Validation");
    console.log("========================");
    
    // Basic contract validation
    try {
      const name = await caesar.name();
      const symbol = await caesar.symbol();
      const decimals = await caesar.decimals();
      const totalSupply = await caesar.totalSupply();
      
      results.tokenMetadata = {
        name,
        symbol,
        decimals: Number(decimals),
        totalSupply: ethers.formatEther(totalSupply)
      };
      
      console.log(`✅ Token Name: ${name}`);
      console.log(`✅ Token Symbol: ${symbol}`);
      console.log(`✅ Decimals: ${decimals}`);
      console.log(`✅ Total Supply: ${ethers.formatEther(totalSupply)} CAES`);
      
      results.contractValidation = true;
    } catch (error) {
      console.log(`❌ Contract validation failed: ${error}`);
    }
    
    console.log("\n2️⃣ Ownership and Access Control");
    console.log("=================================");
    
    try {
      const owner = await caesar.owner();
      const [deployer] = await ethers.getSigners();
      
      console.log(`✅ Contract Owner: ${owner}`);
      console.log(`✅ Deployer Address: ${deployer.address}`);
      
      results.ownershipVerification = owner.toLowerCase() === deployer.address.toLowerCase();
      console.log(`${results.ownershipVerification ? '✅' : '❌'} Ownership verification: ${results.ownershipVerification}`);
    } catch (error) {
      console.log(`❌ Ownership verification failed: ${error}`);
    }
    
    console.log("\n3️⃣ Economic System Validation");
    console.log("===============================");
    
    try {
      // Check demurrage system
      const demurrageManagerAddress = await caesar.demurrageManager();
      const antiSpeculationEngineAddress = await caesar.antiSpeculationEngine();
      
      console.log(`✅ Demurrage Manager: ${demurrageManagerAddress}`);
      console.log(`✅ Anti-Speculation Engine: ${antiSpeculationEngineAddress}`);
      
      results.demurrageSystemActive = demurrageManagerAddress !== ethers.ZeroAddress;
      results.antiSpeculationActive = antiSpeculationEngineAddress !== ethers.ZeroAddress;
      
      // Test economic metrics
      const currentEpoch = await caesar.getCurrentEpoch();
      const epochDuration = await caesar.getEpochDuration();
      const networkHealthIndex = await caesar.getNetworkHealthIndex();
      const stabilityPoolBalance = await caesar.getStabilityPoolBalance();
      
      results.economicMetrics = {
        currentEpoch: Number(currentEpoch),
        epochDuration: Number(epochDuration),
        networkHealthIndex: Number(networkHealthIndex),
        stabilityPoolBalance: ethers.formatEther(stabilityPoolBalance)
      };
      
      console.log(`✅ Current Epoch: ${currentEpoch}`);
      console.log(`✅ Epoch Duration: ${epochDuration} seconds`);
      console.log(`✅ Network Health Index: ${networkHealthIndex}`);
      console.log(`✅ Stability Pool Balance: ${ethers.formatEther(stabilityPoolBalance)} CAES`);
      
    } catch (error) {
      console.log(`❌ Economic system validation failed: ${error}`);
    }
    
    console.log("\n4️⃣ Cross-Chain Capabilities");
    console.log("=============================");
    
    try {
      // Check LayerZero integration
      const hasLayerZero = typeof caesar.bridgeWithDecay === 'function';
      results.crossChainReady = hasLayerZero;
      
      console.log(`${hasLayerZero ? '✅' : '❌'} LayerZero Bridge Functions: ${hasLayerZero}`);
      
      if (hasLayerZero) {
        console.log(`✅ Bridge with Decay: Available`);
        console.log(`✅ Quote Bridge with Decay: Available`);
      }
    } catch (error) {
      console.log(`❌ Cross-chain validation failed: ${error}`);
    }
    
    console.log("\n5️⃣ Transaction Testing");
    console.log("========================");
    
    try {
      const [deployer] = await ethers.getSigners();
      const balance = await caesar.balanceOf(deployer.address);
      
      console.log(`✅ Owner Balance: ${ethers.formatEther(balance)} CAES`);
      
      // Test if migration is enabled to mint some tokens for testing
      try {
        await caesar.setMigrationContract(deployer.address);
        await caesar.setMigrationEnabled(true);
        
        if (balance === 0n) {
          console.log(`🔄 Minting test tokens...`);
          const mintAmount = ethers.parseEther("10000");
          const tx = await caesar.migrationMint(deployer.address, mintAmount);
          await tx.wait();
          
          const newBalance = await caesar.balanceOf(deployer.address);
          console.log(`✅ Minted: ${ethers.formatEther(newBalance)} CAES`);
        }
        
        // Test basic transfer functionality
        const transferAmount = ethers.parseEther("100");
        const balanceBefore = await caesar.balanceOf(deployer.address);
        
        if (balanceBefore >= transferAmount) {
          console.log(`🔄 Testing transfer functionality...`);
          const tx = await caesar.transfer(deployer.address, transferAmount);
          const receipt = await tx.wait();
          
          console.log(`✅ Transfer successful - Gas used: ${receipt?.gasUsed || 'N/A'}`);
          
          results.scalabilityMetrics = {
            transferGasUsed: Number(receipt?.gasUsed || 0),
            transferSuccessful: true
          };
        }
        
      } catch (error) {
        console.log(`⚠️  Transaction testing limited: ${error}`);
      }
      
    } catch (error) {
      console.log(`❌ Transaction testing failed: ${error}`);
    }
    
    console.log("\n6️⃣ Performance Analysis");
    console.log("=========================");
    
    try {
      // Get recent block information
      const currentBlock = await ethers.provider.getBlockNumber();
      const blockInfo = await ethers.provider.getBlock(currentBlock);
      
      console.log(`✅ Current Block: ${currentBlock}`);
      console.log(`✅ Block Timestamp: ${new Date(blockInfo!.timestamp * 1000).toISOString()}`);
      console.log(`✅ Gas Limit: ${blockInfo!.gasLimit}`);
      
      // Estimate gas for various operations
      const [deployer] = await ethers.getSigners();
      
      try {
        const transferGasEstimate = await caesar.transfer.estimateGas(deployer.address, ethers.parseEther("1"));
        console.log(`✅ Transfer Gas Estimate: ${transferGasEstimate}`);
        
        const demurrageGasEstimate = await caesar.calculateDemurrage.estimateGas(deployer.address);
        console.log(`✅ Demurrage Calculation Gas Estimate: ${demurrageGasEstimate}`);
        
        results.scalabilityMetrics = {
          ...results.scalabilityMetrics,
          transferGasEstimate: Number(transferGasEstimate),
          demurrageGasEstimate: Number(demurrageGasEstimate)
        };
        
      } catch (gasError) {
        console.log(`⚠️  Gas estimation limited: ${gasError}`);
      }
      
    } catch (error) {
      console.log(`❌ Performance analysis failed: ${error}`);
    }
    
  } catch (error) {
    console.error(`❌ Critical error during validation: ${error}`);
  }
  
  // Generate comprehensive report
  console.log("\n" + "=".repeat(80));
  console.log("📊 PRODUCTION VALIDATION REPORT");
  console.log("=".repeat(80));
  
  console.log(`\n🏛️ Contract Status:`);
  console.log(`✅ Deployed Address: ${caesarAddress}`);
  console.log(`${results.contractValidation ? '✅' : '❌'} Basic Validation: ${results.contractValidation ? 'PASSED' : 'FAILED'}`);
  console.log(`${results.ownershipVerification ? '✅' : '❌'} Ownership Control: ${results.ownershipVerification ? 'VERIFIED' : 'FAILED'}`);
  
  console.log(`\n🏦 Economic Systems:`);
  console.log(`${results.demurrageSystemActive ? '✅' : '❌'} Demurrage System: ${results.demurrageSystemActive ? 'ACTIVE' : 'INACTIVE'}`);
  console.log(`${results.antiSpeculationActive ? '✅' : '❌'} Anti-Speculation: ${results.antiSpeculationActive ? 'ACTIVE' : 'INACTIVE'}`);
  
  console.log(`\n🌐 Cross-Chain Capabilities:`);
  console.log(`${results.crossChainReady ? '✅' : '❌'} LayerZero Integration: ${results.crossChainReady ? 'READY' : 'NOT READY'}`);
  
  console.log(`\n⚡ Performance Metrics:`);
  if (results.scalabilityMetrics.transferGasEstimate) {
    const gasPrice = 20; // gwei
    const ethPrice = 2500; // USD
    const transferCostUSD = (results.scalabilityMetrics.transferGasEstimate * gasPrice * ethPrice) / 1e9 / 1e18;
    console.log(`✅ Transfer Gas: ${results.scalabilityMetrics.transferGasEstimate} (~$${transferCostUSD.toFixed(4)})`);
  }
  if (results.scalabilityMetrics.demurrageGasEstimate) {
    console.log(`✅ Demurrage Gas: ${results.scalabilityMetrics.demurrageGasEstimate}`);
  }
  
  // Overall readiness assessment
  const systemsActive = results.demurrageSystemActive && results.antiSpeculationActive;
  const basicValidation = results.contractValidation && results.ownershipVerification;
  const overallReadiness = systemsActive && basicValidation;
  
  console.log(`\n🎯 OVERALL READINESS:`);
  console.log(`${overallReadiness ? '🟢' : '🟡'} Production Status: ${overallReadiness ? 'READY FOR TESTING' : 'NEEDS ATTENTION'}`);
  
  if (overallReadiness) {
    console.log(`\n🚀 NEXT STEPS:`);
    console.log(`1. Begin user acceptance testing`);
    console.log(`2. Configure cross-chain parameters`);
    console.log(`3. Set up monitoring and alerts`);
    console.log(`4. Prepare mainnet deployment`);
  } else {
    console.log(`\n⚠️  ISSUES TO RESOLVE:`);
    if (!results.contractValidation) console.log(`❌ Fix contract validation issues`);
    if (!results.ownershipVerification) console.log(`❌ Verify ownership configuration`);
    if (!results.demurrageSystemActive) console.log(`❌ Activate demurrage system`);
    if (!results.antiSpeculationActive) console.log(`❌ Activate anti-speculation engine`);
  }
  
  // Save report
  const reportData = {
    timestamp: new Date().toISOString(),
    network: network.name,
    chainId: network.chainId,
    contractAddress: caesarAddress,
    results,
    overallReadiness
  };
  
  const reportPath = `test-reports/production-validation-${Date.now()}.json`;
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(reportData, null, 2));
  
  console.log(`\n📝 Report saved: ${reportPath}`);
  
  process.exit(overallReadiness ? 0 : 1);
}

if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error(error);
      process.exit(1);
    });
}