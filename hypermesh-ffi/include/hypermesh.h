/*
 * Copyright 2026 Hypermesh Foundation. All rights reserved.
 * Licensed under the Business Source License 1.1.
 *
 * C FFI bindings for the HyperMesh SDK.
 *
 * Usage:
 *   1. Call hypermesh_connect() to obtain a client handle.
 *   2. Use the typed API functions (or hypermesh_call for raw RPC).
 *   3. Free every returned char* with hypermesh_free_string().
 *   4. On NULL return, inspect the error via hypermesh_last_error().
 *   5. Call hypermesh_disconnect() when done.
 *
 * Thread safety:
 *   - The client handle is safe to share across threads.
 *   - hypermesh_last_error() returns a thread-local pointer.
 */

#ifndef HYPERMESH_H
#define HYPERMESH_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque client handle. */
typedef struct hypermesh_client_t hypermesh_client_t;

/* -----------------------------------------------------------------------
 * Connection lifecycle
 * ----------------------------------------------------------------------- */

/*
 * Connect to a running HyperMesh daemon.
 *
 * socket_path: Path to the Unix domain socket, or NULL to use the default
 *              3-tier fallback ($HYPERMESH_SOCK / $XDG_RUNTIME_DIR / ~/.hypermesh).
 *
 * Returns an opaque handle on success, or NULL on error.
 */
hypermesh_client_t *hypermesh_connect(const char *socket_path);

/*
 * Disconnect and free a client handle. After this call the pointer is
 * invalid. Passing NULL is a safe no-op.
 */
