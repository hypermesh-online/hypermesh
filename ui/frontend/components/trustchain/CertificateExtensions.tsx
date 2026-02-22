// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Copy, ExternalLink } from 'lucide-react';
import { EnhancedCertificate, CertificateExtension } from './shared/CertificateCard';
import { commonExtensions } from './utils/algorithmInfo';
import { cn } from '@/lib/utils';

interface CertificateExtensionsProps {
  certificate: EnhancedCertificate;
  onCopyToClipboard?: (text: string) => void;
}

export function CertificateExtensions({ 
  certificate, 
  onCopyToClipboard 
}: CertificateExtensionsProps) {
  return (
    <div className="space-y-6">
      <CertificateKeyUsage certificate={certificate} />
      <CertificateExtensionsList 
        extensions={certificate.extensions}
        onCopyToClipboard={onCopyToClipboard}
      />
    </div>
  );
}

function CertificateKeyUsage({ certificate }: { certificate: EnhancedCertificate }) {
  const hasKeyUsage = certificate.keyUsage && certificate.keyUsage.length > 0;
  const hasExtendedKeyUsage = certificate.extendedKeyUsage && certificate.extendedKeyUsage.length > 0;
  const hasSubjectAltNames = certificate.subjectAltNames && certificate.subjectAltNames.length > 0;

  if (!hasKeyUsage && !hasExtendedKeyUsage && !hasSubjectAltNames) {
    return null;
  }

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white">Key Usage & Subject Alternative Names</CardTitle>
        <CardDescription className="text-gray-400">Certificate usage restrictions and alternative identifiers</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {hasKeyUsage && (
          <KeyUsageSection
            title="Key Usage"
            items={certificate.keyUsage!}
            color="text-blue-400"
          />
        )}

        {hasExtendedKeyUsage && (
          <KeyUsageSection
            title="Extended Key Usage"
            items={certificate.extendedKeyUsage!}
            color="text-purple-400"
          />
        )}

        {hasSubjectAltNames && (
          <KeyUsageSection
            title="Subject Alternative Names"
            items={certificate.subjectAltNames!}
            color="text-green-400"
          />
        )}
      </CardContent>
    </Card>
  );
}

function KeyUsageSection({ 
  title, 
  items, 
  color 
}: { 
  title: string;
  items: string[];
  color: string;
}) {
  return (
    <div>
      <h4 className={`text-sm font-medium mb-2 ${color}`}>{title}</h4>
      <div className="flex flex-wrap gap-2">
        {items.map((item, index) => (
          <Badge 
            key={index}
            variant="outline"
            className="text-white border-gray-600 bg-gray-800/50"
          >
            {item}
          </Badge>
        ))}
      </div>
    </div>
  );
}

function CertificateExtensionsList({ 
  extensions, 
  onCopyToClipboard 
}: { 
  extensions: CertificateExtension[];
  onCopyToClipboard?: (text: string) => void;
}) {
  if (!extensions || extensions.length === 0) {
    return (
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardContent className="py-8 text-center">
          <p className="text-gray-400">No certificate extensions found</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white">Certificate Extensions</CardTitle>
        <CardDescription className="text-gray-400">X.509 certificate extensions and their values</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {extensions.map((extension, index) => (
            <ExtensionCard 
              key={index}
              extension={extension}
              onCopyToClipboard={onCopyToClipboard}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function ExtensionCard({ 
  extension, 
  onCopyToClipboard 
}: { 
  extension: CertificateExtension;
  onCopyToClipboard?: (text: string) => void;
}) {
  const extensionName = commonExtensions[extension.oid] || extension.oid;
  
  return (
    <div className="border border-gray-600 rounded-lg p-4 bg-gray-800/30">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <h5 className="text-white font-medium">{extensionName}</h5>
          {extension.critical && (
            <Badge className="bg-red-500/20 text-red-400 border-red-500/30 text-xs">
              Critical
            </Badge>
          )}
        </div>
        
        <div className="flex items-center gap-2">
          {onCopyToClipboard && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onCopyToClipboard(extension.value)}
              className="text-gray-400 hover:text-white"
            >
              <Copy className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>
      
      <div className="space-y-2 text-sm">
        <div>
          <span className="text-gray-400">OID:</span>
          <span className="text-white font-mono ml-2">{extension.oid}</span>
        </div>
        
        <div>
          <span className="text-gray-400">Value:</span>
          <div className="text-white font-mono text-xs bg-gray-900/50 p-2 rounded mt-1 break-all">
            {extension.value}
          </div>
        </div>
        
        {extension.description && (
          <div>
            <span className="text-gray-400">Description:</span>
            <p className="text-gray-300 mt-1">{extension.description}</p>
          </div>
        )}
      </div>
    </div>
  );
}