// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — React side of the Tauri bridge.
//
// The wizard pages and tray-aware UI elements call into Rust via
// @tauri-apps/api. To keep the UI buildable standalone (Gateway use
// case unchanged from C.1/C.2), we lazy-import the API and fall back
// to safe no-ops when the global `__TAURI_INTERNALS__` symbol is
// missing. This avoids hard-pinning a `@tauri-apps/api` dependency in
// `ui/frontend/package.json` while still letting the desktop bundle
// resolve it at runtime via Tauri's injection.

export interface DaemonStatus {
  state: 'stopped' | 'starting' | 'running' | 'error';
  pid: number | null;
  message: string | null;
  socket_path: string;
}

export interface DaemonStartArgs {
  privacy_mode?: string;
  network_id?: string | null;
  foreground?: boolean;
  extra_args?: string[];
}

export interface WizardState {
  should_show: boolean;
  identity_path: string;
  state_path: string;
  privacy_mode: string;
  join_trustnet_test: boolean;
  requested_foundation_grant: boolean;
  completed: boolean;
  completed_at_unix: number | null;
}

export function isTauri(): boolean {
  // Tauri 2 sets __TAURI_INTERNALS__; Tauri 1 set __TAURI__. We accept either.
  if (typeof window === 'undefined') return false;
  const w = window as unknown as Record<string, unknown>;
  return Boolean(w.__TAURI_INTERNALS__) || Boolean(w.__TAURI__);
}

/**
 * Invoke a Tauri command, or return `fallback` when running outside Tauri.
 * Errors from inside Tauri are re-thrown so the UI can surface them.
 */
export async function invokeOrFallback<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  fallback: T,
): Promise<T> {
  if (!isTauri()) return fallback;
  // Lazy import — module is only resolved when running inside the desktop bundle.
  const mod = (await import(/* @vite-ignore */ '@tauri-apps/api/core').catch(() => null)) as
    | { invoke: <R>(cmd: string, args?: Record<string, unknown>) => Promise<R> }
    | null;
  if (!mod) return fallback;
  return mod.invoke<T>(command, args);
}

/**
 * Subscribe to a Tauri event. Returns an unsubscribe function. In
 * non-Tauri environments returns a no-op unsubscribe.
 */
export async function listenOrNoop<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const mod = (await import(/* @vite-ignore */ '@tauri-apps/api/event').catch(() => null)) as
    | {
        listen: <P>(
          event: string,
          handler: (e: { payload: P }) => void,
        ) => Promise<() => void>;
      }
    | null;
  if (!mod) return () => {};
  return mod.listen<T>(event, (e) => handler(e.payload));
}

// -------- Convenience wrappers --------

export const wizard = {
  shouldShow: () => invokeOrFallback<boolean>('wizard_should_show', undefined, false),
  state: () =>
    invokeOrFallback<WizardState>('wizard_state', undefined, {
      should_show: false,
      identity_path: '',
      state_path: '',
      privacy_mode: 'private',
      join_trustnet_test: false,
      requested_foundation_grant: false,
      completed: true,
      completed_at_unix: null,
    }),
  setPrivacy: (mode: string) =>
    invokeOrFallback<void>('wizard_set_privacy', { mode }, undefined),
  setTrustnetTest: (optIn: boolean) =>
    invokeOrFallback<void>('wizard_set_trustnet_test', { optIn }, undefined),
  setFoundationGrant: (requested: boolean) =>
    invokeOrFallback<void>('wizard_set_foundation_grant', { requested }, undefined),
  complete: () => invokeOrFallback<WizardState | null>('wizard_complete', undefined, null),
};

export const daemon = {
  start: (args?: DaemonStartArgs) =>
    invokeOrFallback<DaemonStatus | null>('daemon_start', { args }, null),
  stop: () => invokeOrFallback<DaemonStatus | null>('daemon_stop', undefined, null),
  status: () => invokeOrFallback<DaemonStatus | null>('daemon_status', undefined, null),
  checkUpdate: () =>
    invokeOrFallback<Record<string, unknown> | null>('daemon_check_update', undefined, null),
};
