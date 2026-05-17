// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { CertificatesPanel } from './CertificatesPanel';
import type { CertList, CertRecord } from '@/lib/blockmatrix-api';

const meta = {
  title: 'Pages/CertificatesPanel',
  component: CertificatesPanel,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof CertificatesPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

function makeClient(seeds: Array<[readonly unknown[], unknown]> = []): QueryClient {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, refetchOnMount: false },
    },
  });
  for (const [key, value] of seeds) client.setQueryData(key, value);
  return client;
}

const withProviders = (seeds: Array<[readonly unknown[], unknown]> = []) =>
  function Decorator(Story: React.FC) {
    return (
      <QueryClientProvider client={makeClient(seeds)}>
        <MemoryRouter>
          <div className="bg-black min-h-screen p-6">
            <Story />
          </div>
        </MemoryRouter>
      </QueryClientProvider>
    );
  };

const selfSignedCert: CertRecord = {
  id: 'cert-self-001',
  subject: 'CN=node-ab12cd34ef56',
  issuer: 'CN=node-ab12cd34ef56 (self-signed)',
  valid_from: '2026-01-01T00:00:00Z',
  valid_to: '2027-01-01T00:00:00Z',
  status: 'active',
  serial_number: '01',
  signature_algorithm: 'FALCON-1024',
  signature_algorithm_oid: '1.3.9999.6.7.4',
  key_algorithm: 'FALCON-1024',
  key_algorithm_oid: '1.3.9999.6.7.4',
  fingerprint_sha256: 'sha256:7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b',
  fingerprint_blake3: 'blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcd',
  key_usage: ['digitalSignature', 'keyCertSign'],
  extended_key_usage: ['serverAuth', 'clientAuth'],
  subject_alt_names: ['IP:fd48:4d00::1', 'DNS:trust.hypermesh.online'],
  extensions: [
    { oid: '2.5.29.19', critical: true, name: 'basicConstraints' },
    { oid: '2.5.29.15', critical: true, name: 'keyUsage' },
  ],
  path: '/var/lib/hypermesh/trustchain/self.pem',
};

const expiredCert: CertRecord = {
  ...selfSignedCert,
  id: 'cert-expired-001',
  status: 'expired',
  subject: 'CN=node-old-identity',
  valid_from: '2024-01-01T00:00:00Z',
  valid_to: '2025-01-01T00:00:00Z',
};

export const WithSelfSigned: Story = {
  decorators: [
    withProviders([
      [
        ['trustchain', 'certs'],
        {
          node_id: 'ab12cd34ef56',
          certificates: [selfSignedCert],
          total: 1,
          status: 'ok',
        } satisfies CertList,
      ],
    ]),
  ],
};

export const WithMultiple: Story = {
  decorators: [
    withProviders([
      [
        ['trustchain', 'certs'],
        {
          node_id: 'ab12cd34ef56',
          certificates: [selfSignedCert, expiredCert],
          total: 2,
          status: 'ok',
        } satisfies CertList,
      ],
    ]),
  ],
};

export const Empty: Story = {
  decorators: [
    withProviders([
      [
        ['trustchain', 'certs'],
        {
          node_id: 'ab12cd34ef56',
          certificates: [],
          total: 0,
          status: 'ok',
        } satisfies CertList,
      ],
    ]),
  ],
  parameters: {
    docs: {
      description: {
        story: 'No certificate on disk — fresh node before bootstrap completes.',
      },
    },
  },
};
