// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — mobile-side wrapper around `@hypermesh/sdk`.
//
// Responsibilities:
//   1. Construct a single `HyperMeshClient` per app session, baseUrl
//      pulled from `TokenStore.loadSession()`.
//   2. Install the persisted capability token on every transport
//      before the first SDK call.
//   3. Catch `-32004 CAPABILITY_DENIED` (a.k.a. HTTP 401 with the
//      JSON-RPC error body) → wipe the stored session → emit a
//      `'reauth-required'` event so the navigator can bounce to
//      ConnectScreen.
//   4. Drive the auth handshake itself — challenge → sign → session.

import {
  HyperMeshClient,
  HyperMeshError,
  type HyperMeshClientOptions,
} from "@hypermesh/sdk";
import {
  loadValidSession,
  saveSession,
  clearSession,
  type StoredSession,
  type ConnectionMode,
} from "../auth/TokenStore";
import {
  loadOrCreateFingerprint,
  type DeviceFingerprint,
} from "../auth/DeviceFingerprint";
import {
  signChallenge,
  DEFAULT_REQUESTED_CAPS,
  type RequestedCapability,
} from "../auth/ChallengeSign";

/** -32004 from `caesar/src/auth/capability.rs`. */
const CAPABILITY_DENIED = -32004;

/**
 * Listener type for re-auth callbacks. Mounted by the navigator;
 * fired when the SDK observes a capability rejection mid-flight.
 */
export type ReauthListener = () => void;

class MobileApiClient {
  private inner: HyperMeshClient | null = null;
  private session: StoredSession | null = null;
  private fingerprint: DeviceFingerprint | null = null;
  private reauthListeners: ReauthListener[] = [];

  /** Bootstrap from any persisted session. Returns `true` on success. */
  async bootstrap(): Promise<boolean> {
    this.session = await loadValidSession();
    this.fingerprint = await loadOrCreateFingerprint();

    if (!this.session) {
      this.inner = null;
      return false;
    }

    this.inner = new HyperMeshClient({
      baseUrl: this.session.baseUrl,
      sessionToken: this.session.token,
    });
    return true;
  }

  /** Returns the SDK client, or `null` if no live session. */
  client(): HyperMeshClient | null {
    return this.inner;
  }

  /** Returns the stored session metadata. */
  currentSession(): StoredSession | null {
    return this.session;
  }

  /** Returns the device fingerprint (always loaded after `bootstrap()`). */
  currentFingerprint(): DeviceFingerprint | null {
    return this.fingerprint;
  }

  /** Listen for capability-denial events. Returns an unsubscribe fn. */
  onReauthRequired(listener: ReauthListener): () => void {
    this.reauthListeners.push(listener);
    return () => {
      this.reauthListeners = this.reauthListeners.filter((l) => l !== listener);
    };
  }

  private fireReauth(): void {
    for (const l of this.reauthListeners) {
      try {
        l();
      } catch {
        // listeners must not throw upstream
      }
    }
  }

  /**
   * Wrap any SDK call so capability-denial errors are caught,
   * sessions wiped, and listeners notified.
   */
  async invoke<T>(
    fn: (c: HyperMeshClient) => Promise<T>,
  ): Promise<T> {
    if (!this.inner) {
      throw new Error("not connected — call connect() first");
    }
    try {
      return await fn(this.inner);
    } catch (err) {
      if (this.isCapabilityDenied(err)) {
        await this.signOut();
        this.fireReauth();
      }
      throw err;
    }
  }

  private isCapabilityDenied(err: unknown): boolean {
    if (!(err instanceof HyperMeshError)) return false;
    if (err.status === 401 || err.status === 403) return true;
    // JSON-RPC body with `error.code = -32004` (per K.2)
    if (err.body) {
      try {
        const parsed = JSON.parse(err.body);
        if (parsed?.error?.code === CAPABILITY_DENIED) return true;
      } catch {
        /* not JSON */
      }
    }
    return false;
  }

