// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Global catalog search — Phase M.4.
 *
 * Debounced query against the daemon's `catalog.search` IPC handler.
 * Renders typedef matches with a `source` badge distinguishing local
 * hits from remote neighbor hits (recursive fan-out).
 *
 * Honest empty states:
 *   - daemon registry unwired (status === 'alpha')  → "registry not wired"
 *   - registry wired, zero matches                  → "No assets matching '<q>'"
 *   - some neighbors errored                        → subtle footer note
 */

import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Search, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useCatalogSearch } from '@/lib/hooks/useBlockMatrix';
import type { CatalogSearchMatch } from '@/lib/blockmatrix-api';

interface GlobalSearchProps {
  onResultSelect?: (result: CatalogSearchMatch) => void;
  className?: string;
}

function truncateNodeId(nodeId: string, len = 8): string {
  if (nodeId.length <= len) return nodeId;
  return `${nodeId.slice(0, len)}...`;
}

function SourceBadge({ source }: { source: string }) {
  if (source === 'local') {
    return (
      <Badge className="text-xs bg-green-500/20 text-green-300 border-green-500/40 border">
        local
      </Badge>
    );
  }
  if (source.startsWith('neighbor:')) {
    const nodeId = source.slice('neighbor:'.length);
    return (
      <Badge
        className="text-xs bg-blue-500/20 text-blue-300 border-blue-500/40 border font-mono"
        title={`neighbor:${nodeId}`}
      >
        neighbor:{truncateNodeId(nodeId)}
      </Badge>
    );
  }
  return (
    <Badge className="text-xs bg-gray-500/20 text-gray-300 border-gray-500/40 border">
      {source}
    </Badge>
  );
}

export function GlobalSearch({ onResultSelect, className }: GlobalSearchProps) {
  const [rawInput, setRawInput] = React.useState('');
  const [debounced, setDebounced] = React.useState('');
  const [open, setOpen] = React.useState(false);

  React.useEffect(() => {
    const id = setTimeout(() => setDebounced(rawInput.trim()), 200);
    return () => clearTimeout(id);
  }, [rawInput]);

  const { data, isFetching, error } = useCatalogSearch(debounced, true, 8);

  const showPanel =
    open &&
    debounced.length >= 2 &&
    (isFetching || !!data || !!error);

  const alphaInert = data?.status === 'alpha';
  const matches = data?.matches ?? [];
  const neighborErrors = data?.neighbor_errors ?? [];
  const neighborsQueried = data?.neighbors_queried ?? 0;

  const handleSelect = (match: CatalogSearchMatch) => {
    onResultSelect?.(match);
    setOpen(false);
  };

  return (
    <div
      className={cn('relative w-full max-w-2xl', className)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) {
          setOpen(false);
        }
      }}
    >
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
        <Input
          type="text"
          value={rawInput}
          onChange={(e) => {
            setRawInput(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          placeholder="Search catalog typedefs (name or partial hash)..."
          className="pl-10 pr-4 h-10 bg-black/40 border-gray-800 text-white placeholder-gray-500"
          aria-label="Catalog search"
        />
        {isFetching && (
          <Loader2 className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 animate-spin text-gray-400" />
        )}
      </div>

      {showPanel && (
        <Card
          className="absolute top-12 left-0 right-0 z-50 bg-black/95 border-gray-800 backdrop-blur-xl shadow-2xl"
          tabIndex={-1}
        >
          <CardContent className="p-0 max-h-96 overflow-y-auto">
            {error && (
              <div className="px-4 py-3 text-sm text-red-300">
                Search failed: {(error as Error).message}
              </div>
            )}

            {!error && alphaInert && (
              <div className="px-4 py-3 text-sm text-amber-200/80">
                Catalog search registry not yet wired.
                {data?.note ? (
                  <span className="text-amber-200/50"> ({data.note})</span>
                ) : null}
              </div>
            )}

            {!error && !alphaInert && data && matches.length === 0 && (
              <div className="px-4 py-3 text-sm text-gray-400">
                No assets matching '{debounced}'
              </div>
            )}

            {!error && !alphaInert && matches.length > 0 && (
              <ul className="divide-y divide-gray-800">
                {matches.map((m) => (
                  <li key={`${m.source}-${m.type_hash}`}>
                    <button
                      type="button"
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={() => handleSelect(m)}
                      className="w-full text-left px-4 py-3 hover:bg-cyan-500/10 focus:bg-cyan-500/10 focus:outline-none"
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-white truncate">
                          {m.name}
                        </span>
                        <Badge
                          variant="secondary"
                          className="text-xs"
                        >
                          v{m.version}
                        </Badge>
                        <SourceBadge source={m.source} />
                      </div>
                      <code className="block text-xs text-gray-500 font-mono mt-0.5 truncate">
                        {m.type_hash}
                      </code>
                    </button>
                  </li>
                ))}
              </ul>
            )}

            {!error && !alphaInert && data && (neighborsQueried > 0 || neighborErrors.length > 0) && (
              <div className="px-4 py-2 border-t border-gray-800 text-xs text-gray-500 flex items-center justify-between">
                <span>
                  Queried {neighborsQueried} neighbor
                  {neighborsQueried === 1 ? '' : 's'}
                </span>
                {neighborErrors.length > 0 && (
                  <span
                    className="text-amber-300/70"
                    title={neighborErrors
                      .map((e) => `${e.node_id}: ${e.error}`)
                      .join('\n')}
                  >
                    {neighborErrors.length} peer
                    {neighborErrors.length === 1 ? '' : 's'} timed out
                  </span>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
