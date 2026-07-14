/*
 * Copyright 2026 Hypermesh Foundation. All rights reserved.
 * Licensed under the Business Source License 1.1.
 *
 * HyperMesh C ABI — stable interface for non-Rust consumers.
 *
 * Two families of functions:
 *
 *  1. Daemon IPC (hypermesh_connect / hypermesh_call / hypermesh_status / ...):
 *     open a client handle to a running daemon and marshal JSON. Every returned
 *     char* MUST be freed with hypermesh_free_string(); on NULL, inspect
 *     hypermesh_last_error().
 *
 *  2. Direct crate ABI (hypermesh_identity_* / hypermesh_asset_address_* /
 *     hypermesh_verify_* / hypermesh_compute_content_hash /
 *     hypermesh_signed_proof_verify): call the TrustChain and lib Rust paths
 *     in-process — no daemon required. These preserve the Proof-of-State gate
 *     (signed-proof verification) and the BLAKE3 content-hash mirror invariant
 *     (R4). They cannot mint a valid proof without the signing secret key, nor
 *     serve an asset payload that fails its content hash.
 *
 * Return-code convention for the direct ABI (HM_* constants):
 *   HM_OK (0) success; HM_VERIFY_OK (1) / HM_VERIFY_FAIL (0) for checks;
 *   negative values are errors (also set the thread-local error string).
 *
 * Memory / ownership:
 *   - hypermesh_client_t*  -> hypermesh_disconnect()
 *   - hypermesh_identity_t* -> hypermesh_identity_free()
 *   - char* (IPC results)   -> hypermesh_free_string()
 *   Byte / string outputs use caller-provided buffers with a two-call length
 *   pattern (pass NULL/0 first to learn the required length via *out_len).
 *   All pointer arguments are NULL-checked; no Rust panic crosses the boundary.
 */


#ifndef HYPERMESH_H
#define HYPERMESH_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

// FFI status codes returned by the direct identity/asset ABI.
//
// `0` is success. Negative values are error classes; on any error the
// thread-local message is also set (see `hypermesh_last_error`).
#define HM_OK 0

// A required pointer argument was NULL.
#define HM_ERR_NULL -1

// An argument was invalid (bad length, non-UTF-8, out of range, etc).
#define HM_ERR_INVALID -2

// The caller-provided output buffer was too small; required length is
// written to the `*_len` out-parameter so the caller can retry.
#define HM_ERR_BUFFER_TOO_SMALL -3

// An underlying cryptographic or I/O operation failed.
#define HM_ERR_INTERNAL -4

// A verification check ran successfully but the result was "not authentic".
#define HM_VERIFY_FAIL 0

// A verification check ran successfully and the result was "authentic".
#define HM_VERIFY_OK 1

// Opaque handle wrapping a [`FalconIdentity`] (holds secret key material).
//
// Exposed to C as `hypermesh_identity_t`. Never dereferenced by the caller.
// Must be freed exactly once with [`hypermesh_identity_free`].
typedef struct hypermesh_identity_t hypermesh_identity_t;

// Opaque handle exposed through the C API as `hypermesh_client_t`.
//
// Contains the tokio runtime (owned) and the async SDK client.
typedef struct hypermesh_client_t hypermesh_client_t;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Send an arbitrary JSON-RPC call to the daemon.
//
// `method` and `params_json` are null-terminated UTF-8 C strings.
// Returns a JSON string the caller must free with `hypermesh_free_string`,
// or `NULL` on error (inspect via `hypermesh_last_error`).
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `method` and `params_json` must be valid null-terminated UTF-8 strings.
char *hypermesh_call(hypermesh_client_t *client, const char *method, const char *params_json);

// Free a string previously returned by any `hypermesh_*` function.
//
// # Safety
//
// `s` must be a pointer previously returned by this library, or `NULL`.
// Calling with any other pointer is undefined behavior.
void hypermesh_free_string(char *s);

