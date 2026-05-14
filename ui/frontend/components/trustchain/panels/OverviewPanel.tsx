// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import {
  useNodeStatus,
  useBlockchainHeight,
  useNetworkPeers,
  useTrustchainStatus,
} from '@/lib/hooks/useBlockMatrix';
import { EcosystemMetricsDashboard } from '../EcosystemMetricsDashboard';
import type { SystemStatus as EcoSystemStatus } from '../EcosystemMetricsDashboard';

export function OverviewPanel() {
  const nodeStatus = useNodeStatus();
  const chainHeight = useBlockchainHeight();
  const peers = useNetworkPeers();
  const trustchainStatus = useTrustchainStatus();

  const isNodeOnline = nodeStatus.data !== undefined && !nodeStatus.isError;
  const uptimeMs = (nodeStatus.data?.uptime_secs ?? 0) * 1000;
  const caPhase = typeof trustchainStatus.data?.ca_phase === 'string'
    ? trustchainStatus.data.ca_phase
    : 'unknown';
  const signingAlgorithm = typeof trustchainStatus.data?.signing_algorithm === 'string'
    ? trustchainStatus.data.signing_algorithm
    : 'FALCON-1024';

  const systemStatuses: EcoSystemStatus[] = [
    {
      name: 'TrustChain CA',
      status: isNodeOnline ? ('online' as const) : ('offline' as const),
      uptime: uptimeMs,
      lastChecked: new Date().toISOString(),
      metrics: {
        'CA Phase': caPhase.replace(/_/g, ' '),
        'Chain Height': chainHeight.data?.height?.toLocaleString() ?? '--',
        'Node ID': nodeStatus.data?.node_id?.slice(0, 12) ?? '--',
      },
      description: 'Certificate Authority and self-signed identity bootstrap',
    },
    {
      name: 'STOQ Transport',
      status: isNodeOnline ? ('online' as const) : ('offline' as const),
      uptime: uptimeMs,
      lastChecked: new Date().toISOString(),
      metrics: {
        'Connected Peers': String(nodeStatus.data?.peers ?? peers.data?.length ?? 0),
        Protocol: 'QUIC/IPv6',
        Signing: signingAlgorithm,
      },
      description: 'QUIC transport with PoS authentication',
    },
    {
      name: 'eBPF Security',
      status: isNodeOnline ? ('online' as const) : ('offline' as const),
      uptime: uptimeMs,
      lastChecked: new Date().toISOString(),
      metrics: {
        Validation: isNodeOnline ? 'Active' : 'Inactive',
        Mode: 'XDP',
        Policies: isNodeOnline ? 'Enforced' : 'None',
      },
      description: 'eBPF packet processing and security enforcement',
    },
  ];

  return (
    <EcosystemMetricsDashboard
      systemStatuses={systemStatuses}
      loading={nodeStatus.isLoading}
      onRefresh={() => {
        nodeStatus.refetch();
        chainHeight.refetch();
        peers.refetch();
        trustchainStatus.refetch();
      }}
    />
  );
}
