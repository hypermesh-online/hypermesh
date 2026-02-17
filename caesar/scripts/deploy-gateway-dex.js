// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("hardhat");

async function main() {
  const [deployer] = await ethers.getSigners();
  
  console.log("🚀 Deploying Caesar Token DEX contracts...");
  console.log("Deployer address:", deployer.address);
  console.log("Account balance:", (await deployer.provider.getBalance(deployer.address)).toString());

  // Configuration
  const CAESAR_TOKEN = "0x6299744254422aadb6a57183f47eaae1678cf86cc58a0c78dfc4fd2caa3ba2a4";
  const WETH_ADDRESS = "0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9"; // Sepolia WETH
  
  console.log("CAESAR Token:", CAESAR_TOKEN);
  console.log("WETH Address:", WETH_ADDRESS);

  // Deploy Factory
  console.log("\n🏭 Deploying DEX Factory...");
  const CaesarCoinDEXFactory = await ethers.getContractFactory("CaesarCoinDEXFactory");
  const factory = await CaesarCoinDEXFactory.deploy(
    deployer.address, // owner
    deployer.address  // feeToSetter
  );
  await factory.waitForDeployment();
  const factoryAddress = await factory.getAddress();
  console.log("✅ Factory deployed to:", factoryAddress);

  // Create CAES/WETH pair
  console.log("\n💰 Creating CAES/WETH trading pair...");
  const createPairTx = await factory.createPair(CAESAR_TOKEN, WETH_ADDRESS);
  await createPairTx.wait();
  
  const pairAddress = await factory.getPair(CAESAR_TOKEN, WETH_ADDRESS);
  console.log("✅ CAES/WETH pair created at:", pairAddress);

  // Configure trading fee
  console.log("\n⚙️ Setting trading fee to 0.3%...");
  await factory.setTradingFee(30);
  console.log("✅ Trading fee configured");

  // Save deployment information
  const deploymentInfo = {
    network: network.name,
    chainId: network.config.chainId,
    deployer: deployer.address,
    timestamp: new Date().toISOString(),
    contracts: {
      factory: factoryAddress,
      pairs: {
        "CAES/WETH": pairAddress
      }
    },
    tokens: {
      CAESAR: CAESAR_TOKEN,
      WETH: WETH_ADDRESS
    }
  };

  console.log("\n📋 Deployment Summary:");
  console.log("========================");
  console.log("Network:", network.name);
  console.log("Factory:", factoryAddress);
  console.log("CAES/WETH Pair:", pairAddress);
  console.log("Trading Fee: 0.3%");
  
  // Write deployment info
  const fs = require('fs');
  const deploymentFile = `deployments/gateway-dex-${network.name}.json`;
  fs.writeFileSync(deploymentFile, JSON.stringify(deploymentInfo, null, 2));
  console.log("\n💾 Deployment saved to:", deploymentFile);

  console.log("\n🎉 Caesar Token DEX deployment completed!");
  console.log("\nFrontend can now connect to:", factoryAddress);

  return {
    factory: factoryAddress,
    pair: pairAddress,
    deploymentInfo
  };
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("❌ Deployment failed:", error);
    process.exit(1);
  });