// Return the last error message, or `NULL` if no error has occurred.
//
// The returned pointer is valid until the next FFI call **on the same
// thread**. The caller must NOT free this pointer.
//
// # Safety
//
// `_client` is accepted for API consistency but currently unused
// (errors are thread-local). May be `NULL`.
const char *hypermesh_last_error(const hypermesh_client_t *_client);

// List all stored assets as a JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_asset_list(hypermesh_client_t *client);

// Store a file as a HyperMesh asset. Returns the store result as JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `file_path` must be a valid null-terminated UTF-8 string.
char *hypermesh_asset_store(hypermesh_client_t *client, const char *file_path);

// Fetch an asset by ID and write it to the output path.
// Returns `"ok"` on success.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `asset_id` and `output_path` must be valid null-terminated UTF-8 strings.
char *hypermesh_asset_fetch(hypermesh_client_t *client,
                            const char *asset_id,
                            const char *output_path);

// Construct an `AssetAddress` from matrix coords, a 32-byte content hash, and
// a shard index (0-15), writing the 16 address bytes into `out16`.
//
// `x`/`y`/`z` must fit in i16 range `[-32768, 32767]`. Returns `HM_OK`,
// `HM_ERR_INVALID` (coord overflow or shard > 15), or `HM_ERR_NULL`.
//
// # Safety
//
// `content_hash32` must point to 32 readable bytes; `out16` to 16 writable
// bytes.
int hypermesh_asset_address_new(int64_t x,
                                int64_t y,
                                int64_t z,
                                const uint8_t *content_hash32,
                                uint8_t shard,
                                uint8_t *out16);

// Format the 16-byte address in `bytes16` as an IPv6 string, writing a
// null-terminated string into `out` (capacity `out_cap`). A full IPv6 text
// form fits in 46 bytes; pass at least 46.
//
// # Safety
//
// `bytes16` must point to 16 readable bytes; `out` to `out_cap` writable bytes.
int hypermesh_asset_address_to_ipv6(const uint8_t *bytes16, char *out, uintptr_t out_cap);

// Parse an IPv6 string (`ipv6_str`) into the 16-byte address form (`out16`),
// validating the HyperMesh `fd48:4d00` prefix.
//
// Returns `HM_OK`, `HM_ERR_INVALID` (bad IPv6 or wrong prefix), or
// `HM_ERR_NULL`.
//
// # Safety
//
// `ipv6_str` must be a valid null-terminated UTF-8 string; `out16` must point
// to 16 writable bytes.
int hypermesh_asset_address_from_ipv6(const char *ipv6_str, uint8_t *out16);

// Extract the 6-byte asset fingerprint (bytes 10-15: 5 BLAKE3 hash bytes +
// hash-nibble/shard byte) from `bytes16` into `out6`.
//
// # Safety
//
// `bytes16` must point to 16 readable bytes; `out6` to 6 writable bytes.
int hypermesh_asset_address_fingerprint(const uint8_t *bytes16, uint8_t *out6);

// Extract the matrix coordinates (x,y,z) from `bytes16` into the three
// out-params.
//
// # Safety
//
// `bytes16` must point to 16 readable bytes; `out_x`/`out_y`/`out_z` must be
// valid pointers.
int hypermesh_asset_address_coords(const uint8_t *bytes16,
                                   int64_t *out_x,
                                   int64_t *out_y,
                                   int64_t *out_z);

// Return the shard index (0-15) of the address in `bytes16`, or a negative
// error code (`HM_ERR_NULL`).
//
// # Safety
//
// `bytes16` must point to 16 readable bytes.
int hypermesh_asset_address_shard_index(const uint8_t *bytes16);

// Return `HM_VERIFY_OK` (1) if the address in `bytes16` carries the HyperMesh
// `fd48:4d00` prefix, `HM_VERIFY_FAIL` (0) if not, or `HM_ERR_NULL`.
//
// # Safety
//
// `bytes16` must point to 16 readable bytes.
int hypermesh_asset_address_is_hypermesh(const uint8_t *bytes16);

