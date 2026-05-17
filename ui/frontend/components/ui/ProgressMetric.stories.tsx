// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { ProgressMetric } from './ProgressMetric';

const meta = {
  title: 'Molecules/ProgressMetric',
  component: ProgressMetric,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Inline label + value + progress bar. Status (excellent/good/warning/critical) overrides theme color of the value.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof ProgressMetric>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: 'Storage Used',
    value: 42,
    unit: '%',
    description: '210 GB of 500 GB',
    theme: 'cyan',
  },
};

export const Warning: Story = {
  args: {
    label: 'Memory Pressure',
    value: 78,
    unit: '%',
    status: 'warning',
    description: 'High — consider rebalancing',
  },
};

export const Critical: Story = {
  args: {
    label: 'Shard Health',
    value: 92,
    unit: '%',
    status: 'critical',
    description: '8% of shards below k-of-n threshold',
  },
};

export const NonPercentMax: Story = {
  args: {
    label: 'Active Peers',
    value: 14,
    maxValue: 32,
    description: 'Out of recommended target',
    theme: 'green',
  },
};
