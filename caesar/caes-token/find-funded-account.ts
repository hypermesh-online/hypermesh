// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";

async function main() {
  console.log("=== Finding the Funded Account ===\n");
  
  // Test up to 10 accounts from the mnemonic
  const signers = await ethers.getSigners();
  
  // Connect to mainnet to check balances
  const MAINNET_RPC = "https://eth.llamarpc.com";
  const mainnetProvider = new ethers.JsonRpcProvider(MAINNET_RPC);
  
  const targetAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331";
  
  console.log("Looking for account:", targetAddress);
  console.log("Checking first 10 Hardhat accounts...\n");
  
  for (let i = 0; i < Math.min(signers.length, 10); i++) {
    const signer = signers[i];
    const address = signer.address;
    
    try {
      const balance = await mainnetProvider.getBalance(address);
      const balanceEth = ethers.formatEther(balance);
      
      console.log(`Account ${i}: ${address}`);
      console.log(`  Balance: ${balanceEth} ETH`);
      
      if (address === targetAddress) {
        console.log(`  🎯 THIS IS THE FUNDED WALLET! Account index: ${i}`);
        console.log(`  ✅ Has ${balanceEth} ETH - can make transfer`);
        return i;
      }
      
      if (parseFloat(balanceEth) > 0) {
        console.log(`  💰 Has funds but wrong address`);
      }
      
      console.log("");
      
    } catch (error) {
      console.log(`  ❌ Error checking balance: ${error.message}`);
    }
  }
  
  console.log("❌ Target address not found in first 10 accounts");
  console.log("❌ This means the mnemonic is completely wrong or the derivation path is different");
}

main().catch(console.error);