// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — capability-token persistence for the mobile shell.
//
// The capability token issued by `auth.create_session` is the bearer
// credential for every subsequent SDK call (sent as the
// `X-HyperMesh-Capability` header by `@hypermesh/sdk`'s HttpClient).
//
// We store it in Expo SecureStore — same trust boundary as the device
// private key. The expiry timestamp is also tracked so the SDK client
// can pre-emptively refresh before the daemon emits -32004.

import * as SecureStore from "expo-secure-store";

const STORE_KEY_TOKEN = "hypermesh.session.token";
const STORE_KEY_EXPIRES_AT = "hypermesh.session.expires_at_secs";
const STORE_KEY_SESSION_ID = "hypermesh.session.id";
const STORE_KEY_BASE_URL = "hypermesh.session.base_url";
const STORE_KEY_MODE = "hypermesh.session.mode";

/**
 * Three connection modes from Phase K. Persisted so the re-auth flow
 * after token expiry knows which gateway to talk to.
 */
export type ConnectionMode =
  | "trust-gateway" // trust.hypermesh.online relay
  | "private-domain" // yourname.hypermesh user-owned gateway
  | "self-hosted"; // user-supplied URL

export interface StoredSession {
  /**
   * Base64-encoded `CapabilityToken` JSON. Pass directly into
   * `HyperMeshClient.setCapabilityToken` or the `sessionToken` option.
   */
  token: string;
  /** Unix-seconds expiry (compared against `Date.now()/1000`). */
  expiresAtSecs: number;
  /** Session id for `auth.revoke_session`. */
  sessionId: string;
  /** Gateway base URL. */
  baseUrl: string;
  /** Which Phase K mode minted this session. */
  mode: ConnectionMode;
}

/** Persist a freshly-issued session. */
export async function saveSession(session: StoredSession): Promise<void> {
  await SecureStore.setItemAsync(STORE_KEY_TOKEN, session.token);
  await SecureStore.setItemAsync(
    STORE_KEY_EXPIRES_AT,
    String(session.expiresAtSecs),
  );
  await SecureStore.setItemAsync(STORE_KEY_SESSION_ID, session.sessionId);
  await SecureStore.setItemAsync(STORE_KEY_BASE_URL, session.baseUrl);
  await SecureStore.setItemAsync(STORE_KEY_MODE, session.mode);
}

/** Read the stored session, or `null` if none / corrupted. */
export async function loadSession(): Promise<StoredSession | null> {
  const token = await SecureStore.getItemAsync(STORE_KEY_TOKEN);
  const expRaw = await SecureStore.getItemAsync(STORE_KEY_EXPIRES_AT);
  const sessionId = await SecureStore.getItemAsync(STORE_KEY_SESSION_ID);
  const baseUrl = await SecureStore.getItemAsync(STORE_KEY_BASE_URL);
  const mode = (await SecureStore.getItemAsync(
    STORE_KEY_MODE,
  )) as ConnectionMode | null;

  if (!token || !expRaw || !sessionId || !baseUrl || !mode) {
    return null;
  }

  const expiresAtSecs = Number.parseInt(expRaw, 10);
  if (!Number.isFinite(expiresAtSecs)) {
    return null;
  }

  return { token, expiresAtSecs, sessionId, baseUrl, mode };
}

/**
 * Returns the stored session iff it is still valid and not within
 * the refresh window. The SDK wrapper consults this before every
 * call; if it returns `null`, the wrapper drops back to the
 * re-auth flow.
 */
export async function loadValidSession(
  refreshWindowSecs = 60,
): Promise<StoredSession | null> {
  const s = await loadSession();
  if (!s) return null;
  const now = Math.floor(Date.now() / 1000);
  if (s.expiresAtSecs <= now + refreshWindowSecs) {
    return null;
  }
  return s;
}

/** Drop any persisted session — used on -32004 and on user sign-out. */
export async function clearSession(): Promise<void> {
  await SecureStore.deleteItemAsync(STORE_KEY_TOKEN);
  await SecureStore.deleteItemAsync(STORE_KEY_EXPIRES_AT);
  await SecureStore.deleteItemAsync(STORE_KEY_SESSION_ID);
  await SecureStore.deleteItemAsync(STORE_KEY_BASE_URL);
  await SecureStore.deleteItemAsync(STORE_KEY_MODE);
}
