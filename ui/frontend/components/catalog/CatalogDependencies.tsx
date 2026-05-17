// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Catalog Dependencies — Phase M.4.
 *
 * Wires `catalog.dependencies` IPC handler (via Gateway proxy) to a
 * dependency-graph view. The user enters a type name (or full type
 * hash) and the panel renders the BFS graph: direct deps, transitive
 * deps, missing dep hashes.
 *
 * Renders an honest alpha-inert state when the daemon's catalog
 * registry is not yet wired (M.4.5d backlog).
 */

import React from 'react';
import { useSearchParams } from 'react-router-dom';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Package,
  GitBranch,
  AlertTriangle,
  CheckCircle,
  Search,
  Loader2,
} from 'lucide-react';
import { useCatalogDependencies } from '@/lib/hooks/useBlockMatrix';
import type { DependencyNode } from '@/lib/blockmatrix-api';

function truncateHash(hash: string, len = 16): string {
  if (hash.length <= len) return hash;
  return `${hash.slice(0, len)}...`;
}

function isHexHash(value: string): boolean {
  return /^[0-9a-fA-F]{64}$/.test(value);
}

interface DepRowProps {
  dep: DependencyNode;
}

function DepRow({ dep }: DepRowProps) {
  return (
    <div className="flex items-center justify-between px-4 py-3 border-b border-purple-500/10 last:border-b-0">
      <div className="flex items-center gap-3 min-w-0">
        <Package className="h-4 w-4 text-purple-400 shrink-0" />
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-white truncate">{dep.name}</span>
            <Badge variant="secondary" className="text-xs">
              v{dep.version}
            </Badge>
            <Badge
              variant="outline"
              className="text-xs border-purple-500/30 text-purple-300"
            >
              depth {dep.depth}
            </Badge>
          </div>
          <code className="text-xs text-gray-500 font-mono">
            {truncateHash(dep.type_hash, 24)}
          </code>
        </div>
      </div>
    </div>
  );
}

