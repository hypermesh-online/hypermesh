// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { Activity, Cpu } from 'lucide-react';
import { MetricCard } from './MetricCard';
import { Skeleton } from './skeleton';

const meta = {
  title: 'Molecules/MetricCard',
  component: MetricCard,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Themed metric tile with value, optional subtitle, trend, status dot, and inline progress bar. Loading/empty/error states are expressed via the wrapping skeleton, value="—", or theme="red".',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof MetricCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    title: 'CPU Usage',
    value: '47%',
    subtitle: 'across 8 cores',
    icon: Cpu,
    status: 'good',
    theme: 'cyan',
    progress: { value: 47, showPercentage: true },
  },
};

export const WithTrend: Story = {
  args: {
    title: 'Throughput',
    value: '2.95 Gbps',
    subtitle: 'last 5 min average',
    icon: Activity,
    trend: { value: '+12.4% vs prior window', direction: 'up' },
    theme: 'green',
    status: 'excellent',
  },
};

export const Loading: Story = {
  render: () => (
    <div className="w-72 rounded-lg border border-cyan-500/30 bg-black/40 backdrop-blur-lg p-4 space-y-3">
      <Skeleton className="h-4 w-1/2" />
      <Skeleton className="h-8 w-2/3" />
      <Skeleton className="h-2 w-full" />
    </div>
  ),
  args: { title: '', value: '' },
};

export const Empty: Story = {
  args: {
    title: 'Pending Rewards',
    value: '—',
    subtitle: 'no data yet',
    theme: 'cyan',
  },
};

export const Error: Story = {
  args: {
    title: 'Daemon Status',
    value: 'offline',
    subtitle: 'unable to reach BlockMatrix IPC',
    theme: 'red',
    status: 'critical',
  },
};
