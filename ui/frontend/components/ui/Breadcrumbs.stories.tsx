// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { MemoryRouter } from 'react-router-dom';
import { Home, Shield } from 'lucide-react';
import { Breadcrumbs } from './Breadcrumbs';

const meta = {
  title: 'Molecules/Breadcrumbs',
  component: Breadcrumbs,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Breadcrumb navigation. When `items` is omitted, the component derives the trail from `useLocation()`. A MemoryRouter wrapper is required in stories.',
      },
    },
  },
  tags: ['autodocs'],
} satisfies Meta<typeof Breadcrumbs>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Explicit: Story = {
  args: {
    items: [
      { label: 'Dashboard', href: '/', icon: Home },
      { label: 'TrustChain', href: '/trustchain', icon: Shield },
      { label: 'Certificates', href: '/trustchain/certificates' },
    ],
    theme: 'green',
  },
  decorators: [
    (Story) => (
      <MemoryRouter initialEntries={['/trustchain/certificates']}>
        <Story />
      </MemoryRouter>
    ),
  ],
};

export const AutoFromPath: Story = {
  args: { theme: 'cyan' },
  decorators: [
    (Story) => (
      <MemoryRouter initialEntries={['/hypermesh/management']}>
        <Story />
      </MemoryRouter>
    ),
  ],
};

export const RootOnly: Story = {
  args: { theme: 'cyan' },
  decorators: [
    (Story) => (
      <MemoryRouter initialEntries={['/']}>
        <Story />
      </MemoryRouter>
    ),
  ],
  parameters: {
    docs: {
      description: { story: 'At the dashboard root, the component renders nothing.' },
    },
  },
};
