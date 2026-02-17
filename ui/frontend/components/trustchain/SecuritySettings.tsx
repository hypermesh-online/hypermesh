// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useCertificates, useTrustHierarchy } from '@/lib/api';
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
  const { certificates, isLoading: certsLoading } = useCertificates();
  
  const securityStats = {
    activeCertificates: certificates?.filter(c => c.status === 'active').length || 0,
    expiringSoon: certificates?.filter(c => {
      const daysUntilExpiry = Math.ceil((new Date(c.validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
      return daysUntilExpiry <= 30;
    }).length || 0,
    revokedCertificates: certificates?.filter(c => c.status === 'revoked').length || 0,
    totalCertificates: certificates?.length || 0
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
        title="Revoked"
        value={securityStats.revokedCertificates}
        description="Revoked certificates"
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
  const { certificates, isLoading: certsLoading } = useCertificates();
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
        ) : certificates && certificates.length > 0 ? (
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
  certificates: any[];
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
  certificate: any;
  isSelected: boolean;
  onSelect: () => void;
}) {
  const daysUntilExpiry = Math.ceil((new Date(certificate.validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
  const expiringSoon = isExpiringSoon(certificate.validTo);
  const expired = isExpired(certificate.validTo);

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
          Fingerprint: <span className="text-gray-300 font-mono">{certificate.fingerprint.slice(0, 32)}...</span>
        </div>
      </div>
    </div>
  );
}

function CertificateActions({ certificate }: { certificate: any }) {
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

function CertificateDetails({ certificate }: { certificate: any }) {
  const daysUntilExpiry = Math.ceil((new Date(certificate.validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
  const expiringSoon = isExpiringSoon(certificate.validTo);
  const expired = isExpired(certificate.validTo);

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
      <div>
        <span className="text-gray-400">Issuer:</span>
        <div className="text-white font-mono text-xs truncate">{certificate.issuer}</div>
      </div>
      <div>
        <span className="text-gray-400">Valid From:</span>
        <div className="text-white font-mono text-xs">{new Date(certificate.validFrom).toLocaleDateString()}</div>
      </div>
      <div>
        <span className="text-gray-400">Valid To:</span>
        <div className={cn(
          'font-mono text-xs',
          expired ? 'text-red-400' : expiringSoon ? 'text-yellow-400' : 'text-white'
        )}>
          {new Date(certificate.validTo).toLocaleDateString()}
        </div>
      </div>
      <div>
        <span className="text-gray-400">Trust Level:</span>
        <div className={cn('font-mono text-xs', getTrustLevelColor(certificate.trustLevel))}>
          {certificate.trustLevel}
        </div>
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

function TrustHierarchyCard() {
  const { data: trustHierarchy, isLoading: hierarchyLoading } = useTrustHierarchy();

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Shield className="h-5 w-5 text-green-400" />
          Trust Hierarchy
        </CardTitle>
        <CardDescription className="text-gray-400">
          Certificate authority chain and trust relationships
        </CardDescription>
      </CardHeader>
      <CardContent>
        {hierarchyLoading ? (
          <TrustHierarchyLoadingSkeleton />
        ) : trustHierarchy ? (
          <TrustHierarchyTree hierarchy={trustHierarchy} />
        ) : (
          <div className="text-center py-6 text-gray-400">
            No trust hierarchy information available
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function TrustHierarchyLoadingSkeleton() {
  return (
    <div className="animate-pulse space-y-3">
      <div className="h-8 bg-gray-700 rounded w-full"></div>
      <div className="h-6 bg-gray-700 rounded w-3/4"></div>
      <div className="h-6 bg-gray-700 rounded w-1/2"></div>
    </div>
  );
}

function TrustHierarchyTree({ hierarchy }: { hierarchy: any }) {
  return (
    <div className="space-y-4">
      <TrustHierarchyRootCA rootCA={hierarchy.rootCA} />
      <TrustHierarchyIntermediates intermediates={hierarchy.intermediates} />
      <TrustHierarchyLeaves leaves={hierarchy.leaves} />
      
      <div className="pt-3 border-t border-green-500/20 text-xs text-gray-500">
        Last validated: {new Date(hierarchy.lastValidated).toLocaleString()}
      </div>
    </div>
  );
}

function TrustHierarchyRootCA({ rootCA }: { rootCA: any }) {
  return (
    <div className="border border-green-500/30 rounded-lg p-3 bg-green-500/5">
      <div className="flex items-center gap-2 mb-2">
        <Shield className="h-4 w-4 text-green-400" />
        <span className="text-white font-medium">Root CA</span>
        <Badge className="bg-green-500/20 text-green-400 border-green-500/30 text-xs">Trusted</Badge>
      </div>
      <div className="text-sm text-gray-400">{rootCA.subject}</div>
      <div className="text-xs text-gray-500 mt-1">Fingerprint: {rootCA.fingerprint.slice(0, 24)}...</div>
    </div>
  );
}

function TrustHierarchyIntermediates({ intermediates }: { intermediates: any[] }) {
  return (
    <>
      {intermediates.map((cert, index) => (
        <div key={cert.id} className="ml-4 border border-blue-500/30 rounded-lg p-3 bg-blue-500/5">
          <div className="flex items-center gap-2 mb-2">
            <Key className="h-4 w-4 text-blue-400" />
            <span className="text-white font-medium">Intermediate CA {index + 1}</span>
            <StatusIndicator status={cert.status} size="sm" />
          </div>
          <div className="text-sm text-gray-400">{cert.subject}</div>
          <div className="text-xs text-gray-500 mt-1">Valid until: {new Date(cert.validTo).toLocaleDateString()}</div>
        </div>
      ))}
    </>
  );
}

function TrustHierarchyLeaves({ leaves }: { leaves: any[] }) {
  return (
    <>
      {leaves.slice(0, 3).map((cert, index) => (
        <div key={cert.id} className="ml-8 border border-purple-500/30 rounded-lg p-3 bg-purple-500/5">
          <div className="flex items-center gap-2 mb-2">
            <Lock className="h-4 w-4 text-purple-400" />
            <span className="text-white font-medium">End Entity {index + 1}</span>
            <StatusIndicator status={cert.status} size="sm" />
          </div>
          <div className="text-sm text-gray-400">{cert.subject}</div>
          <div className="text-xs text-gray-500 mt-1">Valid until: {new Date(cert.validTo).toLocaleDateString()}</div>
        </div>
      ))}
    </>
  );
}

export function SecuritySettings() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Security Settings</h2>
      
      <SecurityOverviewCards />
      <CertificateManagementCard />
      <TrustHierarchyCard />
    </div>
  );
}