// Compute the BLAKE3 content hash of `data` (`data_len` bytes) into `out32`.
//
// This is the content-addressing primitive: the asset's identity is
// `BLAKE3(payload)`. Use it to derive the `content_hash32` argument to
// [`hypermesh_asset_address_new`].
//
// # Safety
//
// `data` must point to `data_len` readable bytes; `out32` to 32 writable bytes.
int hypermesh_compute_content_hash(const uint8_t *data, uintptr_t data_len, uint8_t *out32);

// Verify the mirror invariant: `BLAKE3(data) == hash32` (R4).
//
// Returns `HM_VERIFY_OK` (1) if the content matches the claimed hash,
// `HM_VERIFY_FAIL` (0) if it does not, or `HM_ERR_NULL`.
//
// This is the same integrity check the internal pipeline runs before trusting
// any asset payload — exposed directly so C consumers cannot bypass it.
//
// # Safety
//
// `hash32` must point to 32 readable bytes; `data` to `data_len` readable bytes.
int hypermesh_verify_content_hash(const uint8_t *hash32, const uint8_t *data, uintptr_t data_len);

// Get the current blockchain height as a JSON number.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_blockchain_height(hypermesh_client_t *client);

// Get a block by index. Returns block info as a JSON string.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_blockchain_block(hypermesh_client_t *client, uint64_t index);

// Fetch the caller's Caesar wallet info. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_caesar_wallet(hypermesh_client_t *client);

// Fetch the current Caesar balance. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_caesar_balance(hypermesh_client_t *client);

// Fetch recent Caesar transactions. Returns JSON array.
//
// `limit` controls the maximum number of transactions returned (0 = default).
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_caesar_transactions(hypermesh_client_t *client, uint32_t limit);

// Fetch accumulated Caesar rewards. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_caesar_rewards(hypermesh_client_t *client);

// Route a Caesar EVP packet to a destination. Returns JSON result.
//
// `destination` is the target node or address.
// `amount_grams` is the gold-gram equivalent value to send.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `destination` must be a valid null-terminated UTF-8 string.
char *hypermesh_caesar_route_packet(hypermesh_client_t *client,
                                    const char *destination,
                                    double amount_grams);

// Fetch the current Caesar Governor parameters. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_caesar_governor_params(hypermesh_client_t *client);

// Browse catalog packages. Returns JSON with paginated results.
//
// `query` is an optional search filter (may be NULL for unfiltered).
// `page` is the zero-based page number.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `query` must be a valid null-terminated UTF-8 string or NULL.
char *hypermesh_catalog_browse(hypermesh_client_t *client, const char *query, uint32_t page);

// Search catalog packages by query string. Returns JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `query` must be a valid null-terminated UTF-8 string.
char *hypermesh_catalog_search(hypermesh_client_t *client, const char *query);

// Get detailed info about a specific catalog package. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `name` must be a valid null-terminated UTF-8 string.
char *hypermesh_catalog_package_info(hypermesh_client_t *client, const char *name);

// Fetch catalog registry statistics. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_catalog_registry_stats(hypermesh_client_t *client);

// Show the full daemon config as a JSON string.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_config_show(hypermesh_client_t *client);

// Get a single config value by key. Returns the value as a JSON string.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `key` must be a valid null-terminated UTF-8 string.
char *hypermesh_config_get(hypermesh_client_t *client, const char *key);

