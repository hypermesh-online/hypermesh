// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { GlobalSearch } from './GlobalSearch';

const meta = {
  title: 'Organisms/GlobalSearch',
  component: GlobalSearch,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Catalog typedef search popover. Reads `useCatalogSearch(query, true, 8)` — story seeds the React Query cache for the specific query/maxNeighbors triple. The popover is gated by `open && debounced.length >= 2`, so stories surface the popover via a pre-filled controlled prompt only when documenting result rendering; the closed state is the default.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof GlobalSearch>;

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

export const Closed: Story = {
  args: {},
  decorators: [withProviders([])],
  parameters: {
    docs: {
      description: {
        story: 'Default closed state — the input is empty and no popover is shown.',
      },
    },
  },
};

export const AlphaInert: Story = {
  args: {},
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'catalog', 'search', '', true, 8],
        {
          status: 'alpha',
          query: '',
          note: 'registry not wired (alpha)',
          matches: [],
          total: 0,
          neighbors_queried: 0,
          neighbor_errors: [],
        },
      ],
    ]),
  ],
  parameters: {
    docs: {
      description: {
        story:
          'The daemon reports `status: alpha` — when the user types, the popover surfaces the inert "registry not yet wired" banner.',
      },
    },
  },
};

export const WithResults: Story = {
  args: {},
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'catalog', 'search', '', true, 8],
        {
          status: 'ok',
          query: '',
          matches: [
            {
              type_hash: 'b3a1c4d5e6f7',
              name: 'video.codec.h265',
              version: '1.4.0',
              source: 'local',
            },
            {
              type_hash: 'd1e2f3a4b5c6',
              name: 'video.container.mp4',
              version: '2.0.1',
              source: 'neighbor:node-abcdef',
            },
          ],
          total: 2,
          neighbors_queried: 3,
          neighbor_errors: [{ node_id: 'node-slow01', error: 'timeout' }],
        },
      ],
    ]),
  ],
};
