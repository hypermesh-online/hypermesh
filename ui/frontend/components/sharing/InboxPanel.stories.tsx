// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { InboxPanel } from './InboxPanel';

const meta = {
  title: 'Organisms/InboxPanel',
  component: InboxPanel,
  parameters: { layout: 'padded' },
  tags: ['autodocs'],
} satisfies Meta<typeof InboxPanel>;

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

export const Empty: Story = {
  decorators: [
    withProviders([
      [['blockmatrix', 'share', 'inbox'], { invites: [], count: 0 }],
    ]),
  ],
};

export const WithItems: Story = {
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'share', 'inbox'],
        {
          count: 2,
          invites: [
            {
              invite_id: 'inv-1',
              asset_id: 'a-001',
              sender_node_id: 'node-abc123def456',
              sender_name: 'lab.private',
              asset_name: 'capture-2026-05.mp4',
              asset_size: 1024 * 1024 * 480,
              shard_count: 14,
              created_at: Math.floor(Date.now() / 1000) - 120,
            },
            {
              invite_id: 'inv-2',
              asset_id: 'a-002',
              sender_node_id: 'node-deadbeef0001',
              asset_name: 'dataset.csv.zst',
              asset_size: 1024 * 200,
              shard_count: 4,
              created_at: Math.floor(Date.now() / 1000) - 3600,
            },
          ],
        },
      ],
    ]),
  ],
};

export const Loading: Story = {
  decorators: [withProviders([])],
  parameters: {
    docs: {
      description: {
        story: 'No data seeded — the component renders its skeleton state.',
      },
    },
  },
};
