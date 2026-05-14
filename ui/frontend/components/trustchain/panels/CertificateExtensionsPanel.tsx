// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { FileText } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTrustchainCerts } from '@/lib/hooks/useBlockMatrix';
import type { CertExtension, CertRecord } from '@/lib/blockmatrix-api';

export function CertificateExtensionsPanel() {
  const certsQuery = useTrustchainCerts();
  const certificates: CertRecord[] = certsQuery.data?.certificates ?? [];
  const [selectedId, setSelectedId] = React.useState<string | null>(null);

  if (certsQuery.isLoading) {
    return (
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardContent className="py-8">
          <div className="h-32 bg-gray-700 rounded-lg animate-pulse" />
        </CardContent>
      </Card>
    );
  }

  if (certsQuery.error) {
    return (
      <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
        <CardContent className="py-8 text-center text-red-400">
          Failed to load certificates: {String((certsQuery.error as Error).message ?? certsQuery.error)}
        </CardContent>
      </Card>
    );
  }

  if (certificates.length === 0) {
    return (
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardContent className="py-12 text-center">
          <FileText className="h-12 w-12 text-gray-600 mx-auto mb-3" />
          <h3 className="text-lg font-medium text-white mb-2">No Certificates</h3>
          <p className="text-gray-400">No certificates issued yet — register a certificate to see extension data.</p>
        </CardContent>
      </Card>
    );
  }

  const selectedCert = certificates.find((c) => c.id === selectedId) ?? certificates[0];

  return (
    <div className="space-y-6">
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <FileText className="h-5 w-5 text-green-400" />
            Certificate Extensions
          </CardTitle>
          <CardDescription className="text-gray-400">
            X.509 v3 extensions parsed from each certificate
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            {certificates.map((cert) => (
              <button
                key={cert.id}
                onClick={() => setSelectedId(cert.id)}
                className={`px-3 py-1.5 rounded-lg border text-sm transition-all ${
                  selectedCert.id === cert.id
                    ? 'border-green-500/50 bg-green-500/10 text-green-400'
                    : 'border-gray-600 bg-gray-800/30 text-gray-300 hover:border-green-500/30'
                }`}
              >
                {cert.subject}
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <CertExtensionsView cert={selectedCert} />
    </div>
  );
}

function CertExtensionsView({ cert }: { cert: CertRecord }) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white text-base truncate">{cert.subject}</CardTitle>
        <CardDescription className="text-gray-400">
          Key usage, alternative names, and raw extension OIDs
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <ChipSection
          title="Key Usage"
          items={cert.key_usage}
          color="text-blue-400"
          chipBorder="border-blue-500/30"
          chipBg="bg-blue-500/10"
        />
        <ChipSection
          title="Extended Key Usage"
          items={cert.extended_key_usage}
          color="text-purple-400"
          chipBorder="border-purple-500/30"
          chipBg="bg-purple-500/10"
        />
        <ChipSection
          title="Subject Alternative Names"
          items={cert.subject_alt_names}
          color="text-green-400"
          chipBorder="border-green-500/30"
          chipBg="bg-green-500/10"
        />

        <Separator className="bg-green-500/20" />

        <ExtensionsTable extensions={cert.extensions} />
      </CardContent>
    </Card>
  );
}

function ChipSection({
  title,
  items,
  color,
  chipBorder,
  chipBg,
}: {
  title: string;
  items: string[];
  color: string;
  chipBorder: string;
  chipBg: string;
}) {
  return (
    <div>
      <h4 className={cn('text-sm font-medium mb-2', color)}>{title}</h4>
      {items.length === 0 ? (
        <span className="text-gray-500 text-sm">(none)</span>
      ) : (
        <div className="flex flex-wrap gap-2">
          {items.map((item, idx) => (
            <Badge
              key={`${item}-${idx}`}
              variant="outline"
              className={cn('text-white font-mono text-xs', chipBorder, chipBg)}
            >
              {item}
            </Badge>
          ))}
        </div>
      )}
    </div>
  );
}

function ExtensionsTable({ extensions }: { extensions: CertExtension[] }) {
  if (extensions.length === 0) {
    return (
      <div>
        <h4 className="text-sm font-medium mb-2 text-gray-300">Extensions</h4>
        <span className="text-gray-500 text-sm">(none)</span>
      </div>
    );
  }

  return (
    <div>
      <h4 className="text-sm font-medium mb-2 text-gray-300">
        Extensions <span className="text-gray-500 font-normal">({extensions.length})</span>
      </h4>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-xs text-gray-400 border-b border-gray-700">
              <th className="py-2 pr-3 font-medium">Name</th>
              <th className="py-2 pr-3 font-medium">OID</th>
              <th className="py-2 pr-3 font-medium">Critical</th>
            </tr>
          </thead>
          <tbody>
            {extensions.map((ext, idx) => (
              <tr key={`${ext.oid}-${idx}`} className="border-b border-gray-800 last:border-0">
                <td className="py-2 pr-3 text-white">{ext.name ?? <span className="text-gray-500">Unknown</span>}</td>
                <td className="py-2 pr-3 text-gray-300 font-mono text-xs break-all">{ext.oid}</td>
                <td className="py-2 pr-3">
                  {ext.critical ? (
                    <Badge className="bg-red-500/20 text-red-400 border-red-500/30 text-xs">yes</Badge>
                  ) : (
                    <Badge className="bg-gray-500/20 text-gray-300 border-gray-500/30 text-xs">no</Badge>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
