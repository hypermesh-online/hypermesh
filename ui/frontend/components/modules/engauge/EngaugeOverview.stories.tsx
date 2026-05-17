// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import EngaugeOverview from './EngaugeOverview';

const meta = {
  title: 'Pages/EngaugeOverview',
  component: EngaugeOverview,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof EngaugeOverview>;

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
          <div className="bg-black min-h-screen">
            <Story />
          </div>
        </MemoryRouter>
      </QueryClientProvider>
    );
  };

export const AlphaInert: Story = {
  decorators: [
    withProviders([
      [
        ['engauge', 'capacity'],
        {
          cpu_usage: 0,
          memory_usage: 0,
          storage_usage: 0,
          network_usage: 0,
          total_capacity: 0,
        },
      ],
      [
        ['engauge', 'traffic'],
        { bytes_in: 0, bytes_out: 0, packets_in: 0, packets_out: 0 },
      ],
      [['engauge', 'throttle'], { active: false, current_rate: 0 }],
    ]),
  ],
  parameters: {
    docs: {
      description: {
        story: 'Engauge reporting enabled but no live samples yet — all metrics at zero.',
      },
    },
  },
};

export const WithMetrics: Story = {
  decorators: [
    withProviders([
      [
        ['engauge', 'capacity'],
        {
          cpu_usage: 0.47,
          memory_usage: 0.62,
          storage_usage: 0.31,
          network_usage: 0.18,
          total_capacity: 1.0,
        },
      ],
      [
        ['engauge', 'traffic'],
        {
          bytes_in: 1024 * 1024 * 240,
          bytes_out: 1024 * 1024 * 180,
          packets_in: 184_021,
          packets_out: 142_117,
        },
      ],
      [['engauge', 'throttle'], { active: false, current_rate: 0 }],
    ]),
  ],
};

export const Loading: Story = {
  decorators: [withProviders([])],
  parameters: {
    docs: {
      description: {
        story: 'No data seeded — page renders the `<ModuleLoading />` skeleton.',
      },
    },
  },
};
