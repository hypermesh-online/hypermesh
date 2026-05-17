// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { NetworkStatus } from './NetworkStatus';

const meta = {
  title: 'Molecules/NetworkStatus',
  component: NetworkStatus,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Compact pill showing the connection state of a named network. Animated pulse on the dot for connected/connecting states.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof NetworkStatus>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Connected: Story = {
  args: { name: 'trust.hypermesh.online', status: 'connected', details: '12 peers' },
};

export const Connecting: Story = {
  args: { name: 'family.private', status: 'connecting', details: 'handshake' },
};

export const Disconnected: Story = {
  args: { name: 'lab.private', status: 'disconnected' },
};

export const ErrorState: Story = {
  args: { name: 'public', status: 'error', details: 'PoS rejected' },
};
