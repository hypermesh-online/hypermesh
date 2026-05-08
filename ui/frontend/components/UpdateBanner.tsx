// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Phase J.1 — UpdateBanner.
 *
 * Persistent banner that surfaces foundation-published release feed
 * entries. Polls `/api/v1/system/check_update` (proxied through the
 * Gateway to the daemon's `system.check_update` IPC handler) and
 * shows an "Update available" call-to-action when an entry is
 * available for the current channel.
 *
 * Alpha note: The Gateway HTTP route is added as part of the J.1
 * follow-up sub-step. Until that lands, this component renders as a
 * no-op when the endpoint returns 404 — it does not block the rest of
 * the dashboard from loading.
 */

import React, { useEffect, useState } from 'react';
import { AlertCircle, ExternalLink, X } from 'lucide-react';

interface UpdateInfo {
  up_to_date: boolean;
  available_version?: string;
  current_version?: string;
  release_notes_url?: string;
  breaking_changes?: boolean;
  channel?: string;
}

const POLL_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

async function fetchUpdate(): Promise<UpdateInfo | null> {
  try {
    const res = await fetch('/api/v1/system/check_update');
    if (!res.ok) return null;
    return (await res.json()) as UpdateInfo;
  } catch {
    return null;
  }
}

export function UpdateBanner() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      const result = await fetchUpdate();
      if (!cancelled) setInfo(result);
    };
    poll();
    const id = setInterval(poll, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (dismissed) return null;
  if (!info || info.up_to_date) return null;
  if (!info.available_version) return null;

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