void hypermesh_disconnect(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Raw RPC call
 * ----------------------------------------------------------------------- */

/*
 * Send an arbitrary JSON-RPC method call to the daemon.
 *
 * method:      Null-terminated method name (e.g. "node.status").
 * params_json: Null-terminated JSON string for the params object.
 *
 * Returns a heap-allocated JSON string (caller frees with
 * hypermesh_free_string), or NULL on error.
 */
char *hypermesh_call(hypermesh_client_t *client,
                     const char *method,
                     const char *params_json);

/* -----------------------------------------------------------------------
 * Typed API — Node
 * ----------------------------------------------------------------------- */

/* Fetch the current node status. Returns JSON. */
char *hypermesh_status(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Typed API — DNS
 * ----------------------------------------------------------------------- */

/* Resolve a DNS name. Returns the address string. */
char *hypermesh_dns_resolve(hypermesh_client_t *client, const char *name);

/* List all registered DNS entries. Returns JSON array. */
char *hypermesh_dns_list(hypermesh_client_t *client);

/* Register a DNS name pointing to the given address. Returns JSON. */
char *hypermesh_dns_register(hypermesh_client_t *client,
                             const char *name,
                             const char *addr);

/* -----------------------------------------------------------------------
 * Typed API — Network
 * ----------------------------------------------------------------------- */

/* List connected peers. Returns JSON array. */
char *hypermesh_peers(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Typed API — Blockchain
 * ----------------------------------------------------------------------- */

/* Get the current blockchain height. Returns JSON number. */
char *hypermesh_blockchain_height(hypermesh_client_t *client);

/* Get a block by index. Returns JSON. */
char *hypermesh_blockchain_block(hypermesh_client_t *client, uint64_t index);

/* -----------------------------------------------------------------------
 * Typed API — Topology
 * ----------------------------------------------------------------------- */

/* Get this node's topology info. Returns JSON. */
char *hypermesh_topology_info(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Typed API — Assets
 * ----------------------------------------------------------------------- */

/* List all stored assets. Returns JSON array. */
char *hypermesh_asset_list(hypermesh_client_t *client);

/* Store a file as a HyperMesh asset. Returns JSON with asset_id. */
char *hypermesh_asset_store(hypermesh_client_t *client, const char *file_path);

/* Fetch an asset by ID and write it to output_path. Returns "ok" on success. */
char *hypermesh_asset_fetch(hypermesh_client_t *client,
                            const char *asset_id,
                            const char *output_path);

/* -----------------------------------------------------------------------
 * Typed API — Domains
 * ----------------------------------------------------------------------- */

/* List registered domains. Returns JSON array. */
char *hypermesh_domain_list(hypermesh_client_t *client);

/* Register a domain with name and privacy mode. Returns JSON. */
char *hypermesh_domain_register(hypermesh_client_t *client,
                                const char *name,
                                const char *privacy);

/* -----------------------------------------------------------------------
 * Typed API — Dashboards
 * ----------------------------------------------------------------------- */

/* List deployed dashboards. Returns JSON array. */
char *hypermesh_dashboard_list(hypermesh_client_t *client);

/* Deploy a dashboard from the given path. Returns JSON. */
char *hypermesh_dashboard_deploy(hypermesh_client_t *client, const char *path);

/* -----------------------------------------------------------------------
 * Typed API — Config
 * ----------------------------------------------------------------------- */

/* Show the full daemon config. Returns JSON. */
char *hypermesh_config_show(hypermesh_client_t *client);

/* Get a single config value by key. Returns JSON. */
char *hypermesh_config_get(hypermesh_client_t *client, const char *key);

/* -----------------------------------------------------------------------
 * Typed API — Caesar EVP
 * ----------------------------------------------------------------------- */

/* Fetch the caller's Caesar wallet info. Returns JSON. */
char *hypermesh_caesar_wallet(hypermesh_client_t *client);

/* Fetch the current Caesar balance. Returns JSON. */
char *hypermesh_caesar_balance(hypermesh_client_t *client);

/* Fetch recent Caesar transactions (limit=0 for default). Returns JSON array. */
char *hypermesh_caesar_transactions(hypermesh_client_t *client, uint32_t limit);

/* Fetch accumulated Caesar rewards. Returns JSON. */
char *hypermesh_caesar_rewards(hypermesh_client_t *client);

/* Route a Caesar EVP packet to a destination. Returns JSON. */
char *hypermesh_caesar_route_packet(hypermesh_client_t *client,
                                    const char *destination,
                                    double amount_grams);

/* Fetch current Caesar Governor parameters. Returns JSON. */
char *hypermesh_caesar_governor_params(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Typed API — TrustChain
 * ----------------------------------------------------------------------- */

/* List all TrustChain certificates. Returns JSON array. */
char *hypermesh_trustchain_certificates(hypermesh_client_t *client);

/* Issue a new certificate for a subject and scope. Returns JSON. */
char *hypermesh_trustchain_issue(hypermesh_client_t *client,
                                 const char *subject,
                                 const char *scope);

/* Validate a PEM-encoded certificate. Returns JSON validation result. */
char *hypermesh_trustchain_validate(hypermesh_client_t *client,
                                    const char *cert_pem);

/* Revoke a certificate by ID. Returns JSON result. */
char *hypermesh_trustchain_revoke(hypermesh_client_t *client,
                                  const char *cert_id);

/* List TrustChain DNS zones. Returns JSON array. */
char *hypermesh_trustchain_dns_zones(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Typed API — Engauge Analytics
 * ----------------------------------------------------------------------- */

/* Fetch current node capacity metrics. Returns JSON. */
char *hypermesh_engauge_capacity(hypermesh_client_t *client);

/* Fetch current traffic statistics. Returns JSON. */
char *hypermesh_engauge_traffic(hypermesh_client_t *client);

/* Fetch marketplace resource pool info. Returns JSON. */
char *hypermesh_engauge_marketplace(hypermesh_client_t *client);

/* Fetch detailed node-level metrics. Returns JSON. */
char *hypermesh_engauge_node_metrics(hypermesh_client_t *client);

/* Fetch active resource leases. Returns JSON array. */
char *hypermesh_engauge_leases(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Typed API — Catalog Registry
 * ----------------------------------------------------------------------- */

/* Browse catalog packages. query may be NULL. Returns paginated JSON. */
char *hypermesh_catalog_browse(hypermesh_client_t *client,
                               const char *query,
                               uint32_t page);

/* Search catalog packages by query string. Returns JSON array. */
char *hypermesh_catalog_search(hypermesh_client_t *client,
                               const char *query);

/* Get detailed info about a specific catalog package. Returns JSON. */
char *hypermesh_catalog_package_info(hypermesh_client_t *client,
                                     const char *name);

/* Fetch catalog registry statistics. Returns JSON. */
char *hypermesh_catalog_registry_stats(hypermesh_client_t *client);

/* -----------------------------------------------------------------------
 * Memory management
 * ----------------------------------------------------------------------- */

/*
 * Free a string previously returned by any hypermesh_* function.
 * Passing NULL is a safe no-op.
 */
void hypermesh_free_string(char *s);

/*
 * Return the last error message for the current thread, or NULL if no
 * error has occurred. The returned pointer is valid until the next FFI
 * call on the same thread. Do NOT free this pointer.
 */
const char *hypermesh_last_error(const hypermesh_client_t *client);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* HYPERMESH_H */
