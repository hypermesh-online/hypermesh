// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useTrustchainCerts, useTrustchainFederation } from '@/lib/hooks/useBlockMatrix';
import type { CertRecord, FederationPeer } from '@/lib/blockmatrix-api';
import { MetricCard } from './shared/MetricCard';
import { StatusIndicator } from './shared/StatusIndicator';
import { isExpiringSoon, isExpired } from './utils/dateFormatters';
import { getTrustLevelColor } from './utils/statusHelpers';
import { 
  Shield,
  Key,
  Lock,
  AlertTriangle,
  Plus,
  Upload,
  Download,
  Trash2
} from 'lucide-react';
import { cn } from '@/lib/utils';

function SecurityOverviewCards() {
  const certsQuery = useTrustchainCerts();
  const certificates: CertRecord[] = certsQuery.data?.certificates ?? [];

  const securityStats = {
    activeCertificates: certificates.filter(c => c.status === 'active').length,
    expiringSoon: certificates.filter(c => {
      const daysUntilExpiry = Math.ceil((new Date(c.valid_to).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
      return daysUntilExpiry <= 30 && daysUntilExpiry > 0;
    }).length,
    expiredCertificates: certificates.filter(c => c.status === 'expired').length,
    totalCertificates: certificates.length
  };
  
  return (
    <div className="grid gap-4 md:grid-cols-4">
      <MetricCard
        title="Active Certificates"
        value={securityStats.activeCertificates}
        description="Valid certificates"
        icon={Shield}
        color="text-green-400"
        className="border-green-500/30"
      />
      <MetricCard
        title="Expiring Soon"
        value={securityStats.expiringSoon}
        description="Within 30 days"
        icon={AlertTriangle}
        color="text-yellow-400"
        className="border-yellow-500/30"
      />
      <MetricCard
        title="Expired"
        value={securityStats.expiredCertificates}
        description="Expired certificates"
        icon={Lock}
        color="text-red-400"
        className="border-red-500/30"
      />
      <MetricCard
        title="Total"
        value={securityStats.totalCertificates}
        description="All certificates"
        icon={Key}
        color="text-blue-400"
        className="border-blue-500/30"
      />
    </div>
  );
}

function CertificateManagementCard() {
  const certsQuery = useTrustchainCerts();
  const certificates: CertRecord[] = certsQuery.data?.certificates ?? [];
  const certsLoading = certsQuery.isLoading;
  const certsError = certsQuery.error;
  const [selectedCert, setSelectedCert] = React.useState<string | null>(null);
  
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <Key className="h-5 w-5 text-green-400" />
              Certificate Management
            </CardTitle>
            <CardDescription className="text-gray-400">
              Manage X.509 certificates and trust chains
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button 
              variant="outline" 
              size="sm" 
              className="border-green-500/30 text-green-400"
              onClick={() => alert('Certificate import dialog would open here')}
            >
              <Upload className="h-4 w-4 mr-2" />
              Import
            </Button>
            <Button 
              className="bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-400 hover:to-emerald-500 text-black"
              onClick={() => alert('Certificate generation dialog would open here')}
            >
              <Plus className="h-4 w-4 mr-2" />
              Generate
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {certsLoading ? (
          <CertificateLoadingSkeleton />
        ) : certsError ? (
          <div className="text-center py-6 text-red-400 text-sm">
            Failed to load certificates: {String((certsError as Error).message ?? certsError)}
          </div>
        ) : certificates.length > 0 ? (
          <CertificateList
            certificates={certificates.slice(0, 5)}
            selectedCert={selectedCert}
            onSelectCert={setSelectedCert}
          />
        ) : (
          <EmptyCertificateState />
        )}
      </CardContent>
    </Card>
  );
}

function CertificateLoadingSkeleton() {
  return (
    <div className="space-y-3">
      {[1,2,3].map(i => (
        <div key={i} className="animate-pulse h-16 bg-gray-700 rounded-lg"></div>
      ))}
    </div>
  );
}

function CertificateList({
  certificates,
  selectedCert,
  onSelectCert
}: {
  certificates: CertRecord[];
  selectedCert: string | null;
  onSelectCert: (id: string) => void;
}) {
  return (
    <div className="space-y-3">
      {certificates.map((cert) => (
        <CertificateCard
          key={cert.id}
          certificate={cert}
          isSelected={selectedCert === cert.id}
          onSelect={() => onSelectCert(cert.id)}
        />
      ))}
    </div>
  );
}

function CertificateCard({
  certificate,
  isSelected,
  onSelect
}: {
  certificate: CertRecord;
  isSelected: boolean;
  onSelect: () => void;
}) {
  const expiringSoon = isExpiringSoon(certificate.valid_to);
  const expired = isExpired(certificate.valid_to);

  return (
    <div
      className={cn(
        'border rounded-lg p-4 transition-all duration-300 cursor-pointer',
        isSelected
          ? 'border-green-500/50 bg-green-500/5'
          : 'border-green-500/20 hover:border-green-500/30'
      )}
      onClick={onSelect}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-3">
          <h4 className="text-white font-medium">{certificate.subject}</h4>
          <StatusIndicator status={certificate.status} size="sm" />
          {expiringSoon && !expired && (
            <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30 text-xs">
              Expiring Soon
            </Badge>
          )}
        </div>
        <CertificateActions certificate={certificate} />
      </div>

      <CertificateDetails certificate={certificate} />

      <div className="mt-3 pt-3 border-t border-green-500/20">
        <div className="text-xs text-gray-400">
          Cert ID: <span className="text-gray-300 font-mono">{certificate.id}</span>
        </div>
      </div>
    </div>
  );
}

function CertificateActions({ certificate }: { certificate: CertRecord }) {
  return (
    <div className="flex items-center gap-2">
      <Button
        variant="ghost"
        size="sm"
        className="text-green-400 hover:bg-green-500/20"
        onClick={(e) => {
          e.stopPropagation();
          alert(`Exporting certificate ${certificate.subject}`);
        }}
      >
        <Download className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="sm"
        className="text-red-400 hover:bg-red-500/20"
        onClick={(e) => {
          e.stopPropagation();
          alert(`Revoking certificate ${certificate.subject}`);
        }}
      >
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  );
}

function CertificateDetails({ certificate }: { certificate: CertRecord }) {
  const expiringSoon = isExpiringSoon(certificate.valid_to);
  const expired = isExpired(certificate.valid_to);

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
      <div>
        <span className="text-gray-400">Issuer:</span>
        <div className="text-white font-mono text-xs truncate">{certificate.issuer}</div>
      </div>
      <div>
        <span className="text-gray-400">Valid From:</span>
        <div className="text-white font-mono text-xs">{new Date(certificate.valid_from).toLocaleDateString()}</div>
      </div>
      <div>
        <span className="text-gray-400">Valid To:</span>
        <div className={cn(
          'font-mono text-xs',
          expired ? 'text-red-400' : expiringSoon ? 'text-yellow-400' : 'text-white'
        )}>
          {new Date(certificate.valid_to).toLocaleDateString()}
        </div>
      </div>
      <div>
        <span className="text-gray-400">Key Algorithm:</span>
        <div className="text-cyan-400 font-mono text-xs truncate">{certificate.key_algorithm}</div>
      </div>
    </div>
  );
}

