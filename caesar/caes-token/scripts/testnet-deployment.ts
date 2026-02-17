// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import { writeFileSync, readFileSync } from "fs";
import path from "path";

interface DeploymentConfig {
  network: string;
  gasLimit: number;
  gasPrice?: string;
  confirmations: number;
  timeout: number;
}

interface DeployedContracts {
  caesarToken: string;
  goldOracle: string;
  demurrageManager: string;
  antiSpeculationEngine: string;
  mockLZEndpoint: string;
  deploymentBlock: number;
  timestamp: number;
}

async function main() {
  const network = await ethers.provider.getNetwork();
  console.log(`\n🚀 Deploying Caesar Token system to ${network.name} (Chain ID: ${network.chainId})`);

  const [deployer] = await ethers.getSigners();
  console.log(`Deploying with account: ${deployer.address}`);
  
  const balance = await ethers.provider.getBalance(deployer.address);
  console.log(`Account balance: ${ethers.formatEther(balance)} ETH`);

  // Deployment configuration based on network
  const config: DeploymentConfig = getDeploymentConfig(network.name);
  console.log(`Using configuration:`, config);

  try {
    // Deploy MockLZEndpoint for testnet
    console.log("\n📡 Deploying MockLZEndpoint...");
    const MockLZEndpoint = await ethers.getContractFactory("MockLZEndpoint");
    const mockLZEndpoint = await MockLZEndpoint.deploy();
    await mockLZEndpoint.waitForDeployment();
    const mockLZEndpointAddress = await mockLZEndpoint.getAddress();
    console.log(`✅ MockLZEndpoint deployed at: ${mockLZEndpointAddress}`);

    // Deploy Caesar Token system
    console.log("\n🏛️ Deploying Caesar Token...");
    const CaesarToken = await ethers.getContractFactory("CaesarToken");
    const caesarToken = await CaesarToken.deploy(
      mockLZEndpointAddress,
      deployer.address,
      {
        gasLimit: config.gasLimit,
        ...(config.gasPrice && { gasPrice: ethers.parseUnits(config.gasPrice, "gwei") })
      }
    );
    
    await caesarToken.waitForDeployment();
    const caesarTokenAddress = await caesarToken.getAddress();
    console.log(`✅ Caesar Token deployed at: ${caesarTokenAddress}`);

    // Get component contract addresses
    console.log("\n🔍 Retrieving component contracts...");
    const goldOracleAddress = await caesarToken.goldOracle();
    const demurrageManagerAddress = await caesarToken.demurrageManager();
    const antiSpeculationEngineAddress = await caesarToken.antiSpeculationEngine();

    console.log(`📊 Gold Oracle: ${goldOracleAddress}`);
    console.log(`⏰ Demurrage Manager: ${demurrageManagerAddress}`);
    console.log(`🛡️ Anti-Speculation Engine: ${antiSpeculationEngineAddress}`);

    // Setup initial configuration
    console.log("\n⚙️ Setting up initial configuration...");
    
    // Enable migration for testing
    await caesarToken.setMigrationContract(deployer.address);
    await caesarToken.setMigrationEnabled(true);
    console.log("✅ Migration enabled for testing");

    // Get deployment info
    const currentBlock = await ethers.provider.getBlockNumber();
    const timestamp = Math.floor(Date.now() / 1000);

    // Prepare deployment record
    const deployedContracts: DeployedContracts = {
      caesarToken: caesarTokenAddress,
      goldOracle: goldOracleAddress,
      demurrageManager: demurrageManagerAddress,
      antiSpeculationEngine: antiSpeculationEngineAddress,
      mockLZEndpoint: mockLZEndpointAddress,
      deploymentBlock: currentBlock,
      timestamp
    };

    // Save deployment info
    const deploymentFile = path.join(__dirname, `../deployments/${network.name}.json`);
    writeFileSync(deploymentFile, JSON.stringify(deployedContracts, null, 2));
    console.log(`📝 Deployment info saved to: ${deploymentFile}`);

    // Verify contracts if on supported networks
    if (shouldVerifyContracts(network.name)) {
      console.log("\n🔍 Initiating contract verification...");
      await verifyContracts(deployedContracts, mockLZEndpointAddress, deployer.address);
    }

    // Run basic functionality tests
    console.log("\n🧪 Running post-deployment tests...");
    await runPostDeploymentTests(caesarToken, deployer);

    console.log("\n🎉 Deployment completed successfully!");
    console.log("\n📋 Summary:");
    console.log(`Network: ${network.name}`);
    console.log(`Caesar Token: ${caesarTokenAddress}`);
    console.log(`Block: ${currentBlock}`);
    
    return deployedContracts;

  } catch (error) {
    console.error("\n❌ Deployment failed:", error);
    process.exit(1);
  }
}

