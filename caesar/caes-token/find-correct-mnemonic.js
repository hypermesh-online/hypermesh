// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("ethers");

const targetAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331";

// Try with the original "fourty-two" as you specified
const possibleMnemonics = [
    // Original with fourty-two as you said
    "basic oligarchy test deployment driver hello firefighter spaceship twelve eleven seven fourty-two",
];

console.log("Searching for correct mnemonic that generates:", targetAddress);
console.log();

for (let i = 0; i < possibleMnemonics.length; i++) {
    const mnemonic = possibleMnemonics[i];
    console.log(`Testing mnemonic ${i + 1}: "${mnemonic}"`);
    
    try {
        // Try to create wallet directly (ethers v6 doesn't have isValidMnemonic in utils)
        const wallet = ethers.Wallet.fromPhrase(mnemonic);
        const address = wallet.address;
        
        console.log(`  Generated address: ${address}`);
        console.log(`  Matches target: ${address === targetAddress}`);
        console.log(`  Private key: ${wallet.privateKey}`);
        
        if (address === targetAddress) {
            console.log();
            console.log("🎉 FOUND IT! This is the correct mnemonic:");
            console.log(`"${mnemonic}"`);
            console.log(`Private key: ${wallet.privateKey}`);
            process.exit(0);
        }
    } catch (error) {
        console.log(`  ❌ Error: ${error.message}`);
    }
    console.log();
}

console.log("❌ None of the attempted mnemonics generated the target address");
console.log("The original mnemonic might be completely different or corrupted");