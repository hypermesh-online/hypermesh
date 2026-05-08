// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — challenge signing for capability-token issuance.
//
// FLOW (any of the 3 modes — Trust / Private / Self-hosted):
//
//   1. Mobile fetches a fresh challenge from the gateway:
//        POST /api/v1/auth/challenge → { challenge_hex, expires_at }
//   2. Mobile signs `challenge_hex` bytes with the device key.
//   3. Mobile calls `auth.create_session` with:
//        { device_pubkey, requested_capabilities, ttl_secs,
//          challenge_hex, signature_hex }
//   4. Daemon verifies signature → issues `CapabilityToken`.
//   5. Mobile stashes token via `TokenStore.saveToken`.
//
// As with `DeviceFingerprint.ts`, the actual signing primitive is an
// ECDSA-P256 placeholder for C.4 alpha. The C.4.5 sprint replaces this
// with FALCON-1024 via UniFFI. The function shape stays.

import { _readPrivateKeyHex } from "./DeviceFingerprint";

function hexToBytes(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) throw new Error("hex string has odd length");
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Sign a hex-encoded challenge using the device key.
 *
 * Returns the signature as hex. Throws if no fingerprint exists
 * (caller must `loadOrCreateFingerprint()` first).
 *
 * NOTE: this is an ECDSA-P256 placeholder. The trust gateway will
 * reject these signatures until the FALCON-1024 verifier is matched
 * by the FALCON-1024 signer (C.4.5). The Connect screen surfaces this
 * fact to the user via the "alpha placeholder" badge.
 */
export async function signChallenge(challengeHex: string): Promise<string> {
  const privHex = await _readPrivateKeyHex();
  if (!privHex) {
    throw new Error(
      "no device private key — call loadOrCreateFingerprint() first",
    );
  }

  const subtle = (globalThis as { crypto?: { subtle?: SubtleCrypto } }).crypto
    ?.subtle;
  const challenge = hexToBytes(challengeHex);

  if (!subtle) {
    // Insecure fallback — same warning as in DeviceFingerprint.
    // eslint-disable-next-line no-console
    console.warn(
      "[hypermesh] WebCrypto unavailable — emitting insecure stub signature.",
    );
    const sig = new Uint8Array(64);
    for (let i = 0; i < sig.length; i++) sig[i] = (challenge[i % challenge.length] ^ i) & 0xff;
    return bytesToHex(sig);
  }

  const privBytes = hexToBytes(privHex);
  const privKey = await subtle.importKey(
    "pkcs8",
    privBytes,
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["sign"],
  );

  const signature = await subtle.sign(
    { name: "ECDSA", hash: { name: "SHA-256" } },
    privKey,
    challenge,
  );

  return bytesToHex(new Uint8Array(signature));
}

/**
 * Phase K.2 capability requested when minting a session.
 *
 * Mirrors `caesar/src/auth/capability.rs` — the daemon will reject
 * unrecognised tags with -32004.
 */
export type RequestedCapability =
  | "viewonly"
  | "wallet"
  | "assetwrite"
  | "admin";

/** Default scope for the mobile shell — read-only + wallet send. */
export const DEFAULT_REQUESTED_CAPS: ReadonlyArray<RequestedCapability> = [
  "viewonly",
  "wallet",
];