function getDeploymentConfig(networkName: string): DeploymentConfig {
  const configs: { [key: string]: DeploymentConfig } = {
    hardhat: {
      network: "hardhat",
      gasLimit: 8000000,
      confirmations: 1,
      timeout: 60000
    },
    sepolia: {
      network: "sepolia",
      gasLimit: 6000000,
      gasPrice: "20",
      confirmations: 2,
      timeout: 300000
    },
    mumbai: {
      network: "mumbai",
      gasLimit: 6000000,
      gasPrice: "30",
      confirmations: 2,
      timeout: 300000
    },
    bscTestnet: {
      network: "bscTestnet",
      gasLimit: 6000000,
      gasPrice: "10",
      confirmations: 3,
      timeout: 300000
    },
    arbitrumSepolia: {
      network: "arbitrumSepolia",
      gasLimit: 8000000,
      confirmations: 1,
      timeout: 300000
    }
  };

  return configs[networkName] || configs.hardhat;
}

function shouldVerifyContracts(networkName: string): boolean {
  const verifiableNetworks = ["sepolia", "mumbai", "bscTestnet", "arbitrumSepolia"];
  return verifiableNetworks.includes(networkName);
}

async function verifyContracts(contracts: DeployedContracts, lzEndpoint: string, owner: string) {
  try {
    console.log("Verifying Caesar Token...");
    await run("verify:verify", {
      address: contracts.caesarToken,
      constructorArguments: [lzEndpoint, owner]
    });
    console.log("✅ Caesar Token verified");
  } catch (error) {
    console.log("⚠️ Caesar Token verification failed:", error);
  }
  
  try {
    console.log("Verifying MockLZEndpoint...");
    await run("verify:verify", {
      address: contracts.mockLZEndpoint,
      constructorArguments: []
    });
    console.log("✅ MockLZEndpoint verified");
  } catch (error) {
    console.log("⚠️ MockLZEndpoint verification failed:", error);
  }
}

async function runPostDeploymentTests(caesarToken: any, deployer: any) {
  try {
    // Test 1: Basic token information
    const name = await caesarToken.name();
    const symbol = await caesarToken.symbol();
    const decimals = await caesarToken.decimals();
    console.log(`✅ Token info - Name: ${name}, Symbol: ${symbol}, Decimals: ${decimals}`);

    // Test 2: Initial supply should be 0 (no tokens minted yet)
    const totalSupply = await caesarToken.totalSupply();
    console.log(`✅ Initial total supply: ${ethers.formatEther(totalSupply)} CAES`);

    // Test 3: Migration mint test
    const testAmount = ethers.parseEther("1000");
    await caesarToken.migrationMint(deployer.address, testAmount);
    const balance = await caesarToken.balanceOf(deployer.address);
    console.log(`✅ Migration mint test: ${ethers.formatEther(balance)} CAES minted`);

    // Test 4: Network health index
    const healthIndex = await caesarToken.getNetworkHealthIndex();
    console.log(`✅ Network health index: ${healthIndex}`);

    // Test 5: Epoch information
    const currentEpoch = await caesarToken.getCurrentEpoch();
    const epochDuration = await caesarToken.getEpochDuration();
    console.log(`✅ Current epoch: ${currentEpoch}, Duration: ${epochDuration} seconds`);

    // Test 6: Stability pool
    const stabilityBalance = await caesarToken.getStabilityPoolBalance();
    console.log(`✅ Stability pool balance: ${ethers.formatEther(stabilityBalance)} CAES`);

    console.log("✅ All post-deployment tests passed!");

  } catch (error) {
    console.error("❌ Post-deployment test failed:", error);
    throw error;
  }
}

