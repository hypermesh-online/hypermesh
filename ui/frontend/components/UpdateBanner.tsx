// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Phase J.1 — UpdateBanner (M.4 wiring).
 *
 * Persistent banner that surfaces foundation-published release feed
 * entries. Consumes the typed `useSystemCheckUpdate` hook (polled every
 * 5 minutes) which calls the daemon's `system.check_update` IPC handler
 * via the Gateway proxy.
 *
 * Renders nothing when:
 *   - the daemon's release feed is not configured (alpha-default inert)
 *   - the node is up-to-date
 *   - the query errors (best-effort — banner is non-blocking)
 *   - the user has dismissed it
 *
 * In the Tauri desktop bundle, the tray emits an `update-available`
 * event when it polls the IPC directly; we listen for that and force a
 * refetch via the React Query client so the banner can appear without
 * waiting for the next 5-minute HTTP poll.
 */

import React, { useEffect, useState } from 'react';
import { AlertCircle, ExternalLink, X } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { listenOrNoop } from './wizard/tauriBridge';
import { useSystemCheckUpdate } from '@/lib/hooks/useBlockMatrix';

export function UpdateBanner() {
  const queryClient = useQueryClient();
  const { data: info, isError } = useSystemCheckUpdate();
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    listenOrNoop<string | null>('update-available', () => {
      queryClient.invalidateQueries({
        queryKey: ['blockmatrix', 'system', 'check_update'],
      });
    }).then((un) => {
      if (cancelled) un();
      else unlisten = un;
    });
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [queryClient]);

  if (dismissed) return null;
  if (isError || !info) return null;
  if (info.up_to_date) return null;
  if (!info.available_version) return null;
  // Defensive: if the daemon reports a release-feed-not-configured note,
  // stay silent even if some other field looks update-ish.
  if (typeof info.note === 'string' && info.note.toLowerCase().includes('not configured')) {
    return null;
  }

  const breaking = info.breaking_changes === true;
  const bannerColor = breaking
    ? 'bg-amber-500/10 border-amber-500/40 text-amber-200'
    : 'bg-cyan-500/10 border-cyan-500/40 text-cyan-200';

  return (
    <div
      className={`flex items-center justify-between gap-4 px-4 py-2 border-b ${bannerColor}`}
      role="status"
      aria-live="polite"
    >
      <div className="flex items-center gap-3">
        <AlertCircle className="h-4 w-4 shrink-0" />
        <span className="text-sm">
          Update available: <strong>v{info.available_version}</strong>
          {info.current_version ? (
            <span className="opacity-70"> (current: v{info.current_version})</span>
          ) : null}
          {breaking ? (
            <span className="ml-2 px-2 py-0.5 rounded bg-amber-500/30 text-amber-100 text-xs">
              breaking changes
            </span>
          ) : null}
        </span>
      </div>
      <div className="flex items-center gap-2">
        {info.release_notes_url ? (
          <a
            href={info.release_notes_url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-xs underline"
          >
            View release notes
            <ExternalLink className="h-3 w-3" />
          </a>
        ) : null}
        <button
          onClick={() => setDismissed(true)}
          aria-label="Dismiss update banner"
          className="p-1 rounded hover:bg-white/10"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
