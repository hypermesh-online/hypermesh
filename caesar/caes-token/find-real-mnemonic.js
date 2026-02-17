// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { ethers } = require("ethers");

const targetAddress = "0xfD33Cf15893DaC5a0ACFdE12f06DAC63a330b331";

// The issue is "fourty-two" is not a valid BIP39 word
// Let's try valid words that sound similar or could be typos
const possibleLastWords = [
    "forty", "two", "twelve", "twenty", "fifty", "sixty", "thirty", 
    "forest", "forget", "format", "former", "fortune", "forward",
    "tower", "toward", "town", "toy", "track", "trade", "train",
    "transfer", "travel", "treat", "tree", "trial", "tribe", "trick"
];

console.log("Searching for correct mnemonic that generates:", targetAddress);
console.log("Base: 'basic oligarchy test deployment driver hello firefighter spaceship twelve eleven seven'");
console.log();

const baseMnemonic = "basic oligarchy test deployment driver hello firefighter spaceship twelve eleven seven";

for (let word of possibleLastWords) {
    const mnemonic = `${baseMnemonic} ${word}`;
    console.log(`Testing with final word: "${word}"`);
    
    try {
        const wallet = ethers.Wallet.fromPhrase(mnemonic);
        const address = wallet.address;
        
        console.log(`  Generated address: ${address}`);
        console.log(`  Matches target: ${address === targetAddress}`);
        
        if (address === targetAddress) {
            console.log();
            console.log("🎉 FOUND THE CORRECT MNEMONIC!");
            console.log(`Full mnemonic: "${mnemonic}"`);
            console.log(`Private key: ${wallet.privateKey}`);
            process.exit(0);
        }
    } catch (error) {
        console.log(`  ❌ Error: ${error.message.split('(')[0]}`);
    }
    console.log();
}

console.log("❌ None of the attempted words generated the target address");
console.log("The original mnemonic corruption might be more significant");