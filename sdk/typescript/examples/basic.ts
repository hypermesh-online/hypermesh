/**
 * Basic usage of the HyperMesh TypeScript SDK.
 *
 * Run: npx tsx examples/basic.ts
 */
import { HyperMeshClient, HyperMeshError } from "../src/index.js";

async function main() {
  const client = new HyperMeshClient(); // http://localhost:9293

  // Ping the node
  const pong = await client.node.ping();
  console.log("Ping:", pong);

  // Get node status
  const status = await client.node.status();
  console.log("Node ID:", status.node_id);
  console.log("Chain height:", status.chain_height);
  console.log("Peers:", status.peers);
  console.log("Privacy mode:", status.privacy_mode);
  console.log("Uptime:", status.uptime_secs, "seconds");
  console.log("Coordinate:", status.coordinate);

  // List connected peers
  const peers = await client.network.peers();
  console.log("\nConnected peers:", peers.count);
  for (const peer of peers.peers) {
    console.log(" -", JSON.stringify(peer));
  }

  // Error handling
  try {
    await client.blockchain.block(999999);
  } catch (err) {
    if (err instanceof HyperMeshError) {
      console.log("\nExpected error for block 999999:");
      console.log("  Status:", err.status);
      console.log("  Message:", err.message);
    }
  }
}

main().catch(console.error);
