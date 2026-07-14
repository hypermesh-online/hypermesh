<!-- Written by Richard Christopher, Copyright 2026 HyperMesh Foundation -->

# HyperMesh C ABI — Certs + Assets (P9)

Stable C ABI over the SAME validated Rust paths, for non-Rust consumers (mobile native, other-language SDKs, embedded). Header: `include/hypermesh.h` (cbindgen-generated on build). Link the `cdylib` or `staticlib` (`crate-type = ["cdylib","staticlib"]`).

## Two families
1. **Direct (in-process, no daemon)** — local, self-contained operations:
   - Identity/certs: `hypermesh_identity_generate/load/free`, `_node_id`, `_public_key`, `_kyber_public_key`, `_sign`; `hypermesh_verify_signature`; `hypermesh_signed_proof_verify` (the PoS gate).
   - Assets: `hypermesh_asset_address_new/to_ipv6/from_ipv6/fingerprint/coords/shard_index/is_hypermesh`; `hypermesh_compute_content_hash`; `hypermesh_verify_content_hash` (mirror invariant).
2. **Daemon IPC (pre-existing)** — full PoS-gated pipeline: `hypermesh_asset_store/fetch`, `hypermesh_trustchain_issue/validate`, etc. Require a running node.

## Return codes
`HM_OK=0`; checks return `HM_VERIFY_OK=1`/`HM_VERIFY_FAIL=0`; errors `HM_ERR_NULL=-1`, `HM_ERR_INVALID=-2`, `HM_ERR_BUFFER_TOO_SMALL=-3`, `HM_ERR_INTERNAL=-4`. Details in the thread-local `hypermesh_last_error`. Byte/string outputs use the two-call length pattern (NULL out → learn `*out_len`, retry sized).

## PoS / mirror invariant — NOT bypassable (verified)
- `hypermesh_signed_proof_verify` runs the identical logic as `TrustChainProofProvider::validate_proof`: recompute `BLAKE3(proof_bytes‖nonce)`, verify the FALCON-1024 detached signature, run inner `StateProof::validate()`. It VERIFIES; it cannot MINT.
- The only signing path (`hypermesh_identity_sign`) needs the secret key held INSIDE the opaque `hypermesh_identity_t` handle — the secret key NEVER crosses the C boundary (grep-confirmed: zero `secret_key` in the ABI).
- `hypermesh_verify_content_hash` = `validate_blake3_hash` (BLAKE3(data)==hash); `hypermesh_asset_address_new` binds the content hash into the address.

## Safety
Every handle-returning fn has a matching free; all pointers null-checked; every `extern "C"` fn wrapped in `catch_unwind` (no panic crosses into C). AssetAddress is a `[u8;16]` value type (no handle).

## Known gap (documented, not faked)
No daemon-less asset register/fetch — retrieval needs the live blockmatrix node (async, storage, reflector pool). The direct ABI exposes local address/hash/verify; store/fetch stay behind daemon IPC. A future embeddable `retrieve(AssetAddress)->payload` (the transmission-payload retrieval, P3) would close this. Cert ISSUANCE (`TrustChainCA::issue_certificate`) is likewise async/CA-dependent — direct ABI exposes identity/sign/verify primitives; issuance stays IPC.

## Verified (2026-07-06)
`cargo build -p hypermesh-ffi` clean (6m24s cold). `cargo test -p hypermesh-ffi` 32 pass / 0 fail. C smoke test (`tests/smoke.c` linked against `libhypermesh_ffi.a`) — ALL CHECKS PASSED, exit 0: AssetAddress construct/parse/IPv6 round-trip + non-HyperMesh reject; BLAKE3 verify + tamper-reject; FALCON-1024 generate/sign/verify + tampered-message reject.
