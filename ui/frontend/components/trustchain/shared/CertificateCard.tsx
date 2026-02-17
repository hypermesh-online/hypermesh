// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { CheckCircle, XCircle, AlertTriangle, Clock, Shield } from 'lucide-react';
import { StatusIndicator } from './StatusIndicator';

export interface CertificateExtension {
  oid: string;
  critical: boolean;
  value: string;
  description?: string;
}

export interface EnhancedCertificate {
  id: string;
  subject: string;
  issuer: string;
  validFrom: string;
  validTo: string;
  status: 'active' | 'expired' | 'revoked' | 'pending';
  trustLevel: 'root' | 'intermediate' | 'leaf';
  fingerprint: string;
  keyAlgorithm: string;
  signatureAlgorithm: string;
  extensions: CertificateExtension[];
  keyUsage?: string[];
  extendedKeyUsage?: string[];
  subjectAltNames?: string[];
  issuerAltNames?: string[];
  crlDistributionPoints?: string[];
  authorityInfoAccess?: string[];
  basicConstraints?: {
    isCA: boolean;
    pathLenConstraint?: number;
  };
}

interface CertificateCardProps {
  certificate: EnhancedCertificate;
  onExport?: (format: 'pem' | 'der' | 'p12') => void;
  onRevoke?: () => void;
  onValidate?: () => void;
  onViewInChain?: () => void;
  className?: string;
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'active':
      return <CheckCircle className="h-5 w-5 text-green-400" />;
    case 'expired':
      return <Clock className="h-5 w-5 text-yellow-400" />;
    case 'revoked':
      return <XCircle className="h-5 w-5 text-red-400" />;
    case 'pending':
      return <AlertTriangle className="h-5 w-5 text-yellow-400" />;
    default:
      return <Shield className="h-5 w-5 text-gray-400" />;
  }
}

function getTrustLevelColor(level: string): string {
  switch (level) {
    case 'root':
      return 'text-green-400 bg-green-500/20 border-green-500/30';
    case 'intermediate':
      return 'text-blue-400 bg-blue-500/20 border-blue-500/30';
    case 'leaf':
      return 'text-purple-400 bg-purple-500/20 border-purple-500/30';
    default:
      return 'text-gray-400 bg-gray-500/20 border-gray-500/30';
  }
}

export function CertificateCard({
  certificate,
  onExport,
  onRevoke,
  onValidate,
  onViewInChain,
  className
}: CertificateCardProps) {
  return (
    <div className={`space-y-4 ${className || ''}`}>
      <CertificateHeader certificate={certificate} />
      <CertificateActions 
        onExport={onExport}
        onRevoke={onRevoke}
        onValidate={onValidate}
        onViewInChain={onViewInChain}
      />
    </div>
  );
}

function CertificateHeader({ certificate }: { certificate: EnhancedCertificate }) {
  return (
    <div className="flex items-center justify-between mb-4">
      <div className="flex items-center gap-3">
        {getStatusIcon(certificate.status)}
        <div>
          <h3 className="text-lg font-medium text-white">{certificate.subject}</h3>
          <p className="text-sm text-gray-400">Issued by: {certificate.issuer}</p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <StatusIndicator status={certificate.status} />
        <Badge className={getTrustLevelColor(certificate.trustLevel)}>
          {certificate.trustLevel}
        </Badge>
      </div>
    </div>
  );
}

function CertificateActions({
  onExport,
  onRevoke,
  onValidate,
  onViewInChain
}: {
  onExport?: (format: 'pem' | 'der' | 'p12') => void;
  onRevoke?: () => void;
  onValidate?: () => void;
  onViewInChain?: () => void;
}) {
  return (
    <div className="flex items-center gap-2">
      {onExport && (
        <Button
          variant="outline"
          size="sm"
          onClick={() => onExport('pem')}
          className="border-green-500/30 text-green-400"
        >
          Export PEM
        </Button>
      )}
      {onValidate && (
        <Button
          variant="outline"
          size="sm"
          onClick={onValidate}
          className="border-blue-500/30 text-blue-400"
        >
          Validate
        </Button>
      )}
      {onViewInChain && (
        <Button
          variant="outline"
          size="sm"
          onClick={onViewInChain}
          className="border-purple-500/30 text-purple-400"
        >
          View in Chain
        </Button>
      )}
      {onRevoke && (
        <Button
          variant="outline"
          size="sm"
          onClick={onRevoke}
          className="border-red-500/30 text-red-400"
        >
          Revoke
        </Button>
      )}
    </div>
  );
}