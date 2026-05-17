// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { CaesarOverview } from './CaesarOverview';

const meta = {
  title: 'Pages/CaesarOverview',
  component: CaesarOverview,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof CaesarOverview>;

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
        ['caesar', 'overview'],
        { balance: 0, locked: 0, pending_rewards: 0, total_staked: 0, recent_transactions: 0 },
      ],
      [
        ['caesar', 'balance'],
        { total: 0, available: 0, locked: 0, pending: 0, staked: 0 },
      ],
      [
        ['caesar', 'rewards'],
        { total_earned: 0, pending_rewards: 0, claimed_rewards: 0, daily_rate: 0, multiplier: 0 },
      ],
      [
        ['caesar', 'staking'],
        { total_staked: 0, available_to_stake: 0, total_rewards: 0, apy: 0, active_stakes: [] },
      ],
      [['caesar', 'transactions', 10], { transactions: [], total: 0 }],
    ]),
  ],
  parameters: {
    docs: {
      description: {
        story:
          'Caesar EVP not yet wired to live oracle — every metric is zero but the page renders normally.',
      },
    },
  },
};

export const WithBalance: Story = {
  decorators: [
    withProviders([
      [
        ['caesar', 'overview'],
        {
          balance: 1240.5,
          locked: 100,
          pending_rewards: 12.4,
          total_staked: 500,
          recent_transactions: 8,
        },
      ],
      [
        ['caesar', 'balance'],
        { total: 1740.5, available: 1140.5, locked: 100, pending: 0, staked: 500 },
      ],
      [
        ['caesar', 'rewards'],
        {
          total_earned: 124.6,
          pending_rewards: 12.4,
          claimed_rewards: 112.2,
          daily_rate: 1.8,
          multiplier: 1.0,
        },
      ],
      [
        ['caesar', 'staking'],
        { total_staked: 500, available_to_stake: 1140.5, total_rewards: 42.1, apy: 4.2, active_stakes: [] },
      ],
      [
        ['caesar', 'transactions', 10],
        {
          transactions: [
            {
              id: 't1',
              type: 'reward',
              from_wallet: 'caesar:treasury',
              to_wallet: 'self',
              amount: 1.8,
              fee: 0,
              status: 'settled',
              timestamp: Math.floor(Date.now() / 1000) - 3600,
            },
            {
              id: 't2',
              type: 'transfer',
              from_wallet: 'self',
              to_wallet: 'peer-alpha',
              amount: 12,
              fee: 0.1,
              status: 'settled',
              timestamp: Math.floor(Date.now() / 1000) - 7200,
            },
          ],
          total: 2,
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
        story:
          'No data seeded — both overview and balance queries report isLoading=true, so the page renders the `<ModuleLoading />` skeleton.',
      },
    },
  },
};
