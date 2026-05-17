// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { UpdateBanner } from './UpdateBanner';

const meta = {
  title: 'Organisms/UpdateBanner',
  component: UpdateBanner,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component:
          'Persistent update banner. Renders nothing when the daemon is up-to-date or the release feed is not configured. Stories seed `useSystemCheckUpdate` cache to surface each state.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof UpdateBanner>;

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
        <div className="bg-black min-h-screen">
          <Story />
        </div>
      </QueryClientProvider>
    );
  };

export const UpToDate: Story = {
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'system', 'check_update'],
        {
          up_to_date: true,
          current_version: '0.85.0',
          channel: 'stable',
        },
      ],
    ]),
  ],
  parameters: {
    docs: {
      description: { story: 'Banner stays silent when the node is up-to-date.' },
    },
  },
};

export const NotConfigured: Story = {
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'system', 'check_update'],
        {
          up_to_date: true,
          current_version: '0.85.0',
          channel: 'stable',
          note: 'release feed not configured',
        },
      ],
    ]),
  ],
  parameters: {
    docs: {
      description: {
        story: 'Alpha-default — release feed not configured, banner stays silent.',
      },
    },
  },
};

export const Available: Story = {
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'system', 'check_update'],
        {
          up_to_date: false,
          current_version: '0.85.0',
          available_version: '0.86.0',
          channel: 'stable',
          release_notes_url: 'https://example.test/releases/0.86.0',
        },
      ],
    ]),
  ],
};

export const BreakingChanges: Story = {
  decorators: [
    withProviders([
      [
        ['blockmatrix', 'system', 'check_update'],
        {
          up_to_date: false,
          current_version: '0.85.0',
          available_version: '0.90.0',
          channel: 'stable',
          breaking_changes: true,
          release_notes_url: 'https://example.test/releases/0.90.0',
          requires_min_version: '0.85.0',
        },
      ],
    ]),
  ],
};
