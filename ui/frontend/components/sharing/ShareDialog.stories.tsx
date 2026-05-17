// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ShareDialog } from './ShareDialog';

const meta = {
  title: 'Organisms/ShareDialog',
  component: ShareDialog,
  parameters: { layout: 'fullscreen' },
  tags: ['autodocs'],
} satisfies Meta<typeof ShareDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

function makeClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, refetchOnMount: false },
      mutations: { retry: false },
    },
  });
}

const withQuery = (Story: React.FC) => (
  <QueryClientProvider client={makeClient()}>
    <div className="bg-black min-h-screen">
      <Story />
    </div>
  </QueryClientProvider>
);

export const Open: Story = {
  args: {
    assetId: 'a-deadbeef-0001',
    assetName: 'capture-2026-05.mp4',
    isOpen: true,
    onClose: () => {},
  },
  decorators: [withQuery],
};

export const Closed: Story = {
  args: {
    assetId: 'a-deadbeef-0001',
    assetName: 'capture-2026-05.mp4',
    isOpen: false,
    onClose: () => {},
  },
  decorators: [withQuery],
  parameters: {
    docs: {
      description: { story: 'When `isOpen` is false the dialog returns null.' },
    },
  },
};
