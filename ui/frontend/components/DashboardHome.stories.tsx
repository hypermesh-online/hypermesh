// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { DashboardHome } from './DashboardHome';

const meta = {
  title: 'Pages/DashboardHome',
  component: DashboardHome,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof DashboardHome>;

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

const baseStatus = {
  node_id: 'ab12cd34ef56789',
  coordinate: { x: 1, y: 2, z: 3 },
  chain_height: 8421,
  privacy_mode: 'Anonymous',
  peers: 14,
  uptime_secs: 86_400 + 3600,
};

export const Connected: Story = {
  decorators: [
    withProviders([
      [['blockmatrix', 'status'], baseStatus],
      [['blockmatrix', 'blockchain', 'height'], { height: 8421 }],
      [['blockmatrix', 'network', 'peers'], []],
      [['blockmatrix', 'asset', 'list'], []],
    ]),
  ],
};

export const WithAssetsAndPeers: Story = {
  decorators: [
    withProviders([
      [['blockmatrix', 'status'], baseStatus],
      [['blockmatrix', 'blockchain', 'height'], { height: 8421 }],
      [
        ['blockmatrix', 'network', 'peers'],
        [
          { node_id: 'p1', address: 'fd48:4d00::1' },
          { node_id: 'p2', address: 'fd48:4d00::2' },
        ],
      ],
      [
        ['blockmatrix', 'asset', 'list'],
        [
          { id: 'a1', category: 'system', content_hash: 'h1', block_index: 12 },
          { id: 'a2', category: 'application', content_hash: 'h2', block_index: 14 },
        ],
      ],
    ]),
  ],
};

export const Offline: Story = {
  decorators: [withProviders([])],
  parameters: {
    docs: {
      description: {
        story:
          'No cached status — the dashboard renders an "isOnline=false" view with the disconnected indicator and zero-valued metrics.',
      },
    },
  },
};
