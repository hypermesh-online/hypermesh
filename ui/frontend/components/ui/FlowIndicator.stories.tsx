// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { MemoryRouter } from 'react-router-dom';
import { FlowIndicator } from './FlowIndicator';

const meta = {
  title: 'Molecules/FlowIndicator',
  component: FlowIndicator,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'Step-by-step journey indicator with completed/current/upcoming/locked states. Renders react-router Links for accessible steps, so a router is required.',
      },
    },
  },
  decorators: [
    (Story) => (
      <MemoryRouter>
        <Story />
      </MemoryRouter>
    ),
  ],
  tags: ['autodocs'],
} satisfies Meta<typeof FlowIndicator>;

export default meta;
type Story = StoryObj<typeof meta>;

const onboardingSteps = [
  {
    id: 'identity',
    title: 'Identity',
    description: 'Generate FALCON-1024 keypair',
    href: '/trustchain/identity',
    status: 'completed' as const,
  },
  {
    id: 'transport',
    title: 'Transport',
    description: 'Bring up STOQ listener',
    href: '/stoq',
    status: 'completed' as const,
  },
  {
    id: 'peers',
    title: 'Peers',
    description: 'Join a network',
    href: '/hypermesh',
    status: 'current' as const,
  },
  {
    id: 'caesar',
    title: 'Caesar',
    description: 'Enable token rewards',
    href: '/caesar',
    status: 'upcoming' as const,
  },
  {
    id: 'catalog',
    title: 'Catalog',
    description: 'Publish a typedef',
    href: '/catalog',
    status: 'locked' as const,
    requirement: 'CAESAR participation',
  },
];

export const Horizontal: Story = {
  args: { steps: onboardingSteps, title: 'Node Onboarding', theme: 'cyan' },
};

export const Vertical: Story = {
  args: {
    steps: onboardingSteps,
    title: 'Node Onboarding',
    theme: 'green',
    orientation: 'vertical',
  },
};

export const Compact: Story = {
  args: {
    steps: onboardingSteps,
    title: 'Onboarding',
    theme: 'purple',
    compact: true,
  },
};
