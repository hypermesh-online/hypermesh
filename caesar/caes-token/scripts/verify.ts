// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

/**
 * CAES Token Verification Script
 * Verifies deployed CaesarToken on Etherscan
 */
async function main() {
  console.log("🔍 Verifying CAES Token\n");
  
  const network = await ethers.provider.getNetwork();
  const [deployer] = await ethers.getSigners();
  
  // Get deployment info
  const CAES_ADDRESS = "0x69e35749eB8f5Ae282A883329C7d0BF44bCed59C"; // Sepolia
  const LAYERZERO_ENDPOINT = "0x6EDCE65403992e310A62460808c4b910D972f10f"; // Sepolia
  
  console.log("Network:", network.name);
  console.log("Contract:", CAES_ADDRESS);
  console.log("Deployer:", deployer.address);
  console.log("LayerZero:", LAYERZERO_ENDPOINT, "\n");
  
  // Construct verification command
  const cmd = `npx hardhat verify --network ${network.name} ${CAES_ADDRESS} "${LAYERZERO_ENDPOINT}" "${deployer.address}"`;
  
  console.log("Running verification...");
  console.log("Command:", cmd, "\n");
  
  try {
    const { stdout, stderr } = await execAsync(cmd);
    
    if (stdout) {
      console.log("✅ Verification output:");
      console.log(stdout);
    }
    
    if (stderr) {
      console.log("⚠️ Verification warnings:");
      console.log(stderr);
    }
    
    console.log("🎉 Verification completed");
    
  } catch (error) {
    console.log("❌ Verification failed:", error.message);
    
    if (error.message.includes("already verified")) {
      console.log("✅ Contract is already verified");
    } else {
      throw error;
    }
  }
  
  return { verified: true, contract: CAES_ADDRESS };
}

if (require.main === module) {
  main().then(() => process.exit(0)).catch(error => {
    console.error(error);
    process.exit(1);
  });
}

export default main;