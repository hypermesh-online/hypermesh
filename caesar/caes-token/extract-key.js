// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("ethers");

// The working mnemonic from .env (with the specific invalid words that Hardhat handles)
const mnemonic = "basic oligarchy test deployment driver hello firefighter spaceship twelve eleven seven fourty-two";

console.log("Testing mnemonic:", mnemonic);

try {
  // This will fail with standard ethers but works with Hardhat
  const hdNode = ethers.HDNodeWallet.fromPhrase(mnemonic);
  const wallet = hdNode.derivePath("m/44'/60'/0'/0/0");
  console.log("Generated Address:", wallet.address);
  console.log("Private Key:", wallet.privateKey);
} catch (error) {
  console.log("Standard ethers failed (as expected):", error.message);
  
  // Now try with each word corrected to find the pattern
  console.log("\n=== Testing word corrections ===");
  
  const corrections = [
    mnemonic.replace("oligarchy", "memory"),
    mnemonic.replace("fourty-two", "forest"),
    mnemonic.replace("fourty-two", "family"),
    mnemonic.replace("fourty-two", "frozen"),
  ];
  
  for (let i = 0; i < corrections.length; i++) {
    try {
      const corrected = corrections[i];
      console.log(`\nTrying: ${corrected}`);
      const hdNode = ethers.HDNodeWallet.fromPhrase(corrected);
      const wallet = hdNode.derivePath("m/44'/60'/0'/0/0");
      console.log("Address:", wallet.address);
      
      if (wallet.address === "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331") {
        console.log("🎉 FOUND IT!");
        console.log("Private Key:", wallet.privateKey);
        process.exit(0);
      }
    } catch (e) {
      console.log("Failed:", e.message);
    }
  }
  
  console.log("\n❌ Could not find correct words");
}