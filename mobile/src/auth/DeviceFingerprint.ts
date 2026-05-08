// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — Device fingerprint generation + persistence.
//
// SCOPE & DEFERRED WORK:
//
// The TARGET architecture is FALCON-1024 device keys generated on the
// phone via a Rust UniFFI binding. The phone signs trust gateway
// challenges with FALCON, the gateway verifies, and a capability token
// is issued bound to the device pubkey.
//
// FALCON-1024 keygen requires the `pqcrypto-falcon` crate, which has
// no React-Native-compatible WASM/JS shim today. Wrapping it via
// UniFFI (`hypermesh-ffi` crate → C → Swift/Kotlin bridge) is the
// C.4.5 sprint.
//
// For C.4 alpha, we ship a WebCrypto-based ECDSA P-256 placeholder
// keypair. The shape of the API is identical to the future
// FALCON-1024 implementation — only the underlying primitive changes.
// Trust gateway challenge verification will reject these keys until
// the FALCON binding lands; the structure is in place so that swapping
// the impl is mechanical.
//
// Storage: private key bytes are persisted in Expo SecureStore, which
// maps to iOS Keychain / Android Keystore. The platform key store is
// the trust boundary — even the app process cannot read the raw key
// material without going through SecureStore.

import * as SecureStore from "expo-secure-store";

const STORE_KEY_PRIVATE = "hypermesh.device.private_key";
const STORE_KEY_PUBLIC = "hypermesh.device.public_key";
const STORE_KEY_ALG = "hypermesh.device.alg";

/**
 * Algorithm tag stored alongside the keypair so a future migration
 * (ECDSA placeholder → FALCON-1024 real) can detect old keys and
 * regenerate.
 */
export type DeviceKeyAlgorithm = "ecdsa-p256-placeholder" | "falcon-1024";

/** Active algorithm for newly-generated keys. C.4.5 will flip this. */
export const ACTIVE_ALG: DeviceKeyAlgorithm = "ecdsa-p256-placeholder";

export interface DeviceFingerprint {
  /** Public key as hex (lowercase, no 0x prefix). */
  publicKeyHex: string;
  /** Algorithm under which this fingerprint was generated. */
  algorithm: DeviceKeyAlgorithm;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function hexToBytes(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) {
    throw new Error("hex string has odd length");
  }
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

/**
 * Phase C.4 placeholder keygen via WebCrypto P-256.
 *
 * In React Native >= 0.71 with Expo SDK 50, `crypto.subtle` is
 * available via `expo-standard-web-crypto` polyfill (added implicitly
 * by Expo). If unavailable at runtime, we fall back to a
 * cryptographically-weak `Math.random()` stub — flagged loudly so the
 * placeholder cannot accidentally ship as production auth.
 */
async function generatePlaceholderKeypair(): Promise<{
  privateHex: string;
  publicHex: string;
}> {
  const subtle = (globalThis as { crypto?: { subtle?: SubtleCrypto } }).crypto
    ?.subtle;

  if (subtle) {
    const keyPair = await subtle.generateKey(
      { name: "ECDSA", namedCurve: "P-256" },
      true,
      ["sign", "verify"],
    );
    const pubRaw = new Uint8Array(
      await subtle.exportKey("raw", keyPair.publicKey),
    );
    const privPkcs8 = new Uint8Array(
      await subtle.exportKey("pkcs8", keyPair.privateKey),
    );
    return {
      privateHex: bytesToHex(privPkcs8),
      publicHex: bytesToHex(pubRaw),
    };
  }

  // Fallback: emit weak random bytes with a flag.
  // PHASE C.4.5 MUST replace this path entirely.
  // eslint-disable-next-line no-console
  console.warn(
    "[hypermesh] WebCrypto unavailable — using insecure placeholder keypair. " +
      "DO NOT ship production builds against this branch.",
  );
  const priv = new Uint8Array(32);
  const pub = new Uint8Array(65);
  for (let i = 0; i < priv.length; i++) priv[i] = Math.floor(Math.random() * 256);
  for (let i = 0; i < pub.length; i++) pub[i] = Math.floor(Math.random() * 256);
  pub[0] = 0x04; // uncompressed point marker
  return { privateHex: bytesToHex(priv), publicHex: bytesToHex(pub) };
}

/**
 * Load the existing device fingerprint, or generate one if none exists.
 *
 * Idempotent — repeated calls return the same fingerprint.
 */
export async function loadOrCreateFingerprint(): Promise<DeviceFingerprint> {
  const existingPub = await SecureStore.getItemAsync(STORE_KEY_PUBLIC);
  const existingAlg = (await SecureStore.getItemAsync(
    STORE_KEY_ALG,
  )) as DeviceKeyAlgorithm | null;

  if (existingPub && existingAlg) {
    return { publicKeyHex: existingPub, algorithm: existingAlg };
  }

  const { privateHex, publicHex } = await generatePlaceholderKeypair();
  await SecureStore.setItemAsync(STORE_KEY_PRIVATE, privateHex);
  await SecureStore.setItemAsync(STORE_KEY_PUBLIC, publicHex);
  await SecureStore.setItemAsync(STORE_KEY_ALG, ACTIVE_ALG);

  return { publicKeyHex: publicHex, algorithm: ACTIVE_ALG };
}

/**
 * Wipe the on-device keypair. Used when the user explicitly
 * disconnects from a node ("Forget this device" / re-onboard).
 *
 * After this call, the next `loadOrCreateFingerprint()` regenerates.
 */
export async function clearFingerprint(): Promise<void> {
  await SecureStore.deleteItemAsync(STORE_KEY_PRIVATE);
  await SecureStore.deleteItemAsync(STORE_KEY_PUBLIC);
  await SecureStore.deleteItemAsync(STORE_KEY_ALG);
}

/**
 * Internal — return the raw private key bytes. Only used by
 * `ChallengeSign.signChallenge`. Never exposed outside the auth
 * module.
 */
export async function _readPrivateKeyHex(): Promise<string | null> {
  return SecureStore.getItemAsync(STORE_KEY_PRIVATE);
}

/**
 * Convenience — display the fingerprint as a short hex preview
 * (first 8 + last 8 chars) for the Connect / Dashboard screens.
 */
export function previewFingerprint(fp: DeviceFingerprint): string {
  const h = fp.publicKeyHex;
  if (h.length <= 16) return h;
  return `${h.slice(0, 8)}…${h.slice(-8)}`;
}

export const __test_internals = { hexToBytes, bytesToHex };
