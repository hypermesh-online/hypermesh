// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { Badge } from './badge';

const meta = {
  title: 'UI Components/Badge',
  component: Badge,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component: 'A small component for displaying status indicators, labels, or notifications with various visual styles.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    variant: {
      control: 'select',
      options: ['default', 'secondary', 'destructive', 'outline'],
      description: 'Visual style variant of the badge',
    },
    children: {
      control: 'text',
      description: 'Badge content',
    },
  },
} satisfies Meta<typeof Badge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    children: 'Badge',
  },
};

export const Variants: Story = {
  render: () => (
    <div className="flex flex-wrap gap-4">
      <Badge variant="default">Default</Badge>
      <Badge variant="secondary">Secondary</Badge>
      <Badge variant="destructive">Destructive</Badge>
      <Badge variant="outline">Outline</Badge>
    </div>
  ),
};

export const StatusIndicators: Story = {
  render: () => (
    <div className="flex flex-wrap gap-4">
      <Badge className="bg-green-500/20 text-green-400 border-green-500/30">Active</Badge>
      <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">Pending</Badge>
      <Badge className="bg-red-500/20 text-red-400 border-red-500/30">Error</Badge>
      <Badge className="bg-blue-500/20 text-blue-400 border-blue-500/30">Processing</Badge>
      <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30">Connected</Badge>
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story: 'Status badges with custom colors for different states.',
      },
    },
  },
};

export const WithDots: Story = {
  render: () => (
    <div className="flex flex-wrap gap-4">
      <Badge className="bg-green-500/20 text-green-400 border-green-500/30">
        <div className="w-2 h-2 bg-green-400 rounded-full mr-1" />
        Online
      </Badge>
      <Badge className="bg-red-500/20 text-red-400 border-red-500/30">
        <div className="w-2 h-2 bg-red-400 rounded-full mr-1" />
        Offline
      </Badge>
      <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
        <div className="w-2 h-2 bg-yellow-400 rounded-full mr-1 animate-pulse" />
        Connecting
      </Badge>
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story: 'Badges with status dots, including animated indicators.',
      },
    },
  },
};