// Publish an asset payload: compute its content address AND produce a
// FALCON-signed [`WireSignedProof`] for this identity.
//
// This performs the THREE distinct operations of a publish without conflating
// them (see module docs):
//
// 1. `blake3_out = BLAKE3(bytes)` — the 32-byte content address (R4). Written
//    to `blake3_out` (must point to 32 writable bytes).
// 2. Build a [`WireSignedProof`] — FALCON-1024 over `BLAKE3(proof_bytes ||
//    nonce)` where `proof_bytes` is the serialized four-proof StateProof — the
//    SAME wire envelope the node exchanges. Serialized (JSON) into `proof_out`
//    using the two-call length pattern.
//
// The FALCON signature inside the proof is over the proof digest, NOT over the
// raw `bytes`.
//
// # Two-call pattern for `proof_out`
//
// Pass `proof_out = NULL` (any `proof_cap`) to learn the required serialized
// length via `*proof_len`, then call again with a buffer of that size. The
// content address is still computed and written to `blake3_out` on the sizing
// call, so a caller can obtain the hash and size in one pass.
//
// IMPORTANT: every call regenerates a FRESH `WireSignedProof` (new nonce, new
// timestamps, new system readings), so the serialized length can differ
// slightly between the sizing call and the real call. A robust caller loops:
// if the real call returns `HM_ERR_BUFFER_TOO_SMALL`, grow the buffer to the
// newly reported `*proof_len` and retry (or simply oversize the buffer). The
// content address in `blake3_out` is stable — only the proof envelope drifts.
//
// # Returns
//
// `HM_OK` on success (content hash written, proof serialized, `*proof_len` set),
// `HM_ERR_BUFFER_TOO_SMALL` if `proof_out` is too small (`*proof_len` = required),
// `HM_ERR_NULL` for a NULL required argument, or `HM_ERR_INTERNAL` on a crypto
// or serialization failure (message in `hypermesh_last_error`).
//
// # Safety
//
// `identity` must be a valid handle; `bytes` must point to `len` readable
// bytes (or be NULL iff `len == 0`); `blake3_out` must point to 32 writable
// bytes; if `proof_out` is non-NULL it must have `proof_cap` writable bytes;
// `proof_len` must be a valid pointer.
int hypermesh_publish(const hypermesh_identity_t *identity,
                      const uint8_t *bytes,
                      uintptr_t len,
                      uint8_t *blake3_out,
                      uint8_t *proof_out,
                      uintptr_t proof_cap,
                      uintptr_t *proof_len);

// Verify a [`WireSignedProof`] exactly as `TrustChainProofProvider::validate_proof`
// does — FALCON verify over `BLAKE3(proof_bytes || nonce)` against the embedded
// signer pubkey, then inner four-proof [`StateProof`] structural validation.
//
// This is the contract-named verify seam. It delegates to the existing
// [`crate::api::identity::hypermesh_signed_proof_verify`], which already
// implements the exact `validate_proof` semantics — no duplicated logic.
//
// Verify-only: holds no key, cannot mint. A forged or tampered proof returns
// `HM_VERIFY_FAIL`.
//
// # Returns
//
// `HM_VERIFY_OK` (1) if authentic, `HM_VERIFY_FAIL` (0) if the signature or
// inner proof is invalid, `HM_ERR_NULL` if `wire_proof` is NULL, or
// `HM_ERR_INVALID` (negative) on malformed input.
//
// # Safety
//
// `wire_proof` must point to `wire_len` readable bytes.
int hypermesh_verify_proof(const uint8_t *wire_proof, uintptr_t wire_len);

// Return the FALCON-1024 detached signature length in bytes (PQClean: 1280).
//
// PQClean is authoritative for the concrete sizes:
// - signature: 1280 bytes (this function)
// - public key: 1793 bytes
// - secret key: 2305 bytes
//
// A consumer sizing a buffer for the FALCON signature inside a
// [`WireSignedProof`] should use this rather than a hardcoded constant.
uintptr_t hypermesh_falcon_signature_max_len(void);

// List deployed dashboards as a JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_dashboard_list(hypermesh_client_t *client);

// Deploy a dashboard from the given path. Returns result as JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `path` must be a valid null-terminated UTF-8 string.
char *hypermesh_dashboard_deploy(hypermesh_client_t *client, const char *path);

// Resolve a DNS name. Returns the address as a C string.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `name` must be a valid null-terminated UTF-8 string.
char *hypermesh_dns_resolve(hypermesh_client_t *client, const char *name);

// List all DNS entries as a JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_dns_list(hypermesh_client_t *client);

// Register a DNS name pointing to the given address.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `name` and `addr` must be valid null-terminated UTF-8 strings.
char *hypermesh_dns_register(hypermesh_client_t *client, const char *name, const char *addr);

