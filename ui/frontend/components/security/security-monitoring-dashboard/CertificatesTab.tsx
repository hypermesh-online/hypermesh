// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Key, CheckCircle, XCircle } from 'lucide-react';

interface CertificatesTabProps {
  certificates: any[] | undefined;
  certsLoading: boolean;
  systemStatus: any;
  onValidate: () => void;
  isValidating: boolean;
}

export function CertificatesTab({
  certificates,
  certsLoading,
  systemStatus,
  onValidate,
  isValidating
}: CertificatesTabProps) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <Key className="h-5 w-5 text-green-400" />
              TrustChain Certificate Authority
            </CardTitle>
            <CardDescription className="text-gray-400">X.509 certificate management with post-quantum cryptography</CardDescription>
          </div>
          <Button
            onClick={onValidate}
            disabled={isValidating || certsLoading}
            className="bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-400 hover:to-emerald-500 text-black"
          >
            {isValidating ? 'Validating...' : 'Validate Certificates'}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {certsLoading ? (
          <div className="space-y-3">
            {[1,2,3].map(i => (
              <div key={i} className="animate-pulse h-16 bg-gray-700 rounded-lg"></div>
            ))}
          </div>
        ) : certificates && certificates.length > 0 ? (
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {certificates.slice(0, 10).map((cert) => {
              const daysUntilExpiry = Math.ceil((new Date(cert.validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
              const isExpiringSoon = daysUntilExpiry <= 30 && daysUntilExpiry > 0;
              const isExpired = daysUntilExpiry <= 0;

              return (
                <div key={cert.id} className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="text-white font-medium">{cert.subject}</h4>
                      <Badge variant="outline" className={cn(
                        'text-xs',
                        cert.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                        cert.status === 'revoked' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                        'bg-yellow-500/20 text-yellow-400 border-yellow-500/30'
                      )}>
                        {cert.status}
                      </Badge>
                      {isExpired && (
                        <Badge variant="outline" className="text-xs bg-red-500/20 text-red-400 border-red-500/30">
                          Expired
                        </Badge>
                      )}
                      {isExpiringSoon && !isExpired && (
                        <Badge variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
                          Expiring Soon
                        </Badge>
                      )}
                    </div>
                    <div className="text-sm text-gray-400">
                      Serial: {cert.serialNumber} -
                      Expires: {new Date(cert.validTo).toLocaleDateString()} ({daysUntilExpiry} days)
                    </div>
                    <div className="text-xs text-gray-500">
                      Issuer: {cert.issuer || 'TrustChain CA'} - Trust Level: {cert.trustLevel || 'leaf'}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {cert.status === 'active' ? (
                      <CheckCircle className="h-4 w-4 text-green-400" />
                    ) : (
                      <XCircle className="h-4 w-4 text-red-400" />
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="text-center py-8 text-gray-400">
            {systemStatus ? 'No certificates available' : 'System offline - unable to load certificates'}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
