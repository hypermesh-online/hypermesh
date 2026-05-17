// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { Sidebar } from './Sidebar';

const meta = {
  title: 'Organisms/Sidebar',
  component: Sidebar,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof Sidebar>;

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
          <div className="bg-black min-h-screen flex">
            <Story />
          </div>
        </MemoryRouter>
      </QueryClientProvider>
    );
  };

export const Connected: Story = {
  decorators: [
    withProviders(
      [
        [
          ['blockmatrix', 'status'],
          {
            node_id: 'ab12cd34ef56',
            coordinate: { x: 0, y: 0, z: 0 },
            chain_height: 8421,
            privacy_mode: 'Anonymous',
            peers: 14,
            uptime_secs: 86_400,
          },
        ],
        [['blockmatrix', 'share', 'inbox'], { invites: [], count: 0 }],
        [['blockmatrix', 'message', 'inbox'], { messages: [], count: 0 }],
      ],
      '/hypermesh',
    ),
  ],
};

export const WithBadges: Story = {
  decorators: [
    withProviders(
      [
        [
          ['blockmatrix', 'status'],
          {
            node_id: 'ab12cd34ef56',
            coordinate: { x: 0, y: 0, z: 0 },
            chain_height: 8421,
            privacy_mode: 'Private',
            peers: 14,
            uptime_secs: 86_400,
          },
        ],
        [
          ['blockmatrix', 'share', 'inbox'],
          {
            invites: [
              {
                invite_id: 'inv-1',
                asset_id: 'a1',
                sender_node_id: 'peer-1',
                asset_name: 'video.mp4',
                asset_size: 1024 * 1024 * 200,
                shard_count: 14,
                created_at: Math.floor(Date.now() / 1000),
              },
            ],
            count: 1,
          },
        ],
        [['blockmatrix', 'message', 'inbox'], { messages: [], count: 12 }],
      ],
      '/',
    ),
  ],
};

export const Offline: Story = {
  decorators: [withProviders([], '/')],
  parameters: {
    docs: {
      description: {
        story:
          'No cached status — the sidebar shows "not connected" and "Offline" in the bottom rail.',
      },
    },
  },
};
