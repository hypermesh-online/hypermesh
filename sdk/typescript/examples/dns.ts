/**
 * DNS operations with the HyperMesh TypeScript SDK.
 *
 * Run: npx tsx examples/dns.ts
 */
import { HyperMeshClient, HyperMeshError } from "../src/index.js";

async function main() {
  const client = new HyperMeshClient();

  // List existing DNS records
  const dns = await client.dns.list();
  console.log("DNS records:", dns.count);
  for (const record of dns.records) {
    console.log(`  ${record.name} -> ${record.address}`);
  }

  // Register a new record
  console.log("\nRegistering example.hypermesh -> ::1");
  const reg = await client.dns.register("example.hypermesh", "::1");
  console.log("Register result:", reg);

  // Resolve the record
  try {
    const resolved = await client.dns.resolve("example.hypermesh");
    console.log("Resolved:", resolved.name, "->", resolved.address);
  } catch (err) {
    if (err instanceof HyperMeshError) {
      console.log("Resolve failed:", err.message);
    }
  }

  // List records again to confirm registration
  const updated = await client.dns.list();
  console.log("\nDNS records after registration:", updated.count);
  for (const record of updated.records) {
    console.log(`  ${record.name} -> ${record.address}`);
  }
}

main().catch(console.error);
