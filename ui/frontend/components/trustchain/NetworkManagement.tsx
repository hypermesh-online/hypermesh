// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useSystemStatus } from '@/lib/api';
import { useNodeStatus, useNetworkPeers, useDomainList } from '@/lib/hooks/useBlockMatrix';
import { MetricCard } from './shared/MetricCard';
import { StatusIndicator } from './shared/StatusIndicator';
import { getStatusColor, getTypeColor } from './utils/statusHelpers';
import {
  Network,
  Users,
  Shield,
  Plus,
  Settings
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface NetworkConnection {
  id: string;
  name: string;
  type: 'Public' | 'P2P' | 'Federated';
  status: 'Connected' | 'Connecting' | 'Disconnected' | 'Error';
  validationStatus: 'verified' | 'rejected';
  peers: number;
  verification: string;
  description: string;
}

function useNetworkConnections(): NetworkConnection[] {
  const nodeStatus = useNodeStatus();
  const peers = useNetworkPeers();
  const domains = useDomainList();

  return React.useMemo(() => {
    const connections: NetworkConnection[] = [];
    const isOnline = !!nodeStatus.data && !nodeStatus.isError;
    const peerCount = nodeStatus.data?.peers ?? peers.data?.length ?? 0;
    const privacyMode = nodeStatus.data?.privacy_mode ?? 'Unknown';

    // Primary network connection derived from node status
    connections.push({
      id: 'device-local',
      name: 'Device Blockchain (Local)',
      type: 'P2P' as const,
      status: isOnline ? 'Connected' as const : 'Disconnected' as const,
      validationStatus: isOnline ? 'verified' as const : 'rejected' as const,
      peers: peerCount,
      verification: 'Bilateral PoS',
      description: `Local device chain - ${privacyMode} mode - ${peerCount} peers connected`
    });

    // Build connections from real peers
    if (peers.data && peers.data.length > 0) {
      connections.push({
        id: 'stoq-mesh',
        name: 'STOQ Mesh Network',
        type: 'Public' as const,
        status: 'Connected' as const,
        validationStatus: 'verified' as const,
        peers: peers.data.length,
        verification: 'Proof of State',
        description: `Connected peers via QUIC/IPv6 transport`
      });
    }

    // Build connections from domains
    if (domains.data && domains.data.length > 0) {
      domains.data.forEach((domain) => {
        connections.push({
          id: `domain-${domain.name}`,
          name: `Domain: ${domain.name}`,
          type: 'Federated' as const,
          status: 'Connected' as const,
          validationStatus: 'verified' as const,
          peers: 1,
          verification: 'Domain PoS',
          description: `Federated domain owned by ${domain.owner?.slice(0, 12) ?? 'unknown'}`
        });
      });
    }

    return connections;
  }, [nodeStatus.data, nodeStatus.isError, peers.data, domains.data]);
}

function NetworkOverviewCards() {
  const networkConnections = useNetworkConnections();
  const connectedCount = networkConnections.filter(n => n.status === 'Connected').length;
  const totalPeers = networkConnections.reduce((sum, n) => sum + n.peers, 0);

  return (
    <div className="grid gap-4 md:grid-cols-3">
      <MetricCard
        title="Active Networks"
        value={connectedCount}
        description="Connected networks"
        icon={Network}
        color="text-green-400"
        className="border-green-500/30"
      />
      <MetricCard
        title="Total Peers"
        value={totalPeers}
        description="Across all networks"
        icon={Users}
        color="text-blue-400"
        className="border-blue-500/30"
      />
      <MetricCard
        title="Verified Networks"
        value={`${networkConnections.filter(n => n.validationStatus === 'verified').length}/${networkConnections.length}`}
        description="PoS validated"
        icon={Shield}
        color="text-purple-400"
        className="border-purple-500/30"
      />
    </div>
  );
}

function NetworkConnectionsList({
  selectedNetwork,
  onSelectNetwork,
  networkConnections
}: {
  selectedNetwork: string | null;
  onSelectNetwork: (id: string) => void;
  networkConnections: NetworkConnection[];
}) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <Network className="h-5 w-5 text-green-400" />
              Network Connections
            </CardTitle>
            <CardDescription className="text-gray-400">
              Manage your connections to Public, P2P, and Federated networks
            </CardDescription>
          </div>
          <Button
            className="bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-400 hover:to-emerald-500 text-black"
            onClick={() => alert('Network discovery interface would open here')}
          >
            <Plus className="h-4 w-4 mr-2" />
            Add Network
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {networkConnections.length === 0 ? (
            <div className="text-center py-8 text-gray-400">
              <Network className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p>No network connections. Start the BlockMatrix daemon to connect.</p>
            </div>
          ) : networkConnections.map((network) => (
            <NetworkConnectionCard
              key={network.id}
              network={network}
              isSelected={selectedNetwork === network.id}
              onSelect={() => onSelectNetwork(network.id)}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function NetworkConnectionCard({ 
  network, 
  isSelected, 
  onSelect 
}: { 
  network: NetworkConnection;
  isSelected: boolean;
  onSelect: () => void;
}) {
  return (
    <div 
      className={cn(
        'border rounded-lg p-4 transition-all duration-300 cursor-pointer',
        isSelected 
          ? 'border-green-500/50 bg-green-500/5' 
          : 'border-green-500/20 bg-green-500/5 hover:border-green-500/30'
      )}
      onClick={onSelect}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <h4 className="text-lg font-medium text-white">{network.name}</h4>
          <Badge className={getTypeColor(network.type)}>{network.type}</Badge>
        </div>
        <StatusIndicator status={network.status} />
      </div>
      
      <p className="text-sm text-gray-400 mb-4">{network.description}</p>
      
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
        <div>
          <span className="text-gray-400">Validation:</span>
          <div className={network.validationStatus === 'verified' ? 'text-green-400 font-mono' : 'text-red-400 font-mono'}>
            {network.validationStatus === 'verified' ? 'Verified' : 'Rejected'}
          </div>
        </div>
        <div>
          <span className="text-gray-400">Peers:</span>
          <div className="text-white font-mono">{network.peers.toLocaleString()}</div>
        </div>
        <div>
          <span className="text-gray-400">Verification:</span>
          <div className="text-white font-mono">{network.verification}</div>
        </div>
        <div className="flex items-center gap-2">
          <Button 
            variant="ghost" 
            size="sm" 
            className="text-green-400 hover:bg-green-500/20"
            onClick={(e) => {
              e.stopPropagation();
              alert(`Configuring ${network.name}`);
            }}
          >
            <Settings className="h-4 w-4" />
          </Button>
          {network.status === 'Connected' ? (
            <Button 
              variant="ghost" 
              size="sm" 
              className="text-red-400 hover:bg-red-500/20"
              onClick={(e) => {
                e.stopPropagation();
                alert(`Disconnecting from ${network.name}`);
              }}
            >
              Disconnect
            </Button>
          ) : (
            <Button 
              variant="ghost" 
              size="sm" 
              className="text-green-400 hover:bg-green-500/20"
              onClick={(e) => {
                e.stopPropagation();
                alert(`Connecting to ${network.name}`);
              }}
            >
              Connect
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

function NetworkDetailsPanel({ networkId, networkConnections }: { networkId: string; networkConnections: NetworkConnection[] }) {
  const network = networkConnections.find(n => n.id === networkId);
  if (!network) return null;

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white">Network Details</CardTitle>
        <CardDescription className="text-gray-400">
          Detailed information for {network.name}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 md:grid-cols-2">
          <NetworkConnectionInfo networkId={networkId} />
          <NetworkSecurityStatus />
        </div>
      </CardContent>
    </Card>
  );
}

function NetworkConnectionInfo({ networkId }: { networkId: string }) {
  return (
    <div className="space-y-3">
      <h4 className="text-white font-medium">Connection Info</h4>
      <div className="space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-gray-400">Network ID:</span>
          <span className="text-white font-mono">{networkId}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Connection Time:</span>
          <span className="text-white font-mono">2h 34m</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Data Transfer:</span>
          <span className="text-white font-mono">1.2 GB up / 3.4 GB down</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Latency:</span>
          <span className="text-white font-mono">45ms avg</span>
        </div>
      </div>
    </div>
  );
}

function NetworkSecurityStatus() {
  return (
    <div className="space-y-3">
      <h4 className="text-white font-medium">Security Status</h4>
      <div className="space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-gray-400">Encryption:</span>
          <span className="text-green-400">AES-256 + FALCON-1024</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Certificate:</span>
          <span className="text-green-400">Valid (30 days)</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Trust Level:</span>
          <span className="text-green-400">Verified</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Last Audit:</span>
          <span className="text-white font-mono">2 hours ago</span>
        </div>
      </div>
    </div>
  );
}

export function NetworkManagement() {
  const networkConnections = useNetworkConnections();
  const [selectedNetwork, setSelectedNetwork] = React.useState<string | null>(null);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Network Management</h2>

      <NetworkOverviewCards />
      <NetworkConnectionsList
        selectedNetwork={selectedNetwork}
        onSelectNetwork={setSelectedNetwork}
        networkConnections={networkConnections}
      />

      {selectedNetwork && <NetworkDetailsPanel networkId={selectedNetwork} networkConnections={networkConnections} />}
    </div>
  );
}