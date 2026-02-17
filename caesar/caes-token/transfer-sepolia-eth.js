// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("ethers");
require('dotenv').config();

async function main() {
  console.log("🔄 Transferring Sepolia ETH for deployment...\n");
  
  const provider = new ethers.JsonRpcProvider(process.env.ETH_TESTNET_RPC);

  // Test wallet private key - MUST be provided via environment variable
  const testWalletPrivateKey = process.env.TEST_WALLET_PRIVATE_KEY;
  if (!testWalletPrivateKey) {
    throw new Error("TEST_WALLET_PRIVATE_KEY environment variable not set");
  }
  const testWallet = new ethers.Wallet(testWalletPrivateKey, provider);
  
  const fromAddress = testWallet.address; // 0x08CF6C943C42d9cF56A73e47e095c33716c28595
  const toAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331"; // Main deployer
  
  console.log("From (Test Wallet):", fromAddress);
  console.log("To (Main Deployer):", toAddress);
  
  // Check balances
  const fromBalance = await provider.getBalance(fromAddress);
  const toBalance = await provider.getBalance(toAddress);
  
  console.log("\n📊 Current Balances:");
  console.log(`Test Wallet: ${ethers.formatEther(fromBalance)} ETH`);
  console.log(`Main Deployer: ${ethers.formatEther(toBalance)} ETH`);
  
  // Transfer 0.06 ETH (leaving test wallet with 0.04 ETH for gas)
  const transferAmount = ethers.parseEther("0.06");
  
  console.log(`\n💸 Transferring ${ethers.formatEther(transferAmount)} ETH...`);
  
  try {
    const tx = await testWallet.sendTransaction({
      to: toAddress,
      value: transferAmount,
      gasLimit: 21000
    });
    
    console.log("Transaction hash:", tx.hash);
    console.log("Waiting for confirmation...");
    
    const receipt = await tx.wait();
    console.log("✅ Transaction confirmed in block:", receipt.blockNumber);
    
    // Check final balances
    const finalFromBalance = await provider.getBalance(fromAddress);
    const finalToBalance = await provider.getBalance(toAddress);
    
    console.log("\n📊 Final Balances:");
    console.log(`Test Wallet: ${ethers.formatEther(finalFromBalance)} ETH`);
    console.log(`Main Deployer: ${ethers.formatEther(finalToBalance)} ETH`);
    
    if (finalToBalance >= ethers.parseEther("0.1")) {
      console.log("\n🎉 SUCCESS! Main deployer now has sufficient ETH for deployment");
    } else {
      console.log("\n⚠️ Main deployer still needs more ETH");
    }
    
  } catch (error) {
    console.error("❌ Transfer failed:", error.message);
  }
}

main().catch(console.error);