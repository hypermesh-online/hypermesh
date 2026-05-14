// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Settings as SettingsIcon, Shield, Lock, Key } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useConfigShow, useTrustchainIdentity, useTrustchainStatus } from '@/lib/hooks/useBlockMatrix';

function readString(obj: Record<string, unknown> | undefined, key: string): string | null {
  if (!obj) return null;
  const v = obj[key];
  return typeof v === 'string' ? v : null;
}

function readNumber(obj: Record<string, unknown> | undefined, key: string): number | null {
  if (!obj) return null;
  const v = obj[key];
  return typeof v === 'number' ? v : null;
}

function readBool(obj: Record<string, unknown> | undefined, key: string): boolean | null {
  if (!obj) return null;
  const v = obj[key];
  return typeof v === 'boolean' ? v : null;
}

function readObject(obj: Record<string, unknown> | undefined, key: string): Record<string, unknown> | undefined {
  if (!obj) return undefined;
  const v = obj[key];
  return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : undefined;
}

export function SettingsPanel() {
  const configQuery = useConfigShow();
  const identityQuery = useTrustchainIdentity();
  const statusQuery = useTrustchainStatus();

  if (configQuery.isLoading || identityQuery.isLoading || statusQuery.isLoading) {
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

  if (configQuery.error) {
    return (
      <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
        <CardContent className="py-8 text-center text-red-400">
          Failed to load configuration: {String((configQuery.error as Error).message ?? configQuery.error)}
        </CardContent>
      </Card>
    );
  }

  const config = configQuery.data;
  const nodeConfig = readObject(config, 'node');
  const networkConfig = readObject(config, 'network');
  const storageConfig = readObject(config, 'storage');
  const privacyConfig = readObject(config, 'privacy');

  const nodeId = readString(nodeConfig, 'id') ?? identityQuery.data?.node_id ?? 'unknown';
  const dataDir = readString(nodeConfig, 'data_dir') ?? '/var/lib/hypermesh';
  const privacyMode = readString(privacyConfig, 'mode') ?? identityQuery.data?.privacy_mode ?? 'Unknown';
  const bindAddress = readString(networkConfig, 'bind_address') ?? '::';
  const stoqPort = readNumber(networkConfig, 'stoq_port') ?? 9292;
  const maxPeers = readNumber(networkConfig, 'max_peers') ?? 0;
  const storageDir = readString(storageConfig, 'storage_dir') ?? dataDir;
  const reflectorEnabled = readBool(networkConfig, 'reflector_enabled');

  const signingAlgorithm = readString(statusQuery.data, 'signing_algorithm') ?? 'FALCON-1024';
  const encryptionAlgorithm = readString(statusQuery.data, 'encryption_algorithm') ?? 'Kyber-1024';
  const keyExchange = readString(statusQuery.data, 'key_exchange') ?? 'X25519MLKEM768';
  const caPhase = readString(statusQuery.data, 'ca_phase') ?? 'unknown';

  return (
    <div className="space-y-6">
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <SettingsIcon className="h-5 w-5 text-green-400" />
            Node Configuration
          </CardTitle>
          <CardDescription className="text-gray-400">
            Current node configuration loaded from disk (read-only in alpha)
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <ConfigRow label="Node ID" value={nodeId} mono />
          <ConfigRow label="Data Directory" value={dataDir} mono />
          <ConfigRow label="Storage Directory" value={storageDir} mono />
          <ConfigRow label="Privacy Mode" value={privacyMode} />

          <Separator className="bg-green-500/20" />

          <h4 className="text-white font-medium">Network</h4>
          <ConfigRow label="Bind Address" value={bindAddress} mono />
          <ConfigRow label="STOQ Port (UDP)" value={String(stoqPort)} mono />
          <ConfigRow
            label="Max Peers"
            value={maxPeers > 0 ? String(maxPeers) : 'unlimited'}
            mono
          />
          {reflectorEnabled !== null && (
            <ConfigRow
              label="Reflector Mode"
              value={reflectorEnabled ? 'enabled' : 'disabled'}
            />
          )}
        </CardContent>
      </Card>

      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-white flex items-center gap-2">
                <Shield className="h-5 w-5 text-purple-400" />
                Quantum Security
              </CardTitle>
              <CardDescription className="text-gray-400">
                Post-quantum cryptographic configuration (compile-time locked)
              </CardDescription>
            </div>
            <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30">
              {caPhase.replace(/_/g, ' ')}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <SecurityRow icon={Key} iconColor="text-green-400" label="Signing" value={signingAlgorithm} />
          <SecurityRow icon={Lock} iconColor="text-blue-400" label="Encryption" value={encryptionAlgorithm} />
          <SecurityRow icon={Shield} iconColor="text-purple-400" label="Key Exchange" value={keyExchange} />
        </CardContent>
      </Card>
    </div>
  );
}

function ConfigRow({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-sm text-gray-400 flex-shrink-0">{label}</span>
      <span className={cn('text-sm text-white truncate', mono && 'font-mono')}>{value}</span>
    </div>
  );
}

interface SecurityRowProps {
  icon: React.ComponentType<{ className?: string }>;
  iconColor: string;
  label: string;
  value: string;
}

function SecurityRow({ icon: Icon, iconColor, label, value }: SecurityRowProps) {
  return (
    <div className="flex items-center justify-between p-3 border border-green-500/20 rounded-lg bg-green-500/5">
      <div className="flex items-center gap-2">
        <Icon className={cn('h-4 w-4', iconColor)} />
        <span className="text-white text-sm">{label}</span>
      </div>
      <Badge variant="outline" className={cn('font-mono text-xs', iconColor, 'border-current/30')}>
        {value}
      </Badge>
    </div>
  );
}
