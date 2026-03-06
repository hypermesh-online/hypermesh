/**
 * Blockchain operations with the HyperMesh TypeScript SDK.
 *
 * Run: npx tsx examples/blockchain.ts
 */
import { HyperMeshClient } from "../src/index.js";

async function main() {
  const client = new HyperMeshClient();

  // Get the chain height
  const { height } = await client.blockchain.height();
  console.log("Blockchain height:", height);

  // Fetch the genesis block
  const genesis = await client.blockchain.block(0);
  console.log("\nGenesis block:");
  console.log("  Index:", genesis.index);
  console.log("  Hash:", genesis.hash);
  console.log("  Previous hash:", genesis.previous_hash);
  console.log("  Timestamp:", genesis.timestamp);

  // Fetch the latest block (if height > 0)
  if (height > 0) {
    const latest = await client.blockchain.block(height - 1);
    console.log("\nLatest block (index", latest.index, "):");
    console.log("  Hash:", latest.hash);
  }

  // Validate the chain
  const validation = await client.blockchain.validate();
  console.log("\nBlockchain valid:", validation.valid);
  if (validation.blocks_checked !== undefined) {
    console.log("Blocks checked:", validation.blocks_checked);
  }
  if (validation.errors && validation.errors.length > 0) {
    console.log("Errors:", validation.errors);
  }
}

main().catch(console.error);
