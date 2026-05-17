// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { Header } from './Header';

const meta = {
  title: 'Organisms/Header',
  component: Header,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof Header>;

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

const withProviders = (
  seeds: Array<[readonly unknown[], unknown]> = [],
  route = '/',
) =>
  function Decorator(Story: React.FC) {
    return (
      <QueryClientProvider client={makeClient(seeds)}>
        <MemoryRouter initialEntries={[route]}>
          <div className="bg-black min-h-screen">
            <Story />
          </div>
        </MemoryRouter>
      </QueryClientProvider>
    );
  };

const baseStatus = {
  node_id: 'ab12cd34ef56',
  coordinate: { x: 0, y: 0, z: 0 },
  chain_height: 8421,
  privacy_mode: 'Anonymous',
  peers: 14,
  uptime_secs: 86_400,
};

export const Connected: Story = {
  decorators: [
    withProviders([[['blockmatrix', 'status'], baseStatus]], '/hypermesh'),
  ],
};

export const TrustChainTheme: Story = {
  decorators: [
    withProviders(
      [[['blockmatrix', 'status'], { ...baseStatus, privacy_mode: 'Private' }]],
      '/trustchain/certificates',
    ),
  ],
};

export const Connecting: Story = {
  decorators: [withProviders([], '/')],
  parameters: {
    docs: {
      description: {
        story: 'No status cached — header pill shows "Connecting...".',
      },
    },
  },
};
