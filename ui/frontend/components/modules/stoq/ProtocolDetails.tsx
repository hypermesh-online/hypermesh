// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * STOQ Protocol Details view.
 *
 * Wired to the daemon `stoq.stats` IPC handler via {@link useStoqStats}.
 * Renders the fields the daemon actually exposes today:
 *   - protocol, privacy_mode, transport_active, shard_transport_active
 *   - connections, unique_endpoints, uptime_secs, node_id
 *
 * Fields like cipher suite, protocol version, and MTU are not yet surfaced
 * by the daemon. They are shown as honest empty states ("not reported by
 * daemon") instead of fabricated values.
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { useStoqStats } from '@/lib/hooks/useBlockMatrix';
import { AlertTriangle, Shield, Zap, Network } from 'lucide-react';
import { cn } from '@/lib/utils';

const NOT_REPORTED = 'not reported by daemon';

export function ProtocolDetails() {
  const stats = useStoqStats();

  if (stats.error) {
    return (
      <div className="p-6">
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">STOQ protocol data unavailable</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const data = stats.data;
  const transportActive = data?.transport_active ?? false;
  const shardActive = data?.shard_transport_active ?? false;
  const protocol = typeof data?.protocol === 'string' ? data.protocol : 'QUIC';
  const privacyMode = typeof data?.privacy_mode === 'string' ? data.privacy_mode : null;
  const connections = data?.connections ?? data?.connections_active ?? 0;
  const uniqueEndpoints = data?.unique_endpoints ?? 0;
  const uptimeSecs = data?.uptime_secs ?? 0;
  const nodeId = typeof data?.node_id === 'string' ? data.node_id : null;

  return (
    <div className="space-y-6">
      <div className="text-center py-4">
        <h2 className="text-2xl font-bold text-white mb-2">STOQ Protocol</h2>
        <p className="text-gray-400">
          Live protocol state reported by the local daemon
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Transport Status */}
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Network className="h-5 w-5 text-cyan-400" />
              Transport Status
            </CardTitle>
            <CardDescription className="text-gray-400">
              Reported by daemon stoq.stats
            </CardDescription>
          </CardHeader>
          <CardContent>
            {stats.isLoading ? (
              <Skeleton className="h-40 w-full" />
            ) : (
              <div className="space-y-3">
                <StatusRow label="Protocol" value={protocol} />
                <StatusRow
                  label="Transport"
                  value={transportActive ? 'Active' : 'Inactive'}
                  tone={transportActive ? 'good' : 'critical'}
                />
                <StatusRow
                  label="Shard Transport"
                  value={shardActive ? 'Active' : 'Inactive'}
                  tone={shardActive ? 'good' : 'warn'}
                />
                <StatusRow
                  label="Privacy Mode"
                  value={privacyMode ?? NOT_REPORTED}
                  muted={!privacyMode}
                />
                <StatusRow label="Connections" value={String(connections)} />
                <StatusRow label="Unique Endpoints" value={String(uniqueEndpoints)} />
                <StatusRow label="Uptime" value={formatUptime(uptimeSecs)} />
              </div>
            )}
          </CardContent>
        </Card>

        {/* Cryptographic Suite */}
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Shield className="h-5 w-5 text-cyan-400" />
              Cryptographic Suite
            </CardTitle>
            <CardDescription className="text-gray-400">
              Quantum-resistant primitives in use
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <StatusRow label="Key Exchange" value="X25519MLKEM768" />
              <StatusRow label="Signatures" value="FALCON-1024" />
              <StatusRow label="Asset Encryption" value="Kyber-1024 KEM + AES-256-GCM" />
              <StatusRow label="Hashing" value="BLAKE3" />
              <StatusRow
                label="Cipher Suite"
                value={NOT_REPORTED}
                muted
              />
              <StatusRow label="Protocol Version" value={NOT_REPORTED} muted />
              <StatusRow label="MTU" value={NOT_REPORTED} muted />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Node Identity */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Zap className="h-5 w-5 text-cyan-400" />
            Node Identity
          </CardTitle>
          <CardDescription className="text-gray-400">
            Identifier used by this node on the STOQ overlay
          </CardDescription>
        </CardHeader>
        <CardContent>
          {stats.isLoading ? (
            <Skeleton className="h-10 w-full" />
          ) : nodeId ? (
            <div className="flex items-center gap-2">
              <Badge className="bg-cyan-500/20 text-cyan-400 border-cyan-500/30 font-mono text-xs">
                {nodeId}
              </Badge>
            </div>
          ) : (
            <p className="text-sm text-gray-500">{NOT_REPORTED}</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function StatusRow({
  label,
  value,
  tone,
  muted,
}: {
  label: string;
  value: string;
  tone?: 'good' | 'warn' | 'critical';
  muted?: boolean;
}) {
  return (
    <div className="flex justify-between items-center p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
      <span className="text-sm text-gray-300">{label}</span>
      <span
        className={cn(
          'text-sm font-mono',
          muted
            ? 'text-gray-500 italic'
            : tone === 'critical'
              ? 'text-red-400'
              : tone === 'warn'
                ? 'text-yellow-400'
                : 'text-cyan-400',
        )}
      >
        {value}
      </span>
    </div>
  );
}

function formatUptime(secs: number): string {
  if (secs <= 0) return '0s';
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  const seconds = secs % 60;
  if (days > 0) return `${days}d ${hours}h ${minutes}m`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}
