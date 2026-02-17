// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("ethers");

// The working test wallet - we know this private key works
const testWalletPrivateKey = "0x6c642bd2fb3f1a2f16ed3d3209e3669f7f5f1e4e2d30d0c1beb7a90e16a63bbb";
const testWallet = new ethers.Wallet(testWalletPrivateKey);

console.log("Test Wallet (working):");
console.log("Address:", testWallet.address);
console.log("Private Key:", testWallet.privateKey);

// Let's try to derive the target wallet's private key
// Since we know the address 0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331 was generated,
// maybe it was the first account from a different mnemonic

const targetAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331";

// Check if we can find this wallet by testing if it's another index from the test wallet mnemonic
const testMnemonic = "extra clutch little fan foot grape liar simple finish almost boring embrace";

console.log("\nChecking different derivation paths from test wallet mnemonic...");
for (let i = 0; i < 10; i++) {
    try {
        const hdNode = ethers.HDNodeWallet.fromPhrase(testMnemonic, "", `m/44'/60'/0'/0/${i}`);
        console.log(`Index ${i}: ${hdNode.address}`);
        
        if (hdNode.address === targetAddress) {
            console.log("\n🎉 FOUND THE TARGET WALLET!");
            console.log(`Private Key: ${hdNode.privateKey}`);
            console.log(`Mnemonic: ${testMnemonic}`);
            console.log(`Path: m/44'/60'/0'/0/${i}`);
            process.exit(0);
        }
    } catch (error) {
        console.log(`Error at index ${i}: ${error.message}`);
    }
}

console.log("\n❌ Target address not found in first 10 derivations");
console.log("The main wallet might have been created with a completely different method");