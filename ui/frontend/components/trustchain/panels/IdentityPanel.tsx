// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Shield, Key, Lock } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTrustchainIdentity } from '@/lib/hooks/useBlockMatrix';
import type { KeyInfo } from '@/lib/blockmatrix-api';

export function IdentityPanel() {
  const identityQuery = useTrustchainIdentity();

  if (identityQuery.isLoading) {
    return (
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardContent className="py-8">
          <div className="space-y-3 animate-pulse">
            <div className="h-8 bg-gray-700 rounded w-1/3" />
            <div className="h-24 bg-gray-700 rounded" />
            <div className="h-24 bg-gray-700 rounded" />
          </div>
        </CardContent>
      </Card>
    );
  }

  if (identityQuery.error) {
    return (
      <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
        <CardContent className="py-8 text-center text-red-400">
          Failed to load identity: {String((identityQuery.error as Error).message ?? identityQuery.error)}
        </CardContent>
      </Card>
    );
  }

  const identity = identityQuery.data;
  if (!identity) {
    return (
      <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
        <CardContent className="py-8 text-center text-yellow-400">
          No identity data returned by daemon.
        </CardContent>
      </Card>
    );
  }

  const createdLabel = identity.created_at != null
    ? new Date(identity.created_at * 1000).toLocaleString()
    : '—';

  return (
    <div className="space-y-6">
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-white flex items-center gap-2">
                <Shield className="h-5 w-5 text-purple-400" />
                Node Identity
              </CardTitle>
              <CardDescription className="text-gray-400">
                Post-quantum cryptographic keys backing this node
              </CardDescription>
            </div>
            <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30">
              {identity.status}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="text-sm text-gray-400">Node ID</label>
              <div className="text-white font-mono text-sm break-all bg-gray-800/50 p-3 rounded-lg mt-1">
                {identity.node_id || 'unknown'}
              </div>
            </div>
            <div className="grid gap-3">
              <div>
                <label className="text-sm text-gray-400">Privacy Mode</label>
                <div className="mt-1">
                  <Badge variant="outline" className="text-purple-300 border-purple-500/30 bg-purple-500/10">
                    {identity.privacy_mode}
                  </Badge>
                </div>
              </div>
              <div>
                <label className="text-sm text-gray-400">Created</label>
                <div className="text-white font-mono text-sm">{createdLabel}</div>
              </div>
            </div>
          </div>

          <Separator className="bg-green-500/20" />

          <div className="grid gap-6 md:grid-cols-2">
            <KeyCard
              title="FALCON Signing Key"
              description="Post-quantum digital signature key (NIST PQC standard)"
              icon={Key}
              iconColor="text-green-400"
              borderColor="border-green-500/30"
              info={identity.falcon}
            />
            <KeyCard
              title="Kyber KEM Key"
              description="Post-quantum key encapsulation for asset encryption"
              icon={Lock}
              iconColor="text-blue-400"
              borderColor="border-blue-500/30"
              info={identity.kyber}
            />
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

interface KeyCardProps {
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  iconColor: string;
  borderColor: string;
  info: KeyInfo;
}

function KeyCard({ title, description, icon: Icon, iconColor, borderColor, info }: KeyCardProps) {
  return (
    <Card className={cn('bg-black/40 backdrop-blur-lg', borderColor)}>
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2 text-base">
          <Icon className={cn('h-5 w-5', iconColor)} />
          {title}
        </CardTitle>
        <CardDescription className="text-gray-400">{description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex items-center justify-between">
          <span className="text-sm text-gray-400">Status</span>
          {info.present ? (
            <Badge className="bg-green-500/20 text-green-400 border-green-500/30">Present</Badge>
          ) : (
            <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">Not Generated</Badge>
          )}
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-gray-400">Algorithm</span>
          <span className={cn('font-mono text-sm', iconColor)}>{info.key_algorithm}</span>
        </div>

        {info.present && (
          <>
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-400">Public Key Size</span>
              <span className="text-white font-mono text-sm">{info.bytes.toLocaleString()} bytes</span>
            </div>
            {info.fingerprint && (
              <div>
                <label className="text-sm text-gray-400">Fingerprint (BLAKE3)</label>
                <div className="text-white font-mono text-xs break-all bg-gray-800/50 p-3 rounded-lg mt-1">
                  {info.fingerprint}
                </div>
              </div>
            )}
          </>
        )}

        {!info.present && (
          <p className="text-sm text-gray-400">
            No public key found on disk. Run the node bootstrap to generate identity keys.
          </p>
        )}
      </CardContent>
    </Card>
  );
}
