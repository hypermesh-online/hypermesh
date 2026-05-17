// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { MemoryRouter } from 'react-router-dom';
import { ModuleConnections } from './ModuleConnections';

const meta = {
  title: 'Organisms/ModuleConnections',
  component: ModuleConnections,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Lists the other HyperMesh modules with their dependency/enable state relative to `currentModule`. Embeds react-router Links, so a router is required.',
      },
    },
  },
  decorators: [
    (Story) => (
      <MemoryRouter>
        <div className="max-w-2xl">
          <Story />
        </div>
      </MemoryRouter>
    ),
  ],
  tags: ['autodocs'],
} satisfies Meta<typeof ModuleConnections>;

export default meta;
type Story = StoryObj<typeof meta>;

export const FromTrustChain: Story = {
  args: { currentModule: 'trustchain', theme: 'green' },
};

export const FromHyperMesh: Story = {
  args: { currentModule: 'hypermesh', theme: 'cyan' },
};

export const FromCaesar: Story = {
  args: { currentModule: 'caesar', theme: 'yellow' },
};
