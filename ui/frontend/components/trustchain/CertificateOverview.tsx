// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Shield, Key, Clock } from 'lucide-react';
import { EnhancedCertificate } from './shared/CertificateCard';
import { calculateDaysUntilExpiry, isExpiringSoon, isExpired } from './utils/dateFormatters';
import { algorithmInfo } from './utils/algorithmInfo';
import { cn } from '@/lib/utils';

interface CertificateOverviewProps {
  certificate: EnhancedCertificate;
}

export function CertificateOverview({ certificate }: CertificateOverviewProps) {
  return (
    <div className="space-y-6">
      <CertificateBasicInfo certificate={certificate} />
      <CertificateValidityInfo certificate={certificate} />
      <CertificateAlgorithmInfo certificate={certificate} />
    </div>
  );
}

function CertificateBasicInfo({ certificate }: { certificate: EnhancedCertificate }) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Shield className="h-5 w-5 text-green-400" />
          Certificate Information
        </CardTitle>
        <CardDescription className="text-gray-400">Basic certificate details and identifiers</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <label className="text-sm text-gray-400">Subject</label>
            <div className="text-white font-mono break-all">{certificate.subject}</div>
          </div>
          <div>
            <label className="text-sm text-gray-400">Issuer</label>
            <div className="text-white font-mono break-all">{certificate.issuer}</div>
          </div>
        </div>
        
        <Separator className="bg-green-500/20" />
        
        <div>
          <label className="text-sm text-gray-400">Fingerprint (SHA-256)</label>
          <div className="text-white font-mono text-sm break-all bg-gray-800/50 p-3 rounded-lg mt-1">
            {certificate.fingerprint}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function CertificateValidityInfo({ certificate }: { certificate: EnhancedCertificate }) {
  const daysUntilExpiry = calculateDaysUntilExpiry(certificate.validTo);
  const expiringSoon = isExpiringSoon(certificate.validTo);
  const expired = isExpired(certificate.validTo);

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Clock className="h-5 w-5 text-green-400" />
          Validity Period
        </CardTitle>
        <CardDescription className="text-gray-400">Certificate validity dates and status</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <label className="text-sm text-gray-400">Valid From</label>
            <div className="text-white font-mono">{new Date(certificate.validFrom).toLocaleString()}</div>
          </div>
          <div>
            <label className="text-sm text-gray-400">Valid To</label>
            <div className={cn(
              'font-mono',
              expired ? 'text-red-400' : expiringSoon ? 'text-yellow-400' : 'text-white'
            )}>
              {new Date(certificate.validTo).toLocaleString()}
            </div>
          </div>
        </div>
        
        <div className="flex items-center gap-4">
          <div>
            <label className="text-sm text-gray-400">Days Until Expiry</label>
            <div className={cn(
              'text-lg font-bold',
              expired ? 'text-red-400' : expiringSoon ? 'text-yellow-400' : 'text-green-400'
            )}>
              {daysUntilExpiry > 0 ? daysUntilExpiry : 'Expired'}
            </div>
          </div>
          {expiringSoon && !expired && (
            <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
              Expiring Soon
            </Badge>
          )}
          {expired && (
            <Badge className="bg-red-500/20 text-red-400 border-red-500/30">
              Expired
            </Badge>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function CertificateAlgorithmInfo({ certificate }: { certificate: EnhancedCertificate }) {
  const keyAlgInfo = algorithmInfo[certificate.keyAlgorithm] || null;
  const sigAlgInfo = algorithmInfo[certificate.signatureAlgorithm] || null;

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Key className="h-5 w-5 text-green-400" />
          Cryptographic Algorithms
        </CardTitle>
        <CardDescription className="text-gray-400">Key and signature algorithm details</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {keyAlgInfo && (
          <AlgorithmInfoCard
            title="Key Algorithm"
            algorithm={certificate.keyAlgorithm}
            info={keyAlgInfo}
          />
        )}
        
        {sigAlgInfo && (
          <AlgorithmInfoCard
            title="Signature Algorithm"
            algorithm={certificate.signatureAlgorithm}
            info={sigAlgInfo}
          />
        )}
        
        {!keyAlgInfo && !sigAlgInfo && (
          <div className="text-center py-4 text-gray-400">
            Algorithm information not available
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function AlgorithmInfoCard({ 
  title, 
  algorithm, 
  info 
}: { 
  title: string;
  algorithm: string;
  info: any;
}) {
  return (
    <div className={`border rounded-lg p-4 ${info.bgColor} border-opacity-30`}>
      <div className="flex items-center justify-between mb-3">
        <h4 className="text-white font-medium">{title}</h4>
        <Badge className={`${info.color} ${info.bgColor} border-opacity-30`}>
          {info.security}
        </Badge>
      </div>
      
      <div className="space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-gray-400">Algorithm:</span>
          <span className={info.color}>{info.name}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Type:</span>
          <span className="text-white">{info.type}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Key Size:</span>
          <span className="text-white">{info.keySize}</span>
        </div>
      </div>
      
      <div className="mt-3 pt-3 border-t border-gray-600">
        <p className="text-xs text-gray-400">{info.description}</p>
      </div>
    </div>
  );
}