export function CatalogDependencies() {
  const [searchParams, setSearchParams] = useSearchParams();
  const initialHash = searchParams.get('type_hash') ?? undefined;
  const initialName = searchParams.get('type_name') ?? undefined;

  const [inputValue, setInputValue] = React.useState(
    initialHash ?? initialName ?? '',
  );
  const [activeQuery, setActiveQuery] = React.useState<{
    type_hash?: string;
    type_name?: string;
  } | undefined>(
    initialHash
      ? { type_hash: initialHash }
      : initialName
      ? { type_name: initialName }
      : undefined,
  );

  // React to URL changes (e.g. user clicks a global-search result while
  // already on this page).
  React.useEffect(() => {
    const hash = searchParams.get('type_hash');
    const name = searchParams.get('type_name');
    if (hash) {
      setActiveQuery({ type_hash: hash });
      setInputValue(hash);
    } else if (name) {
      setActiveQuery({ type_name: name });
      setInputValue(name);
    }
  }, [searchParams]);

  const { data, isLoading, isFetching, error } = useCatalogDependencies(activeQuery);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = inputValue.trim();
    if (!trimmed) return;
    if (isHexHash(trimmed)) {
      const hash = trimmed.toLowerCase();
      setActiveQuery({ type_hash: hash });
      setSearchParams({ type_hash: hash });
    } else {
      setActiveQuery({ type_name: trimmed });
      setSearchParams({ type_name: trimmed });
    }
  };

  const alphaInert = data?.status === 'alpha';
  const typeNotFound =
    data?.status === 'ok' &&
    data.total === 0 &&
    data.direct_deps.length === 0 &&
    typeof data.note === 'string' &&
    data.note.startsWith('type not found');

  const directCount = data?.direct_deps.length ?? 0;
  const transitiveCount = data?.transitive_deps.length ?? 0;
  const missingCount = data?.missing.length ?? 0;

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-orange-400 to-red-600 bg-clip-text text-transparent mb-2">
          Dependency Graph
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Resolve the full transitive dependency graph for a typedef
          registered in the catalog. Enter a type name or a 64-hex
          content hash.
        </p>
      </div>

      {/* Query input */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardContent className="p-4">
          <form onSubmit={handleSubmit} className="flex items-center gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
              <input
                type="text"
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                placeholder="Type name (e.g. MyTypedef) or 64-hex type_hash"
                className="w-full pl-10 pr-4 py-2 bg-black/40 border border-purple-500/30 rounded text-white placeholder:text-gray-400"
              />
            </div>
            <Button
              type="submit"
              disabled={!inputValue.trim()}
              className="bg-purple-600 hover:bg-purple-700"
            >
              Resolve
            </Button>
          </form>
        </CardContent>
      </Card>

      {/* Empty / loading / error / alpha-inert / data states */}
      {!activeQuery && (
        <div className="text-center py-12">
          <GitBranch className="h-16 w-16 text-gray-600 mx-auto mb-4" />
          <h3 className="text-xl font-semibold text-gray-300 mb-2">
            No typedef selected
          </h3>
          <p className="text-gray-400">
            Enter a type name or content hash above to resolve its dependency
            graph.
          </p>
        </div>
      )}

      {activeQuery && (isLoading || isFetching) && !data && (
        <div className="flex items-center justify-center py-12 text-gray-400">
          <Loader2 className="h-5 w-5 animate-spin mr-2" />
          Resolving dependency graph...
        </div>
      )}

      {activeQuery && error && (
        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardContent className="p-6 text-red-300">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-5 w-5 shrink-0 mt-0.5" />
              <div>
                <h3 className="font-semibold">Failed to resolve dependencies</h3>
                <p className="text-sm text-red-200/80 mt-1">
                  {(error as Error).message}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {data && alphaInert && (
        <Card className="bg-black/40 border-amber-500/30 backdrop-blur-lg">
          <CardContent className="p-6">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-5 w-5 text-amber-400 shrink-0 mt-0.5" />
              <div>
                <h3 className="text-amber-200 font-semibold">
                  Catalog registry not yet wired
                </h3>
                <p className="text-sm text-amber-100/70 mt-1">
                  The daemon's catalog registry is alpha-default inert. Set
                  <code className="mx-1 px-1.5 py-0.5 bg-black/40 rounded text-amber-300">
                    HYPERMESH_CATALOG_ENABLE=1
                  </code>
                  to populate dependency graphs (M.4.5d backlog).
                </p>
                {data.note && (
                  <p className="text-xs text-amber-100/50 mt-2">
                    Daemon note: {data.note}
                  </p>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {data && !alphaInert && typeNotFound && (
        <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
          <CardContent className="p-6 text-center text-gray-400">
            <Package className="h-10 w-10 text-gray-600 mx-auto mb-3" />
            <h3 className="text-gray-200 font-semibold mb-1">
              Type not found
            </h3>
            <p className="text-sm">
              {data.note ?? 'No typedef matches that name in the catalog registry.'}
            </p>
          </CardContent>
        </Card>
      )}

      {data && !alphaInert && !typeNotFound && (
        <>
          {/* Summary cards */}
          <div className="grid gap-4 md:grid-cols-3">
            <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <CheckCircle className="h-8 w-8 text-green-400" />
                  <div>
                    <h3 className="text-xl font-bold text-white">{directCount}</h3>
                    <p className="text-sm text-gray-400">Direct dependencies</p>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <GitBranch className="h-8 w-8 text-purple-400" />
                  <div>
                    <h3 className="text-xl font-bold text-white">{transitiveCount}</h3>
                    <p className="text-sm text-gray-400">Transitive dependencies</p>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <AlertTriangle className="h-8 w-8 text-red-400" />
                  <div>
                    <h3 className="text-xl font-bold text-white">{missingCount}</h3>
                    <p className="text-sm text-gray-400">Missing</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Root type identity */}
          {data.type_hash && (
            <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
              <CardContent className="p-4 flex items-center justify-between">
                <div>
                  <p className="text-xs text-gray-400 uppercase tracking-wider">
                    Resolved type hash
                  </p>
                  <code className="text-sm text-purple-200 font-mono break-all">
                    {data.type_hash}
                  </code>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Direct deps table */}
          {directCount > 0 && (
            <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
              <CardContent className="p-0">
                <div className="px-4 py-3 border-b border-purple-500/20">
                  <h3 className="text-white font-medium">Direct dependencies</h3>
                </div>
                {data!.direct_deps.map((dep) => (
                  <DepRow key={dep.type_hash} dep={dep} />
                ))}
              </CardContent>
            </Card>
          )}

          {/* Transitive deps table */}
          {transitiveCount > 0 && (
            <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
              <CardContent className="p-0">
                <div className="px-4 py-3 border-b border-purple-500/20">
                  <h3 className="text-white font-medium">Transitive dependencies</h3>
                </div>
                {data!.transitive_deps.map((dep) => (
                  <DepRow key={dep.type_hash} dep={dep} />
                ))}
              </CardContent>
            </Card>
          )}

          {/* Missing deps */}
          {missingCount > 0 && (
            <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
              <CardContent className="p-4">
                <div className="flex items-center gap-2 mb-3">
                  <AlertTriangle className="h-4 w-4 text-red-400" />
                  <h3 className="text-white font-medium">Missing dependencies</h3>
                </div>
                <div className="flex flex-wrap gap-2">
                  {data!.missing.map((hash) => (
                    <Badge
                      key={hash}
                      variant="outline"
                      className="font-mono text-xs border-red-500/40 text-red-300 bg-red-500/10"
                      title={hash}
                    >
                      {truncateHash(hash)}
                    </Badge>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {directCount === 0 && transitiveCount === 0 && missingCount === 0 && (
            <div className="text-center py-12">
              <CheckCircle className="h-12 w-12 text-green-500 mx-auto mb-3" />
              <h3 className="text-lg font-semibold text-gray-200 mb-1">
                No dependencies
              </h3>
              <p className="text-sm text-gray-400">
                This typedef has no declared dependencies.
              </p>
            </div>
          )}
        </>
      )}
    </div>
  );
}
