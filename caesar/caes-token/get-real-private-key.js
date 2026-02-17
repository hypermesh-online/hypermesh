// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("hardhat");

async function main() {
  console.log("🔐 Extracting the real private key using Hardhat internals...\n");
  
  // Get the hardhat accounts
  const accounts = await ethers.getSigners();
  const deployer = accounts[0];
  
  console.log("Deployer address:", deployer.address);
  
  // Access Hardhat's internal wallet provider
  const { wallet } = hre.network.provider;
  
  if (wallet) {
    console.log("Found wallet provider");
    
    // Get all accounts from wallet
    const addresses = await wallet.getAddresses();
    console.log("Available addresses:", addresses);
    
    // Get the private key for our address
    for (const address of addresses) {
      if (address.toLowerCase() === deployer.address.toLowerCase()) {
        const privateKey = await wallet.getPrivateKey(address);
        console.log(`\n🎉 FOUND THE PRIVATE KEY!`);
        console.log(`Address: ${address}`);
        console.log(`Private Key: ${privateKey}`);
        
        // Verify this private key generates the correct address
        const testWallet = new ethers.Wallet(privateKey);
        console.log(`\n✅ Verification: ${testWallet.address === address ? 'MATCH' : 'MISMATCH'}`);
        
        return;
      }
    }
  } else {
    console.log("No wallet provider found, trying alternative methods...");
    
    // Try to access mnemonic directly
    const config = require("./hardhat.config.ts");
    console.log("Config networks:", Object.keys(config.networks || {}));
  }
}

main().catch(console.error);