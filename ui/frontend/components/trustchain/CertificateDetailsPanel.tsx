// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useCallback } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { CertificateCard, EnhancedCertificate } from './shared/CertificateCard';
import { CertificateOverview } from './CertificateOverview';
import { CertificateExtensions } from './CertificateExtensions';

interface CertificateDetailsPanelProps {
  certificate: EnhancedCertificate;
  onExport?: (format: 'pem' | 'der' | 'p12') => void;
  onRevoke?: () => void;
  onValidate?: () => void;
  onViewInChain?: () => void;
  className?: string;
}

export function CertificateDetailsPanel({
  certificate,
  onExport,
  onRevoke,
  onValidate,
  onViewInChain,
  className
}: CertificateDetailsPanelProps) {
  const [activeTab, setActiveTab] = useState('overview');

  const handleCopyToClipboard = useCallback((text: string) => {
    navigator.clipboard.writeText(text);
  }, []);

  return (
    <div className={`space-y-6 ${className || ''}`}>
      <CertificateCard
        certificate={certificate}
        onExport={onExport}
        onRevoke={onRevoke}
        onValidate={onValidate}
        onViewInChain={onViewInChain}
      />

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full grid-cols-2 bg-black/20">
          <TabsTrigger value="overview" className="text-white">Overview</TabsTrigger>
          <TabsTrigger value="extensions" className="text-white">Extensions</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="mt-6">
          <CertificateOverview certificate={certificate} />
        </TabsContent>

        <TabsContent value="extensions" className="mt-6">
          <CertificateExtensions 
            certificate={certificate}
            onCopyToClipboard={handleCopyToClipboard}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}

// Export legacy interfaces for compatibility
export type { EnhancedCertificate, CertificateExtension } from './shared/CertificateCard';