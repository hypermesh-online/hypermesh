/**
 * Low-level FFI binding definitions and library discovery for libhypermesh_ffi.
 *
 * This module is an internal dependency of ffi.ts. Use HyperMeshFFI instead.
 */

// ---------------------------------------------------------------------------
// Dynamic imports for optional native dependencies
// ---------------------------------------------------------------------------

/* eslint-disable @typescript-eslint/no-explicit-any */
export let ffi: any;
export let ref: any;

export function loadNativeDeps(): void {
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    ffi = require("ffi-napi");
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    ref = require("ref-napi");
  } catch {
    throw new Error(
      "Native FFI dependencies not installed. " +
        'Run "npm install ffi-napi ref-napi" to use HyperMeshFFI.',
    );
  }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

const LIB_NAME = "libhypermesh_ffi";

export function discoverLibrary(explicitPath?: string): string {
  if (explicitPath) {
    return explicitPath;
  }

  const envPath = process.env["HYPERMESH_FFI_LIB"];
  if (envPath) {
    return envPath;
  }

  const fs = require("fs") as typeof import("fs");
  const path = require("path") as typeof import("path");

  const ext = process.platform === "darwin" ? "dylib" : "so";
  const candidates = [
    path.resolve(__dirname, `../../../target/release/${LIB_NAME}.${ext}`),
    path.resolve(__dirname, `../../../target/debug/${LIB_NAME}.${ext}`),
    path.resolve(__dirname, `../../target/release/${LIB_NAME}.${ext}`),
    path.resolve(__dirname, `../../target/debug/${LIB_NAME}.${ext}`),
    path.resolve(process.cwd(), `target/release/${LIB_NAME}.${ext}`),
    path.resolve(process.cwd(), `target/debug/${LIB_NAME}.${ext}`),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  // Fall back to system linker resolution (ld.so / dyld)
  return `${LIB_NAME}.${ext}`;
}

// ---------------------------------------------------------------------------
// FFI function table
// ---------------------------------------------------------------------------

/** The ffi-napi binding specification for libhypermesh_ffi. */
export function buildFfiSpec(): Record<string, [string, string[]]> {
  return {
    // Lifecycle
    hypermesh_connect: ["pointer", ["string"]],
    hypermesh_disconnect: ["void", ["pointer"]],
    hypermesh_free_string: ["void", ["pointer"]],
    hypermesh_last_error: ["string", ["pointer"]],

    // Raw RPC
    hypermesh_call: ["pointer", ["pointer", "string", "string"]],

    // Node
    hypermesh_status: ["pointer", ["pointer"]],

    // DNS
    hypermesh_dns_resolve: ["pointer", ["pointer", "string"]],
    hypermesh_dns_list: ["pointer", ["pointer"]],
    hypermesh_dns_register: ["pointer", ["pointer", "string", "string"]],

    // Network
    hypermesh_peers: ["pointer", ["pointer"]],

    // Blockchain
    hypermesh_blockchain_height: ["pointer", ["pointer"]],
    hypermesh_blockchain_block: ["pointer", ["pointer", "uint64"]],

    // Topology
    hypermesh_topology_info: ["pointer", ["pointer"]],

    // Assets
    hypermesh_asset_list: ["pointer", ["pointer"]],
    hypermesh_asset_store: ["pointer", ["pointer", "string"]],
    hypermesh_asset_fetch: ["pointer", ["pointer", "string", "string"]],

    // Domains
    hypermesh_domain_list: ["pointer", ["pointer"]],
    hypermesh_domain_register: ["pointer", ["pointer", "string", "string"]],

    // Dashboards
    hypermesh_dashboard_list: ["pointer", ["pointer"]],
    hypermesh_dashboard_deploy: ["pointer", ["pointer", "string"]],

    // Config
    hypermesh_config_show: ["pointer", ["pointer"]],
    hypermesh_config_get: ["pointer", ["pointer", "string"]],

    // Caesar
    hypermesh_caesar_wallet: ["pointer", ["pointer"]],
    hypermesh_caesar_balance: ["pointer", ["pointer"]],
    hypermesh_caesar_transactions: ["pointer", ["pointer", "uint32"]],
    hypermesh_caesar_rewards: ["pointer", ["pointer"]],
    hypermesh_caesar_route_packet: [
      "pointer",
      ["pointer", "string", "double"],
    ],
    hypermesh_caesar_governor_params: ["pointer", ["pointer"]],

    // TrustChain
    hypermesh_trustchain_certificates: ["pointer", ["pointer"]],
    hypermesh_trustchain_issue: ["pointer", ["pointer", "string", "string"]],
    hypermesh_trustchain_validate: ["pointer", ["pointer", "string"]],
    hypermesh_trustchain_revoke: ["pointer", ["pointer", "string"]],
    hypermesh_trustchain_dns_zones: ["pointer", ["pointer"]],

    // Engauge
    hypermesh_engauge_capacity: ["pointer", ["pointer"]],
    hypermesh_engauge_traffic: ["pointer", ["pointer"]],
    hypermesh_engauge_marketplace: ["pointer", ["pointer"]],
    hypermesh_engauge_node_metrics: ["pointer", ["pointer"]],
    hypermesh_engauge_leases: ["pointer", ["pointer"]],

    // Catalog
    hypermesh_catalog_browse: ["pointer", ["pointer", "string", "uint32"]],
    hypermesh_catalog_search: ["pointer", ["pointer", "string"]],
    hypermesh_catalog_package_info: ["pointer", ["pointer", "string"]],
    hypermesh_catalog_registry_stats: ["pointer", ["pointer"]],
  };
}
