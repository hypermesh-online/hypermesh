// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Network, Users, Shield } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTrustchainFederation, useNetworkPeers } from '@/lib/hooks/useBlockMatrix';
import type { FederationPeer } from '@/lib/blockmatrix-api';
import { MetricCard } from '../shared/MetricCard';

function trustBadgeClass(level: FederationPeer['trust_level']): string {
  switch (level) {
    case 'full':
      return 'bg-green-500/20 text-green-400 border-green-500/30';
    case 'conditional':
      return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
    case 'untrusted':
      return 'bg-red-500/20 text-red-400 border-red-500/30';
    default:
      return 'bg-gray-500/20 text-gray-300 border-gray-500/30';
  }
}

export function FederationPanel() {
  const federationQuery = useTrustchainFederation();
  const peersQuery = useNetworkPeers();

  if (federationQuery.isLoading) {
    return (
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardContent className="py-8">
          <div className="space-y-3 animate-pulse">
            <div className="h-8 bg-gray-700 rounded w-1/3" />
            <div className="h-32 bg-gray-700 rounded" />
          </div>
        </CardContent>
      </Card>
    );
  }

  if (federationQuery.error) {
    return (
      <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
        <CardContent className="py-8 text-center text-red-400">
          Failed to load federation: {String((federationQuery.error as Error).message ?? federationQuery.error)}
        </CardContent>
      </Card>
    );
  }

  const federation = federationQuery.data;
  const peers: FederationPeer[] = federation?.peers ?? [];
  const totalPeers = federation?.total_peers ?? peers.length;
  const networkPeers = federation?.network_peers ?? peersQuery.data?.length ?? 0;
  const trustLevels = federation?.trust_levels ?? { full: 0, conditional: 0, untrusted: 0 };
  const note = federation?.note ?? '';
  const totalForBar = Math.max(1, trustLevels.full + trustLevels.conditional + trustLevels.untrusted);
  const fullPct = (trustLevels.full / totalForBar) * 100;
  const condPct = (trustLevels.conditional / totalForBar) * 100;
  const untrustedPct = (trustLevels.untrusted / totalForBar) * 100;

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-3">
        <MetricCard
          title="Federation Peers"
          value={totalPeers}
          description="TrustChain federated CAs"
          icon={Shield}
          color="text-green-400"
          className="border-green-500/30"
        />
        <MetricCard
          title="Network Peers"
          value={networkPeers}
          description="Bilateral PoS-verified peers"
          icon={Network}
          color="text-blue-400"
          className="border-blue-500/30"
        />
        <MetricCard
          title="Full Trust"
          value={trustLevels.full}
          description={`${trustLevels.conditional} conditional · ${trustLevels.untrusted} untrusted`}
          icon={Users}
          color="text-purple-400"
          className="border-purple-500/30"
        />
      </div>

      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Shield className="h-5 w-5 text-green-400" />
            Federated Peer CAs
          </CardTitle>
          <CardDescription className="text-gray-400">
            Bilateral trust relationships with other TrustChain CAs
          </CardDescription>
          {note && (
            <p className="text-xs italic text-gray-500 mt-2">{note}</p>
          )}
        </CardHeader>
        <CardContent className="space-y-5">
          <TrustLevelBar
            fullPct={fullPct}
            condPct={condPct}
            untrustedPct={untrustedPct}
            counts={trustLevels}
            hasAny={trustLevels.full + trustLevels.conditional + trustLevels.untrusted > 0}
          />

          {peers.length === 0 ? (
            <div className="text-center py-8">
              <Shield className="h-10 w-10 text-gray-600 mx-auto mb-3" />
              <p className="text-gray-300 font-medium mb-1">No federated peers</p>
              <p className="text-sm text-gray-400">
                Join a federation via M.2.5 once Distributed CA (Phase F) is active.
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {peers.map((peer) => (
                <div
                  key={peer.node_id}
                  className="border border-green-500/30 rounded-lg p-3 bg-green-500/5 flex items-center justify-between gap-3"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <Shield className="h-4 w-4 text-green-400 flex-shrink-0" />
                    <div className="min-w-0">
                      <div className="text-white font-mono text-sm truncate">{peer.node_id}</div>
                      {peer.fingerprint && (
                        <div className="text-gray-500 font-mono text-xs truncate">
                          fp:{peer.fingerprint.slice(0, 24)}…
                        </div>
                      )}
                    </div>
                  </div>
                  <Badge className={cn('text-xs', trustBadgeClass(peer.trust_level))}>
                    {peer.trust_level}
                  </Badge>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function TrustLevelBar({
  fullPct,
  condPct,
  untrustedPct,
  counts,
  hasAny,
}: {
  fullPct: number;
  condPct: number;
  untrustedPct: number;
  counts: { full: number; conditional: number; untrusted: number };
  hasAny: boolean;
}) {
  return (
    <div>
      <div className="flex items-center justify-between text-xs text-gray-400 mb-1.5">
        <span>Trust level distribution</span>
        {!hasAny && <span className="italic">no peers yet</span>}
      </div>
      <div className="flex h-3 w-full overflow-hidden rounded-full bg-gray-800">
        <div
          className="bg-green-500/60"
          style={{ width: `${fullPct}%` }}
          aria-label={`full ${counts.full}`}
        />
        <div
          className="bg-yellow-500/60"
          style={{ width: `${condPct}%` }}
          aria-label={`conditional ${counts.conditional}`}
        />
        <div
          className="bg-red-500/60"
          style={{ width: `${untrustedPct}%` }}
          aria-label={`untrusted ${counts.untrusted}`}
        />
      </div>
      <div className="flex items-center gap-4 mt-2 text-xs">
        <LegendDot color="bg-green-500/70" label={`full ${counts.full}`} />
        <LegendDot color="bg-yellow-500/70" label={`conditional ${counts.conditional}`} />
        <LegendDot color="bg-red-500/70" label={`untrusted ${counts.untrusted}`} />
      </div>
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <div className="flex items-center gap-1.5 text-gray-300">
      <span className={cn('inline-block h-2 w-2 rounded-full', color)} />
      <span className="font-mono">{label}</span>
    </div>
  );
}
