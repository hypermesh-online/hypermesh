/**
 * Dashboard and config operations with the HyperMesh TypeScript SDK.
 *
 * Run: npx tsx examples/dashboard.ts
 */
import { HyperMeshClient } from "../src/index.js";

async function main() {
  const client = new HyperMeshClient();

  // Dashboard info
  const info = await client.dashboard.info();
  console.log("Dashboard info:", JSON.stringify(info, null, 2));

  // List dashboards
  const dashboards = await client.dashboard.list();
  console.log("\nDashboards:", dashboards.count);
  for (const d of dashboards.dashboards) {
    console.log(`  ${d.name} v${d.version} (domain: ${d.domain})`);
  }

  // Show full config
  const config = await client.config.show();
  console.log("\nNode config:", JSON.stringify(config, null, 2));

  // Get specific config value
  const privacy = await client.config.get("privacy_mode");
  console.log("\nprivacy_mode:", privacy);

  // Asset listing
  const assets = await client.asset.list();
  console.log("\nAssets:", assets.count);
  for (const a of assets.assets) {
    console.log(`  [${a.category}] ${a.content_hash} (scope: ${a.scope})`);
  }

  // Topology
  const topo = await client.topology.info();
  console.log("\nTopology:");
  console.log("  Node:", topo.node_id);
  console.log("  Position:", topo.coordinate);

  const neighbors = await client.topology.neighbors();
  console.log("  Neighbors:", neighbors.count, "(radius:", neighbors.radius, ")");
}

main().catch(console.error);
