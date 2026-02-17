// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import fs from "fs";
import path from "path";

/**
 * CAES Token Deployment Script
 * Deploys CaesarToken (CAESAR/CAES) to any network
 */
async function main() {
  console.log("🚀 Deploying CAES Token\n");
  
  const [deployer] = await ethers.getSigners();
  const network = await ethers.provider.getNetwork();
  
  console.log("Network:", network.name);
  console.log("Deployer:", deployer.address);
  console.log("Balance:", ethers.formatEther(await ethers.provider.getBalance(deployer.address)), "ETH\n");
  
  // LayerZero endpoints
  const endpoints: { [key: string]: string } = {
    "11155111": "0x6EDCE65403992e310A62460808c4b910D972f10f", // Sepolia
    "1": "0x1a44076050125825900e736c501f859c50fE728c",       // Mainnet
    "42161": "0x3c2269811836af69497E5F486A85D7316753cf62",    // Arbitrum
    "137": "0x3c2269811836af69497E5F486A85D7316753cf62",     // Polygon
  };
  
  const lzEndpoint = endpoints[network.chainId.toString()];
  if (!lzEndpoint) {
    throw new Error(`LayerZero endpoint not configured for chain ${network.chainId}`);
  }
  
  console.log("Deploying CaesarToken...");
  const CaesarToken = await ethers.getContractFactory("CaesarToken");
  const caes = await CaesarToken.deploy(lzEndpoint, deployer.address);
  await caes.waitForDeployment();
  
  const address = await caes.getAddress();
  const name = await caes.name();
  const symbol = await caes.symbol();
  const supply = await caes.totalSupply();
  
  console.log("✅ Deployed:", address);
  console.log("   Name:", name);
  console.log("   Symbol:", symbol);
  console.log("   Supply:", ethers.formatEther(supply));
  
  // Save deployment
  const deployment = {
    network: network.name,
    chainId: network.chainId.toString(),
    timestamp: new Date().toISOString(),
    deployer: deployer.address,
    caes: address,
    layerzero: lzEndpoint
  };
  
  const file = path.join(__dirname, "..", "deployments", `${network.name}.json`);
  fs.writeFileSync(file, JSON.stringify(deployment, null, 2));
  
  console.log("💾 Saved to:", file);
  console.log("\n🔍 Verify with:");
  console.log(`npx hardhat verify --network ${network.name} ${address} "${lzEndpoint}" "${deployer.address}"`);
  
  return { caes: address, network: network.name };
}

if (require.main === module) {
  main().then(() => process.exit(0)).catch(error => {
    console.error(error);
    process.exit(1);
  });
}

export default main;