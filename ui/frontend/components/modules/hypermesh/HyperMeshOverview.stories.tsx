// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { HyperMeshOverview } from './HyperMeshOverview';

const meta = {
  title: 'Pages/HyperMeshOverview',
  component: HyperMeshOverview,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof HyperMeshOverview>;

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
  node_id: 'ab12cd34ef56',
  coordinate: { x: 0, y: 0, z: 0 },
  chain_height: 8421,
  privacy_mode: 'Anonymous',
  peers: 7,
  uptime_secs: 86_400,
};

export const Connected: Story = {
  decorators: [
    withProviders([
      [['blockmatrix', 'status'], baseStatus],
      [['blockmatrix', 'blockchain', 'height'], { height: 8421 }],
      [
        ['blockmatrix', 'network', 'peers'],
        Array.from({ length: 7 }, (_, i) => ({
          node_id: `peer-${i}`,
          address: `fd48:4d00::${i + 1}`,
        })),
      ],
      [
        ['blockmatrix', 'asset', 'list'],
        Array.from({ length: 12 }, (_, i) => ({
          id: `a-${i}`,
          category: i % 2 === 0 ? 'system' : 'application',
          content_hash: `h-${i}`,
          block_index: i * 30,
        })),
      ],
    ]),
  ],
};

export const Empty: Story = {
  decorators: [
    withProviders([
      [['blockmatrix', 'status'], { ...baseStatus, peers: 0, chain_height: 1 }],
      [['blockmatrix', 'blockchain', 'height'], { height: 1 }],
      [['blockmatrix', 'network', 'peers'], []],
      [['blockmatrix', 'asset', 'list'], []],
    ]),
  ],
};
