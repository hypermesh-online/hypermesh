// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { AlertTriangle, Key, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTrustchainCerts } from '@/lib/hooks/useBlockMatrix';
import type { CertRecord } from '@/lib/blockmatrix-api';

function statusBadgeClass(status: CertRecord['status']): string {
  switch (status) {
    case 'active':
      return 'bg-green-500/20 text-green-400 border-green-500/30';
    case 'expired':
      return 'bg-red-500/20 text-red-400 border-red-500/30';
    case 'not_yet_valid':
      return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
    default:
      return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
  }
}

function statusLabel(status: CertRecord['status']): string {
  switch (status) {
    case 'not_yet_valid':
      return 'not yet valid';
    default:
      return status;
  }
}

function formatDate(rfc3339: string): string {
  const d = new Date(rfc3339);
  return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleString();
}

function truncate(value: string, max: number): string {
  return value.length <= max ? value : `${value.slice(0, max)}…`;
}

export function CertificatesPanel() {
  const certsQuery = useTrustchainCerts();
  const certList = certsQuery.data;
  const certificates: CertRecord[] = certList?.certificates ?? [];
  const [selectedId, setSelectedId] = React.useState<string | null>(null);

  if (certsQuery.isLoading) {
    return (
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardContent className="py-8">
          <div className="space-y-3 animate-pulse">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 bg-gray-700 rounded-lg" />
            ))}
          </div>
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
      <div className="space-y-4">
        {certList?.error && (
          <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
            <CardContent className="py-4 flex items-start gap-3 text-yellow-300">
              <AlertTriangle className="h-5 w-5 flex-shrink-0 mt-0.5" />
              <div>
                <div className="font-medium">Certificate parse error</div>
                <div className="text-sm text-yellow-200/80 font-mono break-all">{certList.error}</div>
              </div>
            </CardContent>
          </Card>
        )}
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardContent className="py-12 text-center">
            <Key className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Certificates</h3>
            <p className="text-gray-400">
              No certificates issued yet. The node operates in self-signed bootstrap mode.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const selectedCert = certificates.find((c) => c.id === selectedId) ?? certificates[0];

  return (
    <div className="space-y-4">
      {certList?.error && (
        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardContent className="py-4 flex items-start gap-3 text-yellow-300">
            <AlertTriangle className="h-5 w-5 flex-shrink-0 mt-0.5" />
            <div>
              <div className="font-medium">Certificate parse error</div>
              <div className="text-sm text-yellow-200/80 font-mono break-all">{certList.error}</div>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Key className="h-5 w-5 text-green-400" />
              Certificates
            </CardTitle>
            <CardDescription className="text-gray-400">
              {certificates.length} {certificates.length === 1 ? 'certificate' : 'certificates'} on disk
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            {certificates.map((cert) => (
              <button
                key={cert.id}
                onClick={() => setSelectedId(cert.id)}
                className={cn(
                  'w-full text-left p-3 border rounded-lg transition-all duration-200',
                  selectedCert.id === cert.id
                    ? 'border-green-500/50 bg-green-500/10'
                    : 'border-green-500/20 hover:border-green-500/30 hover:bg-green-500/5'
                )}
              >
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <Badge className={cn('text-xs', statusBadgeClass(cert.status))}>
                        {statusLabel(cert.status)}
                      </Badge>
                      <span className="text-white font-medium text-sm truncate">{cert.subject}</span>
                    </div>
                    <div className="text-xs text-gray-400 font-mono truncate">{cert.issuer}</div>
                  </div>
                  <ChevronRight className="h-4 w-4 text-gray-500 flex-shrink-0" />
                </div>
              </button>
            ))}
          </CardContent>
        </Card>

        <CertificateDetailsCard cert={selectedCert} />
      </div>
    </div>
  );
}

function CertificateDetailsCard({ cert }: { cert: CertRecord }) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <CardTitle className="text-white flex items-center gap-2">
              <Key className="h-5 w-5 text-green-400" />
              <span className="truncate">{cert.subject}</span>
            </CardTitle>
            <CardDescription className="text-gray-400 truncate">
              Issued by {cert.issuer}
            </CardDescription>
          </div>
          <Badge className={cn('text-xs', statusBadgeClass(cert.status))}>
            {statusLabel(cert.status)}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <Field label="Valid From">
            <span className="text-white font-mono text-sm">{formatDate(cert.valid_from)}</span>
          </Field>
          <Field label="Valid To">
            <span className="text-white font-mono text-sm">{formatDate(cert.valid_to)}</span>
          </Field>
        </div>

        <Separator className="bg-green-500/20" />

        <div className="grid gap-4 md:grid-cols-2">
          <Field label="Key Algorithm">
            <span className="text-green-400 font-mono text-sm">{cert.key_algorithm}</span>
            <span className="text-gray-500 font-mono text-xs">OID {cert.key_algorithm_oid}</span>
          </Field>
          <Field label="Signature Algorithm">
            <span className="text-blue-400 font-mono text-sm">{cert.signature_algorithm}</span>
            <span className="text-gray-500 font-mono text-xs">OID {cert.signature_algorithm_oid}</span>
          </Field>
        </div>

        <Separator className="bg-green-500/20" />

        <div>
          <label className="text-sm text-gray-400">Serial Number</label>
          <div className="text-white font-mono text-xs break-all bg-gray-800/50 p-3 rounded-lg mt-1">
            {truncate(cert.serial_number, 32)}
          </div>
        </div>

        <div>
          <label className="text-sm text-gray-400">Fingerprint (SHA-256)</label>
          <div className="text-white font-mono text-xs break-all bg-gray-800/50 p-3 rounded-lg mt-1">
            {cert.fingerprint_sha256}
          </div>
        </div>

        <div>
          <label className="text-sm text-gray-400">Fingerprint (BLAKE3)</label>
          <div className="text-white font-mono text-xs break-all bg-gray-800/50 p-3 rounded-lg mt-1">
            {cert.fingerprint_blake3}
          </div>
        </div>

        <div>
          <label className="text-sm text-gray-400">Certificate Path</label>
          <div className="text-gray-300 font-mono text-xs break-all bg-gray-800/30 p-3 rounded-lg mt-1">
            {cert.path}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-sm text-gray-400">{label}</label>
      {children}
    </div>
  );
}