// List registered domains as a JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_domain_list(hypermesh_client_t *client);

// Register a domain with the given name and privacy mode.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `name` and `privacy` must be valid null-terminated UTF-8 strings.
char *hypermesh_domain_register(hypermesh_client_t *client, const char *name, const char *privacy);

// Fetch current node capacity metrics. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_engauge_capacity(hypermesh_client_t *client);

// Fetch current traffic statistics. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_engauge_traffic(hypermesh_client_t *client);

// Fetch marketplace resource pool info. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_engauge_marketplace(hypermesh_client_t *client);

// Fetch detailed node-level metrics. Returns JSON.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_engauge_node_metrics(hypermesh_client_t *client);

// Fetch active resource leases. Returns JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_engauge_leases(hypermesh_client_t *client);

// Generate a fresh post-quantum identity (FALCON-1024 + Kyber-1024).
//
// Returns an opaque handle the caller must free with
// [`hypermesh_identity_free`], or `NULL` on error (see `hypermesh_last_error`).
hypermesh_identity_t *hypermesh_identity_generate(void);

// Load an identity from `data_dir`, generating and persisting one if absent.
//
// `data_dir` is a null-terminated UTF-8 path. Returns an opaque handle the
// caller must free with [`hypermesh_identity_free`], or `NULL` on error.
//
// # Safety
//
// `data_dir` must be a valid null-terminated UTF-8 string.
hypermesh_identity_t *hypermesh_identity_load(const char *data_dir);

// Free an identity handle. Passing `NULL` is a safe no-op.
//
// # Safety
//
// `identity` must be a pointer returned by `hypermesh_identity_generate` /
// `hypermesh_identity_load`, or `NULL`. Must not be freed twice.
void hypermesh_identity_free(hypermesh_identity_t *identity);

// Write the node ID (64-char BLAKE3 hex of the FALCON public key) as a
// null-terminated string into `out` (capacity `out_cap` bytes).
//
// Returns `HM_OK` on success, `HM_ERR_BUFFER_TOO_SMALL` if `out_cap < 65`
// (64 chars + NUL), or a negative error code.
//
// # Safety
//
// `identity` must be a valid handle; `out` must point to at least `out_cap`
// writable bytes.
int hypermesh_identity_node_id(const hypermesh_identity_t *identity, char *out, uintptr_t out_cap);

// Copy the raw FALCON-1024 public key bytes into `out`.
//
// Two-call pattern: pass `out = NULL` (any `out_cap`) to learn the required
// length via `*out_len`, then call again with a buffer of that size.
//
// Returns `HM_OK` on success (with `*out_len` = bytes written),
// `HM_ERR_BUFFER_TOO_SMALL` if `out_cap` is too small (`*out_len` = required).
//
// # Safety
//
// `identity` must be valid; if non-NULL, `out` must have `out_cap` writable
// bytes; `out_len` must be a valid pointer.
int hypermesh_identity_public_key(const hypermesh_identity_t *identity,
                                  uint8_t *out,
                                  uintptr_t out_cap,
                                  uintptr_t *out_len);

// Copy the raw Kyber-1024 public key bytes into `out` (same two-call pattern
// as [`hypermesh_identity_public_key`]). Peers use this to encrypt assets FOR
// this node (KEM encapsulation).
//
// # Safety
//
// See [`hypermesh_identity_public_key`].
int hypermesh_identity_kyber_public_key(const hypermesh_identity_t *identity,
                                        uint8_t *out,
                                        uintptr_t out_cap,
                                        uintptr_t *out_len);

// Sign `data` (`data_len` bytes) with the identity's FALCON-1024 secret key,
// writing the detached signature into `out` (two-call pattern).
//
// Returns `HM_OK` with `*out_len` = signature length on success.
//
// # Safety
//
// `identity` must be valid; `data` must have `data_len` readable bytes; if
// `out` is non-NULL it must have `out_cap` writable bytes; `out_len` valid.
int hypermesh_identity_sign(const hypermesh_identity_t *identity,
                            const uint8_t *data,
                            uintptr_t data_len,
                            uint8_t *out,
                            uintptr_t out_cap,
                            uintptr_t *out_len);

