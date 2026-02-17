// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from "hardhat";

/**
 * CAES Token Test Script
 * Tests deployed CaesarToken functionality
 */
async function main() {
  console.log("🧪 Testing CAES Token\n");
  
  const [deployer] = await ethers.getSigners();
  const network = await ethers.provider.getNetwork();
  
  // Get deployment address
  const CAES_ADDRESS = "0x69e35749eB8f5Ae282A883329C7d0BF44bCed59C"; // Sepolia
  
  console.log("Network:", network.name);
  console.log("Testing:", CAES_ADDRESS, "\n");
  
  const caes = await ethers.getContractAt("CaesarToken", CAES_ADDRESS);
  
  // Basic tests
  const tests = [
    { name: "Name", call: () => caes.name() },
    { name: "Symbol", call: () => caes.symbol() },
    { name: "Decimals", call: () => caes.decimals() },
    { name: "Total Supply", call: () => caes.totalSupply() },
    { name: "Owner", call: () => caes.owner() },
    { name: "Balance", call: () => caes.balanceOf(deployer.address) },
    { name: "Migration Enabled", call: () => caes.migrationEnabled() },
  ];
  
  let passed = 0;
  for (const test of tests) {
    try {
      const result = await test.call();
      console.log(`✅ ${test.name}:`, result.toString());
      passed++;
    } catch (error) {
      console.log(`❌ ${test.name}: ERROR -`, error.message);
    }
  }
  
  console.log(`\n📊 Results: ${passed}/${tests.length} passed`);
  
  if (passed === tests.length) {
    console.log("🎉 All tests passed - CAES token is operational");
  } else {
    console.log("⚠️ Some tests failed - check contract state");
  }
  
  return { passed, total: tests.length };
}

if (require.main === module) {
  main().then(() => process.exit(0)).catch(error => {
    console.error(error);
    process.exit(1);
  });
}

export default main;