// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { Database, RefreshCcw, Download } from 'lucide-react';
import { DataCard } from './DataCard';
import { Skeleton } from './skeleton';

const meta = {
  title: 'Molecules/DataCard',
  component: DataCard,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Themed card with title, description, icon, optional action buttons and badge. Body is `children` — drop any content inside.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof DataCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    title: 'Asset Inventory',
    description: '14 registered assets, 12 healthy',
    icon: Database,
    theme: 'cyan',
    children: (
      <ul className="text-sm text-gray-300 space-y-1">
        <li>asset-a01: container</li>
        <li>asset-b02: storage shard</li>
        <li>asset-c03: dns record</li>
      </ul>
    ),
  },
};

export const WithActions: Story = {
  args: {
    title: 'Topology Map',
    description: 'Last refreshed 30s ago',
    icon: Database,
    theme: 'purple',
    badge: { text: 'live', variant: 'default' },
    actions: [
      { label: 'Refresh', icon: RefreshCcw, onClick: () => {}, variant: 'outline' },
      { label: 'Export', icon: Download, onClick: () => {}, variant: 'outline' },
    ],
    children: (
      <div className="text-sm text-gray-400">3 reflectors / 7 peers / 2 gateways</div>
    ),
  },
};

export const Loading: Story = {
  args: {
    title: 'Asset Inventory',
    icon: Database,
    theme: 'cyan',
    children: (
      <div className="space-y-2">
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-5/6" />
        <Skeleton className="h-4 w-2/3" />
      </div>
    ),
  },
};

export const Empty: Story = {
  args: {
    title: 'Asset Inventory',
    description: 'No assets registered yet',
    icon: Database,
    theme: 'cyan',
    children: (
      <div className="py-6 text-center text-sm text-gray-500">
        Register an asset to see it here.
      </div>
    ),
  },
};