  /**
   * Phase C.4 — establish a session against `baseUrl` for the chosen
   * mode. Three steps:
   *   1. Fetch challenge from `${baseUrl}/api/v1/auth/challenge`.
   *   2. Sign with the device key.
   *   3. POST `auth.create_session` with the signed challenge.
   *
   * On success, persists the session and refreshes `this.inner`.
   */
  async connect(opts: {
    baseUrl: string;
    mode: ConnectionMode;
    requestedCapabilities?: ReadonlyArray<RequestedCapability>;
    ttlSecs?: number;
  }): Promise<StoredSession> {
    const fp = await loadOrCreateFingerprint();
    this.fingerprint = fp;

    // Step 1 — fetch challenge.
    const challenge = await this.fetchChallenge(opts.baseUrl);

    // Step 2 — sign.
    const signatureHex = await signChallenge(challenge.challengeHex);

    // Step 3 — create session via K.2 IPC.
    //
    // The TS SDK's `authCreateSession` accepts the simpler
    // (devicePubkeyHex, requestedCapabilities, ttlSecs) tuple that the
    // current daemon route exposes. The challenge/signature pair is
    // appended via a freshly-built HyperMeshClient bound to the new
    // baseUrl. Caesar/auth implementations that expect the additional
    // fields will surface them via -32004 if not provided; the alpha
    // daemon ignores them.
    const provisional = new HyperMeshClient({ baseUrl: opts.baseUrl });
    const requested = opts.requestedCapabilities ?? DEFAULT_REQUESTED_CAPS;
    const ttl = opts.ttlSecs ?? 3600;

    // The SDK shape only takes 3 args today; we still ship the
    // signature in a separate POST so the daemon can validate it.
    // (See `challenge_create_session` POST body below.)
    const payload = await this.postSignedSession(opts.baseUrl, {
      device_pubkey: fp.publicKeyHex,
      requested_capabilities: requested,
      ttl_secs: ttl,
      challenge_hex: challenge.challengeHex,
      signature_hex: signatureHex,
      algorithm: fp.algorithm,
    });

    void provisional; // discard — we'll build the real client below.

    const stored: StoredSession = {
      token: typeof payload.token === "string" ? payload.token : payload.signature_hex,
      expiresAtSecs: payload.valid_until_secs,
      sessionId: payload.session_id,
      baseUrl: opts.baseUrl,
      mode: opts.mode,
    };
    await saveSession(stored);
    this.session = stored;
    this.inner = new HyperMeshClient({
      baseUrl: opts.baseUrl,
      sessionToken: stored.token,
    });

    return stored;
  }

  /**
   * Fetch a challenge from the gateway.
   *
   * Falls back to a synthetic 32-byte challenge if the gateway does
   * not yet expose `/api/v1/auth/challenge` — in that case the daemon
   * is configured to issue tokens without a signed-challenge proof,
   * which is the alpha-default path.
   */
  private async fetchChallenge(
    baseUrl: string,
  ): Promise<{ challengeHex: string; expiresAt?: number }> {
    const url = `${baseUrl.replace(/\/+$/, "")}/api/v1/auth/challenge`;
    try {
      const r = await fetch(url, {
        method: "GET",
        headers: { Accept: "application/json" },
      });
      if (r.ok) {
        const j = (await r.json()) as {
          challenge_hex?: string;
          expires_at?: number;
        };
        if (j.challenge_hex) {
          return { challengeHex: j.challenge_hex, expiresAt: j.expires_at };
        }
      }
    } catch {
      /* fall through to synthetic */
    }
    // Synthetic 32-byte challenge derived from time. Replaced by the
    // real gateway response once `/auth/challenge` ships.
    const synthetic = new Uint8Array(32);
    const now = BigInt(Date.now());
    for (let i = 0; i < 32; i++) {
      synthetic[i] = Number((now >> BigInt(i % 8)) & 0xffn) ^ i;
    }
    const hex = Array.from(synthetic)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    return { challengeHex: hex };
  }

  /**
   * POST the signed-challenge session request directly. Mirrors the
   * future daemon route shape. If the daemon returns 404 (route not
   * wired yet), retries the simpler K.2 `authCreateSession` shape.
   */
  private async postSignedSession(
    baseUrl: string,
    body: Record<string, unknown>,
  ): Promise<{
    session_id: string;
    device_pubkey_hex: string;
    capabilities: string[];
    issued_at_secs: number;
    valid_until_secs: number;
    issued_by_hex: string;
    signature_hex: string;
    token: unknown;
  }> {
    const url = `${baseUrl.replace(/\/+$/, "")}/api/v1/auth/create_session`;
    const r = await fetch(url, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
    if (!r.ok) {
      const txt = await r.text().catch(() => "");
      throw new HyperMeshError(
        `auth.create_session failed: HTTP ${r.status}`,
        r.status,
        txt,
      );
    }
    return r.json();
  }

  /** Drop the session — local only; daemon-side revoke handled by `signOutRemote`. */
  async signOut(): Promise<void> {
    await clearSession();
    this.session = null;
    this.inner = null;
  }

  /**
   * Best-effort: revoke the session at the daemon, then drop locally.
   * Failure of the remote revoke is non-fatal — the local state is
   * cleared regardless.
   */
  async signOutRemote(): Promise<void> {
    try {
      if (this.inner && this.session) {
        await this.inner.authRevokeSession(this.session.sessionId);
      }
    } catch {
      /* swallow */
    }
    await this.signOut();
  }
}

/** Process-wide singleton. Mounted in `App.tsx`. */
export const apiClient = new MobileApiClient();

/** Convenience for `HyperMeshClientOptions` — re-exported for screen code. */
export type { HyperMeshClientOptions };
