// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("ethers");

// Since we know from Hardhat that this mnemonic generates 0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331
// Let's try using Hardhat's mnemonic parsing approach
const hdkey = require('hdkey');
const bip39 = require('bip39');

async function main() {
  console.log("🔧 Manual ETH Transfer using custom mnemonic derivation...\n");
  
  const targetAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331";
  const recipientAddress = "0x08CF6C943C42d9cF56A73e47e095c33716c28595";
  
  // The corrupted mnemonic that works in Hardhat
  const corruptedMnemonic = "basic oligarchy test deployment driver hello firefighter spaceship twelve eleven seven fourty-two";
  
  // Since Hardhat can handle this, it must be using a custom derivation
  // Let's try to replicate what Hardhat does internally
  
  try {
    // Create a seed from the corrupted mnemonic (bypassing BIP39 validation)
    console.log("Attempting custom seed generation...");
    
    // Try treating it as a raw seed
    const words = corruptedMnemonic.split(' ');
    console.log(`Processing ${words.length} words`);
    
    // Hardhat might be using a different approach - let's try creating the seed manually
    // using a simple hash-based approach that doesn't validate BIP39
    const crypto = require('crypto');
    const seed = crypto.pbkdf2Sync(corruptedMnemonic, 'mnemonic', 2048, 64, 'sha512');
    
    console.log("Generated seed from corrupted mnemonic");
    
    // Create HD key from seed
    const hdWallet = hdkey.fromMasterSeed(seed);
    
    // Derive the first account (m/44'/60'/0'/0/0)
    const derivedKey = hdWallet.derive("m/44'/60'/0'/0/0");
    const privateKey = '0x' + derivedKey.privateKey.toString('hex');
    
    console.log("Derived private key:", privateKey);
    
    // Create wallet from derived private key
    const wallet = new ethers.Wallet(privateKey);
    console.log("Generated address:", wallet.address);
    console.log("Target address:", targetAddress);
    console.log("Match:", wallet.address.toLowerCase() === targetAddress.toLowerCase());
    
    if (wallet.address.toLowerCase() === targetAddress.toLowerCase()) {
      console.log("\n🎉 SUCCESS! We found the correct derivation!");
      
      // Now let's do the transfer
      const provider = new ethers.JsonRpcProvider("https://eth.llamarpc.com");
      const connectedWallet = wallet.connect(provider);
      
      const balance = await provider.getBalance(wallet.address);
      console.log(`Current balance: ${ethers.formatEther(balance)} ETH`);
      
      if (balance >= ethers.parseEther("0.001")) {
        console.log("✅ Sufficient balance for transfer");
        console.log("🚀 Sending 0.001 ETH...");
        
        const tx = await connectedWallet.sendTransaction({
          to: recipientAddress,
          value: ethers.parseEther("0.001"),
          gasLimit: 21000
        });
        
        console.log("Transaction sent:", tx.hash);
        console.log("Waiting for confirmation...");
        
        const receipt = await tx.wait();
        console.log("✅ Transaction confirmed!");
        console.log(`Block: ${receipt.blockNumber}`);
        console.log(`Gas used: ${receipt.gasUsed.toString()}`);
        
        // Check final balances
        const finalSenderBalance = await provider.getBalance(wallet.address);
        const finalRecipientBalance = await provider.getBalance(recipientAddress);
        
        console.log("\n=== Final Balances ===");
        console.log(`Sender: ${ethers.formatEther(finalSenderBalance)} ETH`);
        console.log(`Recipient: ${ethers.formatEther(finalRecipientBalance)} ETH`);
        
      } else {
        console.log("❌ Insufficient balance");
      }
    } else {
      console.log("❌ Address mismatch - trying alternative derivation...");
    }
    
  } catch (error) {
    console.error("Error:", error.message);
  }
}

main().catch(console.error);