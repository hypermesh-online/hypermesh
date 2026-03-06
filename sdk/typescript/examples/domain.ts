/**
 * Domain operations with the HyperMesh TypeScript SDK.
 *
 * Run: npx tsx examples/domain.ts
 */
import { HyperMeshClient } from "../src/index.js";

async function main() {
  const client = new HyperMeshClient();

  // List registered domains
  const domains = await client.domain.list();
  console.log("Domains:", domains.count);
  for (const d of domains.domains) {
    console.log(`  ${d.domain} (privacy: ${d.privacy}, owner: ${d.owner})`);
  }

  // Register a new domain (creates a network-scope blockchain)
  console.log("\nRegistering domain 'testapp' with Private privacy...");
  const reg = await client.domain.register("testapp", "Private");
  console.log("Registered:", reg);

  // Join a domain
  console.log("\nJoining domain 'testapp'...");
  const join = await client.domain.join("testapp");
  console.log("Join result:", join);
}

main().catch(console.error);