// Verify a FALCON-1024 detached signature (stateless — no identity handle).
//
// Returns `HM_VERIFY_OK` (1) if the signature is authentic for `data` under
// `pubkey`, `HM_VERIFY_FAIL` (0) if not authentic, or a negative error code
// on malformed input.
//
// # Safety
//
// `pubkey`/`data`/`signature` must point to their respective lengths of
// readable bytes.
int hypermesh_verify_signature(const uint8_t *pubkey,
                               uintptr_t pubkey_len,
                               const uint8_t *data,
                               uintptr_t data_len,
                               const uint8_t *signature,
                               uintptr_t signature_len);

// Verify a `WireSignedProof` envelope — the on-the-wire PoS-signed proof.
//
// `wire` is the JSON-serialized `WireSignedProof` (`wire_len` bytes). This
// runs the SAME verification the internal Rust path runs:
// 1. Recompute `BLAKE3(proof_bytes || nonce)`.
// 2. Verify the FALCON-1024 detached signature against the embedded pubkey.
// 3. Validate the inner four-proof `StateProof` (binary pass/fail).
//
// Returns `HM_VERIFY_OK` (1) if authentic, `HM_VERIFY_FAIL` (0) if the
// signature or inner proof is invalid, or a negative error on malformed input.
//
// This is the PoS gate exposed over C: a forged or tampered proof returns 0,
// and no ABI function can mint a valid proof without the signing secret key.
//
// # Safety
//
// `wire` must point to `wire_len` readable bytes.
int hypermesh_signed_proof_verify(const uint8_t *wire, uintptr_t wire_len);

// List connected peers as a JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_peers(hypermesh_client_t *client);

// Fetch the current node status as a JSON string.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_status(hypermesh_client_t *client);

// Get this node's topology info as a JSON string.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_topology_info(hypermesh_client_t *client);

// List all TrustChain certificates. Returns JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_trustchain_certificates(hypermesh_client_t *client);

// Issue a new certificate for the given subject and scope. Returns JSON.
//
// `subject` is the entity name (e.g. node ID or domain).
// `scope` is the certificate scope (e.g. "device", "network").
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `subject` and `scope` must be valid null-terminated UTF-8 strings.
char *hypermesh_trustchain_issue(hypermesh_client_t *client,
                                 const char *subject,
                                 const char *scope);

// Validate a PEM-encoded certificate. Returns JSON validation result.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `cert_pem` must be a valid null-terminated UTF-8 string.
char *hypermesh_trustchain_validate(hypermesh_client_t *client, const char *cert_pem);

// Revoke a certificate by its ID. Returns JSON result.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
// `cert_id` must be a valid null-terminated UTF-8 string.
char *hypermesh_trustchain_revoke(hypermesh_client_t *client, const char *cert_id);

// List TrustChain DNS zones. Returns JSON array.
//
// # Safety
//
// `client` must be a valid pointer returned by `hypermesh_connect`.
char *hypermesh_trustchain_dns_zones(hypermesh_client_t *client);

// Connect to a running HyperMesh daemon.
//
// `socket_path` may be `NULL` to use the default 3-tier fallback
// (`$HYPERMESH_SOCK` / `$XDG_RUNTIME_DIR/hypermesh/ctl.sock` /
// `~/.hypermesh/ctl.sock`).
//
// Returns an opaque pointer the caller must eventually pass to
// `hypermesh_disconnect`, or `NULL` on error.
//
// # Safety
//
// `socket_path` must be a valid null-terminated UTF-8 string or `NULL`.
hypermesh_client_t *hypermesh_connect(const char *socket_path);

// Disconnect and free a client handle.
//
// After this call the pointer is invalid and must not be used.
//
// # Safety
//
// `client` must be a pointer previously returned by `hypermesh_connect`,
// or `NULL` (which is a no-op).
void hypermesh_disconnect(hypermesh_client_t *client);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* HYPERMESH_H */
