// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Security Monitoring Dashboard - Real-time TrustChain monitoring
 *
 * Displays certificate authority status, federation info, and identity
 * using real useBlockMatrix hooks.
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { moduleColors } from '@/lib/tokens';
import {
  useTrustchainStatus,
  useTrustchainCerts,
  useTrustchainIdentity,
  useTrustchainFederation,
} from '@/lib/hooks/useBlockMatrix';
import { Shield, AlertTriangle, Network, Key, Users, Lock } from 'lucide-react';
import { cn } from '@/lib/utils';

const colors = moduleColors.trustchain;

export function SecurityMonitoringDashboard() {
  const status = useTrustchainStatus();
  const certs = useTrustchainCerts();
  const identity = useTrustchainIdentity();
  const federation = useTrustchainFederation();

  if (status.isLoading && certs.isLoading) {
    return <ModuleLoading />;
  }

  if (status.error && certs.error) {
    return (
      <div className="p-6 space-y-6">
        <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent text-center`}>
          Security Monitoring Center
        </h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">TrustChain service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the TrustChain backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const totalCerts = status.data?.total_certs ?? 0;
  const activeCerts = status.data?.active_certs ?? 0;
  const revokedCerts = status.data?.revoked_certs ?? 0;
  const certHealth = totalCerts > 0 ? (activeCerts / totalCerts) * 100 : 0;

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="text-center py-4">
        <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent`}>
          Security Monitoring Center
        </h2>
        <p className="text-gray-400 text-sm mt-1">
          TrustChain CA status, certificates, federation, and post-quantum cryptography
        </p>
      </div>

      {/* Overview Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">CA Status</CardTitle>
            <Shield className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            {status.isLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <div className={cn(
                "text-2xl font-bold capitalize",
                status.data?.ca_status === 'active' ? 'text-green-400' :
                status.data?.ca_status === 'degraded' ? 'text-yellow-400' :
                'text-red-400',
              )}>
                {status.data?.ca_status ?? 'Unknown'}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Certificate Health</CardTitle>
            <Lock className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            {status.isLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <>
                <div className="text-2xl font-bold text-green-400">{certHealth.toFixed(1)}%</div>
                <p className="text-xs text-gray-400">{activeCerts} active / {totalCerts} total</p>
                <Progress value={certHealth} className="mt-2 h-1" />
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Revoked</CardTitle>
            <AlertTriangle className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {status.isLoading ? (
              <Skeleton className="h-8 w-16" />
            ) : (
              <div className={cn(
                "text-2xl font-bold",
                revokedCerts > 0 ? 'text-yellow-400' : 'text-green-400',
              )}>
                {revokedCerts}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Federation Peers</CardTitle>
            <Users className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            {federation.isLoading ? (
              <Skeleton className="h-8 w-16" />
            ) : (
              <div className="text-2xl font-bold text-green-400">
                {federation.data?.total_peers ?? 0}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Identity Info */}
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Key className="h-5 w-5 text-green-400" />
            Node Identity
          </CardTitle>
        </CardHeader>
        <CardContent>
          {identity.isLoading ? (
            <div className="space-y-2">
              <Skeleton className="h-6 w-full" />
              <Skeleton className="h-6 w-3/4" />
            </div>
          ) : identity.error ? (
            <p className="text-gray-500 text-center py-4">Identity data unavailable</p>
          ) : identity.data ? (
            <div className="grid gap-3 md:grid-cols-2">
              <IdentityRow label="Node ID" value={identity.data.node_id} mono />
              <IdentityRow label="Key Algorithm" value={identity.data.key_algorithm} />
              <IdentityRow
                label="Public Key"
                value={identity.data.public_key.length > 32
                  ? `${identity.data.public_key.slice(0, 16)}...${identity.data.public_key.slice(-16)}`
                  : identity.data.public_key}
                mono
              />
              <IdentityRow
                label="Created"
                value={new Date(identity.data.created_at).toLocaleDateString()}
              />
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No identity data</p>
          )}
        </CardContent>
      </Card>

      {/* Certificates */}
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Shield className="h-5 w-5 text-green-400" />
            Certificates
            <Badge className="ml-auto bg-green-500/20 text-green-400 border-green-500/30 text-xs">
              {certs.data?.total ?? 0} total
            </Badge>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {certs.isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-14 w-full" />
              ))}
            </div>
          ) : certs.error ? (
            <p className="text-gray-500 text-center py-4">Certificate data unavailable</p>
          ) : certs.data?.certificates && certs.data.certificates.length > 0 ? (
            <div className="space-y-2 max-h-80 overflow-y-auto">
              {certs.data.certificates.map((cert) => (
                <div
                  key={cert.id}
                  className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800"
                >
                  <div>
                    <p className="text-sm font-medium text-white">{cert.subject}</p>
                    <p className="text-xs text-gray-400">
                      Issued by {cert.issuer} | Valid until{' '}
                      {new Date(cert.valid_to).toLocaleDateString()}
                    </p>
                  </div>
                  <Badge className={cn(
                    "text-xs",
                    cert.status === 'active'
                      ? 'bg-green-500/20 text-green-400 border-green-500/30'
                      : cert.status === 'revoked'
                      ? 'bg-red-500/20 text-red-400 border-red-500/30'
                      : 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
                  )}>
                    {cert.status}
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No certificates</p>
          )}
        </CardContent>
      </Card>

      {/* Federation */}
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Network className="h-5 w-5 text-green-400" />
            Federation
          </CardTitle>
        </CardHeader>
        <CardContent>
          {federation.isLoading ? (
            <Skeleton className="h-24 w-full" />
          ) : federation.error ? (
            <p className="text-gray-500 text-center py-4">Federation data unavailable</p>
          ) : federation.data?.peers && federation.data.peers.length > 0 ? (
            <div className="space-y-2">
              {federation.data.peers.map((peer) => (
                <div
                  key={peer.node_id}
                  className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800"
                >
                  <span className="text-sm text-white font-mono truncate max-w-[200px]">
                    {peer.node_id.slice(0, 16)}...
                  </span>
                  <Badge className={cn(
                    "text-xs",
                    peer.trust_level === 'Full'
                      ? 'bg-green-500/20 text-green-400'
                      : peer.trust_level === 'Conditional'
                      ? 'bg-yellow-500/20 text-yellow-400'
                      : 'bg-red-500/20 text-red-400',
                  )}>
                    {peer.trust_level}
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No federation peers</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function IdentityRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="p-3 rounded-lg bg-black/20 border border-gray-800">
      <div className="text-xs text-gray-400 mb-1">{label}</div>
      <div className={cn("text-sm text-white break-all", mono && "font-mono")}>{value}</div>
    </div>
  );
}