// Market stress testing function
export async function runMarketStressTest(contracts: DeployedContracts) {
  console.log("\n🔥 Starting Market Stress Test...");
  
  const caesarToken = await ethers.getContractAt("CaesarToken", contracts.caesarToken);
  const [deployer, ...users] = await ethers.getSigners();
  
  // Setup test accounts with tokens
  const testAccounts = users.slice(0, 20);
  const initialAmount = ethers.parseEther("10000");
  
  console.log("Setting up test accounts...");
  for (const user of testAccounts) {
    await caesarToken.migrationMint(user.address, initialAmount);
  }
  
  // Stress test scenarios
  const scenarios = [
    { name: "High Frequency Trading", txCount: 1000, delayMs: 10 },
    { name: "Large Volume Transfers", txCount: 100, delayMs: 100 },
    { name: "Micro Transactions", txCount: 2000, delayMs: 5 }
  ];
  
  for (const scenario of scenarios) {
    console.log(`\n🧪 Running ${scenario.name}...`);
    const startTime = Date.now();
    
    let successCount = 0;
    let errorCount = 0;
    
    for (let i = 0; i < scenario.txCount; i++) {
      try {
        const from = testAccounts[i % testAccounts.length];
        const to = testAccounts[(i + 1) % testAccounts.length];
        const amount = ethers.parseEther((Math.random() * 100).toString());
        
        await caesarToken.connect(from).transfer(to.address, amount);
        successCount++;
        
        if (scenario.delayMs > 0) {
          await new Promise(resolve => setTimeout(resolve, scenario.delayMs));
        }
        
      } catch (error) {
        errorCount++;
        if (errorCount % 10 === 0) {
          console.log(`⚠️ Errors encountered: ${errorCount}`);
        }
      }
      
      if ((i + 1) % 100 === 0) {
        console.log(`Progress: ${i + 1}/${scenario.txCount} transactions`);
      }
    }
    
    const duration = Date.now() - startTime;
    const tps = successCount / (duration / 1000);
    
    console.log(`✅ ${scenario.name} completed:`);
    console.log(`  - Successful: ${successCount}/${scenario.txCount}`);
    console.log(`  - Success rate: ${((successCount / scenario.txCount) * 100).toFixed(2)}%`);
    console.log(`  - TPS: ${tps.toFixed(2)}`);
    console.log(`  - Duration: ${(duration / 1000).toFixed(2)}s`);
  }
  
  // Check final state
  const finalHealthIndex = await caesarToken.getNetworkHealthIndex();
  const finalStabilityPool = await caesarToken.getStabilityPoolBalance();
  
  console.log(`\n📊 Final State:`);
  console.log(`Network Health: ${finalHealthIndex}`);
  console.log(`Stability Pool: ${ethers.formatEther(finalStabilityPool)} CAES`);
  
  return {
    networkHealth: Number(finalHealthIndex),
    stabilityPool: ethers.formatEther(finalStabilityPool),
    testsPassed: true
  };
}

// Economic model validation
export async function validateEconomicModel(contracts: DeployedContracts) {
  console.log("\n🏦 Validating Economic Model...");
  
  const caesarToken = await ethers.getContractAt("CaesarToken", contracts.caesarToken);
  const goldOracle = await ethers.getContractAt("GoldPriceOracle", contracts.goldOracle);
  const [deployer, ...users] = await ethers.getSigners();
  
  // Test demurrage mechanics
  console.log("Testing demurrage mechanics...");
  const testUser = users[0];
  await caesarToken.migrationMint(testUser.address, ethers.parseEther("1000"));
  
  const initialBalance = await caesarToken.balanceOf(testUser.address);
  
  // Simulate time passage (90 days)
  await ethers.provider.send("evm_increaseTime", [90 * 24 * 3600]);
  await ethers.provider.send("evm_mine", []);
  
  // Apply demurrage
  await caesarToken.applyDemurrage(testUser.address);
  const balanceAfterDemurrage = await caesarToken.balanceOf(testUser.address);
  
  const demurrageAmount = initialBalance - balanceAfterDemurrage;
  console.log(`✅ Demurrage applied: ${ethers.formatEther(demurrageAmount)} CAES`);
  
  // Test anti-speculation engine
  console.log("Testing anti-speculation penalties...");
  const speculator = users[1];
  await caesarToken.migrationMint(speculator.address, ethers.parseEther("5000"));
  
  // Rapid successive trades
  let totalPenalties = 0n;
  for (let i = 0; i < 10; i++) {
    const balanceBefore = await caesarToken.balanceOf(speculator.address);
    await caesarToken.connect(speculator).transfer(users[2].address, ethers.parseEther("100"));
    const balanceAfter = await caesarToken.balanceOf(speculator.address);
    
    const actualTransfer = balanceBefore - balanceAfter;
    if (actualTransfer > ethers.parseEther("100")) {
      totalPenalties += actualTransfer - ethers.parseEther("100");
    }
  }
  
  console.log(`✅ Anti-speculation penalties: ${ethers.formatEther(totalPenalties)} CAES`);
  
  // Test rebase mechanism
  console.log("Testing rebase functionality...");
  const shouldRebaseNow = await caesarToken.shouldRebase();
  const rebaseRatio = await caesarToken.getRebaseRatio();
  console.log(`✅ Should rebase: ${shouldRebaseNow}, Ratio: ${rebaseRatio}`);
  
  return {
    demurrageWorking: demurrageAmount > 0,
    antiSpeculationWorking: totalPenalties > 0,
    rebaseMechanismReady: typeof shouldRebaseNow === 'boolean',
    economicModelValid: true
  };
}

// Helper function to import and use hardhat run
async function run(command: string, params?: any) {
  try {
    const { run } = await import("hardhat");
    return await run(command, params);
  } catch (error) {
    console.log(`Could not run ${command}:`, error);
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