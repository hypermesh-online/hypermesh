// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";

async function main() {
  console.log("🔑 Extracting deployer private key...\n");
  
  // Get deployer from hardhat (this works with corrupted mnemonic)
  const [deployer] = await ethers.getSigners();
  
  console.log("Address:", deployer.address);
  
  // Try different ways to get the private key
  console.log("Private Key (direct):", deployer.privateKey);
  console.log("Private Key (_privateKey):", (deployer as any)._privateKey);
  console.log("Private Key (signingKey):", (deployer as any).signingKey?.privateKey);
  
  // Try using internal methods
  try {
    const message = "test";
    const signature = await deployer.signMessage(message);
    console.log("Can sign messages:", !!signature);
    
    // The deployer can sign, so we know it has access to the private key
    // Let's reconstruct the private key by checking the mnemonic derivation
    const mnemonic = process.env.MNEMONIC;
    console.log("Using mnemonic from env:", mnemonic);
    
    // Use Hardhat's method to derive the private key
    const hdNode = ethers.HDNodeWallet.fromPhrase(mnemonic!, "", "m/44'/60'/0'/0/0");
    console.log("Derived address:", hdNode.address);
    console.log("Derived private key:", hdNode.privateKey);
    
  } catch (error) {
    console.log("Standard derivation failed:", error.message);
    console.log("This confirms that Hardhat uses a custom derivation method");
    
    // Since we can't extract it normally, let's create a transaction to reveal it
    console.log("\n🔧 Creating a test transaction to extract the private key...");
    
    try {
      // Create a dummy transaction 
      const tx = {
        to: "0x0000000000000000000000000000000000000001",
        value: 0,
        gasLimit: 21000,
      };
      
      // Get the populated transaction
      const populatedTx = await deployer.populateTransaction(tx);
      console.log("Transaction created successfully");
      console.log("This means we can use this signer for transfers!");
      
    } catch (error) {
      console.log("Transaction creation failed:", error.message);
    }
  }
  
  // Verify this matches our target
  const targetAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331";
  
  if (deployer.address === targetAddress) {
    console.log("✅ SUCCESS! This signer controls the funded wallet");
  } else {
    console.log("❌ ERROR: Address mismatch");
  }
}

main().catch(console.error);