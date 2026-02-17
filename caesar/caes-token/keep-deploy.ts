// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import fs from "fs";
import path from "path";

/**
 * Production deployment script for CAESAR (CAES) Token
 * This script deploys the CaesarToken contract with proper configuration
 */
async function main() {
  console.log("🚀 Deploying CAESAR (CAES) Token - Production Version\n");
  
  const [deployer] = await ethers.getSigners();
  const network = await ethers.provider.getNetwork();
  
  console.log("📊 Deployment Configuration:");
  console.log("  Network:", network.name);
  console.log("  Chain ID:", network.chainId.toString());
  console.log("  Deployer:", deployer.address);
  console.log("  Balance:", ethers.formatEther(await ethers.provider.getBalance(deployer.address)), "ETH\n");
  
  // LayerZero endpoint addresses for different networks
  const LAYERZERO_ENDPOINTS: { [key: string]: string } = {
    "11155111": "0x6EDCE65403992e310A62460808c4b910D972f10f", // Sepolia
    "40161": "0x6EDCE65403992e310A62460808c4b910D972f10f",    // Arbitrum Sepolia
    "80002": "0x6EDCE65403992e310A62460808c4b910D972f10f",    // Polygon Amoy
    "1": "0x1a44076050125825900e736c501f859c50fE728c",       // Ethereum Mainnet
    "42161": "0x3c2269811836af69497E5F486A85D7316753cf62",    // Arbitrum One
    "137": "0x3c2269811836af69497E5F486A85D7316753cf62",     // Polygon
  };
  
  const lzEndpoint = LAYERZERO_ENDPOINTS[network.chainId.toString()];
  if (!lzEndpoint) {
    throw new Error(`LayerZero endpoint not configured for chain ${network.chainId}`);
  }
  
  console.log("📡 LayerZero Configuration:");
  console.log("  Endpoint:", lzEndpoint, "\n");
  
  try {
    // Deploy CaesarToken (CAES)
    console.log("1️⃣ Deploying CAESAR Token (CAES)...");
    
    const CaesarToken = await ethers.getContractFactory("CaesarToken");
    const deploymentTx = CaesarToken.getDeploymentTransaction(
      lzEndpoint,
      deployer.address
    );
    
    console.log("  Estimated gas:", (await ethers.provider.estimateGas(deploymentTx)).toString());
    
    const caesarToken = await CaesarToken.deploy(
      lzEndpoint,
      deployer.address
    );
    
    await caesarToken.waitForDeployment();
    const tokenAddress = await caesarToken.getAddress();
    
    console.log("✅ CAESAR (CAES) token deployed to:", tokenAddress);
    
    // Get token details
    const name = await caesarToken.name();
    const symbol = await caesarToken.symbol();
    const decimals = await caesarToken.decimals();
    const totalSupply = await caesarToken.totalSupply();
    
    console.log("\n📋 Token Details:");
    console.log("  Name:", name);
    console.log("  Symbol:", symbol);
    console.log("  Decimals:", decimals.toString());
    console.log("  Total Supply:", ethers.formatUnits(totalSupply, decimals), symbol);
    console.log("  Owner:", await caesarToken.owner());
    
    // Save deployment information
    const deploymentInfo = {
      network: network.name,
      chainId: network.chainId.toString(),
      timestamp: new Date().toISOString(),
      deployer: deployer.address,
      contracts: {
        CaesarToken: tokenAddress,
        LayerZeroEndpoint: lzEndpoint
      },
      tokenDetails: {
        name: name,
        symbol: symbol,
        decimals: decimals.toString(),
        totalSupply: ethers.formatUnits(totalSupply, decimals)
      },
      blockNumber: await ethers.provider.getBlockNumber()
    };
    
    // Ensure deployments directory exists
    const deploymentsDir = path.join(__dirname, "..", "deployments");
    if (!fs.existsSync(deploymentsDir)) {
      fs.mkdirSync(deploymentsDir, { recursive: true });
    }
    
    // Write deployment file
    const deploymentFile = path.join(deploymentsDir, `caes-production-${network.name}.json`);
    fs.writeFileSync(deploymentFile, JSON.stringify(deploymentInfo, null, 2));
    
    console.log("\n💾 Deployment information saved to:", deploymentFile);
    
    // Output summary
    console.log("\n🎉 Deployment Summary:");
    console.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    console.log(`✅ CAESAR (CAES) Token: ${tokenAddress}`);
    console.log(`📊 Network: ${network.name} (${network.chainId})`);
    console.log(`👤 Owner: ${deployer.address}`);
    console.log(`🪙 Supply: ${ethers.formatUnits(totalSupply, decimals)} ${symbol}`);
    console.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Verification instructions
    if (network.name !== "hardhat" && network.name !== "localhost") {
      console.log("\n🔍 To verify the contract, run:");
      console.log(`npx hardhat verify --network ${network.name} ${tokenAddress} "${lzEndpoint}" "${deployer.address}"`);
    }
    
    return {
      caesarToken: tokenAddress,
      deployer: deployer.address,
      network: network.name,
      chainId: network.chainId.toString()
    };
    
  } catch (error) {
    console.error("❌ Deployment failed:", error);
    throw error;
  }
}

// Execute deployment
if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error(error);
      process.exit(1);
    });
}

export default main;