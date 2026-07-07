/*
 * Copyright 2026 Hypermesh Foundation. All rights reserved.
 * Licensed under the Business Source License 1.1.
 *
 * C smoke test for the HyperMesh direct C ABI.
 *
 * Exercises the daemon-less paths only (no running node required):
 *   1. AssetAddress: build from a content hash + coords, read back the
 *      content fingerprint / coords / shard, and round-trip via IPv6 text.
 *   2. Content-hash mirror invariant: compute BLAKE3(data), verify it, and
 *      confirm tampered data is rejected.
 *   3. Identity: generate a FALCON-1024 identity, read the node id, sign a
 *      message, and verify the signature (valid + tampered).
 *
 * Build (linking the staticlib or the cdylib) is driven by run_smoke.sh.
 */

#include "hypermesh.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int failures = 0;

static void check(int cond, const char *what) {
    if (cond) {
        printf("  PASS: %s\n", what);
    } else {
        printf("  FAIL: %s\n", what);
        failures++;
    }
}

static int test_asset_address(void) {
    printf("[1] AssetAddress construct / parse / ipv6 round-trip\n");

    /* A deterministic 32-byte "content hash" (0,1,2,...,31). */
    uint8_t content_hash[32];
    for (int i = 0; i < 32; i++) {
        content_hash[i] = (uint8_t)i;
    }

    uint8_t addr[16];
    int rc = hypermesh_asset_address_new(10, -20, 30, content_hash, 3, addr);
    check(rc == HM_OK, "hypermesh_asset_address_new returns HM_OK");

    /* Prefix check. */
    rc = hypermesh_asset_address_is_hypermesh(addr);
    check(rc == HM_VERIFY_OK, "address carries fd48:4d00 prefix");

    /* Coords round-trip. */
    int64_t x = 0, y = 0, z = 0;
    rc = hypermesh_asset_address_coords(addr, &x, &y, &z);
    check(rc == HM_OK && x == 10 && y == -20 && z == 30,
          "matrix coords read back as (10, -20, 30)");

    /* Shard index. */
    rc = hypermesh_asset_address_shard_index(addr);
    check(rc == 3, "shard index reads back as 3");

    /* Content fingerprint: first 5 bytes are the content hash, byte 5 is
     * (hash[5] high nibble | shard). */
    uint8_t fp[6];
    rc = hypermesh_asset_address_fingerprint(addr, fp);
    int fp_ok = (rc == HM_OK) &&
                (memcmp(fp, content_hash, 5) == 0) &&
                (fp[5] == ((content_hash[5] & 0xF0) | 3));
    check(fp_ok, "content fingerprint matches the embedded content hash");

    /* IPv6 text round-trip. */
    char ipv6[64];
    rc = hypermesh_asset_address_to_ipv6(addr, ipv6, sizeof(ipv6));
    check(rc == HM_OK, "address formats to IPv6 text");
    printf("       address = %s\n", ipv6);

    uint8_t back[16];
    rc = hypermesh_asset_address_from_ipv6(ipv6, back);
    check(rc == HM_OK && memcmp(addr, back, 16) == 0,
          "IPv6 text parses back to the identical 16 bytes");

    /* Non-HyperMesh IPv6 must be rejected. */
    rc = hypermesh_asset_address_from_ipv6("2001:db8::1", back);
    check(rc == HM_ERR_INVALID, "non-HyperMesh IPv6 is rejected");

    return 0;
}

static int test_content_hash(void) {
    printf("[2] Content-hash mirror invariant (BLAKE3)\n");

    const char *payload = "the asset payload bytes";
    size_t len = strlen(payload);

    uint8_t hash[32];
    int rc = hypermesh_compute_content_hash((const uint8_t *)payload, len, hash);
    check(rc == HM_OK, "hypermesh_compute_content_hash returns HM_OK");

    rc = hypermesh_verify_content_hash(hash, (const uint8_t *)payload, len);
    check(rc == HM_VERIFY_OK, "content matches its BLAKE3 hash (mirror OK)");

    const char *tampered = "the asset payload byteX";
    rc = hypermesh_verify_content_hash(hash, (const uint8_t *)tampered,
                                       strlen(tampered));
    check(rc == HM_VERIFY_FAIL, "tampered content fails the mirror invariant");

    return 0;
}

static int test_identity(void) {
    printf("[3] Identity: generate / sign / verify (FALCON-1024)\n");

    hypermesh_identity_t *id = hypermesh_identity_generate();
    check(id != NULL, "hypermesh_identity_generate returns a handle");
    if (id == NULL) {
        return 1;
    }

    char node_id[128];
    int rc = hypermesh_identity_node_id(id, node_id, sizeof(node_id));
    check(rc == HM_OK && strlen(node_id) == 64,
          "node id is a 64-char BLAKE3 hex string");
    printf("       node_id = %s\n", node_id);

    /* Public key (two-call length pattern). */
    size_t pk_len = 0;
    rc = hypermesh_identity_public_key(id, NULL, 0, &pk_len);
    check(rc == HM_ERR_BUFFER_TOO_SMALL && pk_len > 0,
          "public key length query works");
    uint8_t pk[4096];
    rc = hypermesh_identity_public_key(id, pk, sizeof(pk), &pk_len);
    check(rc == HM_OK, "public key copied into buffer");

    /* Sign a message (two-call length pattern). */
    const char *msg = "hypermesh ffi smoke-test message";
    size_t msg_len = strlen(msg);
    size_t sig_len = 0;
    hypermesh_identity_sign(id, (const uint8_t *)msg, msg_len, NULL, 0, &sig_len);
    check(sig_len > 0, "signature length query works");

    uint8_t sig[4096];
    rc = hypermesh_identity_sign(id, (const uint8_t *)msg, msg_len, sig,
                                 sizeof(sig), &sig_len);
    check(rc == HM_OK, "message signed with FALCON-1024");

    /* Verify: valid. */
    rc = hypermesh_verify_signature(pk, pk_len, (const uint8_t *)msg, msg_len,
                                    sig, sig_len);
    check(rc == HM_VERIFY_OK, "valid signature verifies");

    /* Verify: tampered message must fail. */
    const char *bad = "tampered smoke-test message xxxx";
    rc = hypermesh_verify_signature(pk, pk_len, (const uint8_t *)bad,
                                    strlen(bad), sig, sig_len);
    check(rc == HM_VERIFY_FAIL, "tampered message fails verification");

    hypermesh_identity_free(id);
    check(1, "identity handle freed without crash");

    return 0;
}

int main(void) {
    printf("=== HyperMesh C ABI smoke test ===\n\n");

    test_asset_address();
    printf("\n");
    test_content_hash();
    printf("\n");
    test_identity();
    printf("\n");

    if (failures == 0) {
        printf("ALL CHECKS PASSED\n");
        return 0;
    }
    printf("%d CHECK(S) FAILED\n", failures);
    return 1;
}
