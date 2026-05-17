// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { MessengerPanel } from './MessengerPanel';

const meta = {
  title: 'Organisms/MessengerPanel',
  component: MessengerPanel,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof MessengerPanel>;

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
      [['blockmatrix', 'message', 'inbox'], { messages: [], count: 0 }],
    ]),
  ],
};

export const WithConversations: Story = {
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'message', 'inbox'],
        {
          count: 3,
          messages: [
            {
              message_id: 'm-1',
              sender_node_id: 'peer-alpha',
              sender_name: 'alpha.private',
              recipient_node_id: 'self',
              body: 'pulling shard map for video.mp4',
              content_type: 'text/plain',
              created_at: Math.floor(Date.now() / 1000) - 60,
            },
            {
              message_id: 'm-2',
              sender_node_id: 'peer-beta',
              recipient_node_id: 'self',
              body: 'block 8421 propagated, thanks',
              content_type: 'text/plain',
              created_at: Math.floor(Date.now() / 1000) - 300,
            },
            {
              message_id: 'm-3',
              sender_node_id: 'self',
              recipient_node_id: 'peer-gamma',
              body: 'queued you for next reflector rotation',
              content_type: 'text/plain',
              created_at: Math.floor(Date.now() / 1000) - 900,
            },
          ],
        },
      ],
    ]),
  ],
};

export const Loading: Story = {
  decorators: [withProviders([])],
};