function EmptyCertificateState() {
  return (
    <div className="text-center py-8">
      <Key className="h-12 w-12 text-gray-600 mx-auto mb-3" />
      <h3 className="text-lg font-medium text-white mb-2">No Certificates</h3>
      <p className="text-gray-400">No certificates found. Generate or import certificates to secure your connections.</p>
    </div>
  );
}

function FederationPeersCard() {
  const federationQuery = useTrustchainFederation();
  const peers = federationQuery.data?.peers ?? [];
  const totalPeers = federationQuery.data?.total_peers ?? 0;

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Shield className="h-5 w-5 text-green-400" />
          Federation Peers
        </CardTitle>
        <CardDescription className="text-gray-400">
          TrustChain federated peer CAs and their bilateral trust levels ({totalPeers} total)
        </CardDescription>
      </CardHeader>
      <CardContent>
        {federationQuery.isLoading ? (
          <FederationLoadingSkeleton />
        ) : federationQuery.error ? (
          <div className="text-center py-6 text-red-400 text-sm">
            Failed to load federation: {String((federationQuery.error as Error).message ?? federationQuery.error)}
          </div>
        ) : peers.length > 0 ? (
          <div className="space-y-3">
            {peers.map((peer) => (
              <FederationPeerRow key={peer.node_id} peer={peer} />
            ))}
          </div>
        ) : (
          <div className="text-center py-6 text-gray-400">
            No federation peers configured
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function FederationLoadingSkeleton() {
  return (
    <div className="animate-pulse space-y-3">
      <div className="h-12 bg-gray-700 rounded w-full"></div>
      <div className="h-12 bg-gray-700 rounded w-full"></div>
      <div className="h-12 bg-gray-700 rounded w-2/3"></div>
    </div>
  );
}

function FederationPeerRow({ peer }: { peer: FederationPeer }) {
  const trustColor = getTrustLevelColor(peer.trust_level);
  return (
    <div className="border border-green-500/30 rounded-lg p-3 bg-green-500/5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Lock className="h-4 w-4 text-green-400" />
          <span className="text-white font-mono text-sm truncate max-w-[280px]">{peer.node_id}</span>
        </div>
        <Badge className={cn('text-xs', trustColor)}>
          {peer.trust_level}
        </Badge>
      </div>
    </div>
  );
}

export function SecuritySettings() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Security Settings</h2>

      <SecurityOverviewCards />
      <CertificateManagementCard />
      <FederationPeersCard />
    </div>
  );
}