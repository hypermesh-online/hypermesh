// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { ActivityItem } from './ActivityItem';

const meta = {
  title: 'Molecules/ActivityItem',
  component: ActivityItem,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Single row in an activity / audit log. Color of the leading dot reflects event type; theme drives the row background tint.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof ActivityItem>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Info: Story = {
  args: {
    type: 'info',
    message: 'BlockMatrix daemon connected',
    time: 'Just now',
    theme: 'cyan',
  },
};

export const Success: Story = {
  args: {
    type: 'success',
    message: 'Block 8421 propagated to 5 peers',
    details: 'fanout=3, latency 42ms',
    time: '2m ago',
    theme: 'green',
  },
};

export const Warning: Story = {
  args: {
    type: 'warning',
    message: 'Shard 3 below replication threshold',
    details: '7-of-10 available, repair queued',
    time: '6m ago',
    theme: 'yellow',
  },
};

export const Error: Story = {
  args: {
    type: 'error',
    message: 'PoS handshake failed for peer x:y:z',
    details: 'FALCON signature mismatch',
    time: '11m ago',
    theme: 'red',
  },
};

export const Stream: Story = {
  render: () => (
    <div className="w-[480px] space-y-2">
      <ActivityItem type="success" message="Peer joined" time="just now" theme="cyan" />
      <ActivityItem type="info" message="Sync poll" time="5s ago" theme="cyan" />
      <ActivityItem type="warning" message="Slow neighbor" time="30s ago" theme="yellow" />
      <ActivityItem type="error" message="PoS reject" time="1m ago" theme="red" />
    </div>
  ),
  args: { message: '', time: '' },
};
