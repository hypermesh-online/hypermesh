// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { UserJourney } from './UserJourney';

const meta = {
  title: 'UI Components/UserJourney',
  component: UserJourney,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component: 'An interactive user journey component that shows progress through the HyperMesh ecosystem, with accessibility features and keyboard navigation support.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    compact: {
      control: 'boolean',
      description: 'Show compact version suitable for sidebars',
    },
    showAchievements: {
      control: 'boolean',
      description: 'Display achievements section',
    },
    onStepSelect: {
      action: 'stepSelected',
      description: 'Callback when a step is selected',
    },
  },
} satisfies Meta<typeof UserJourney>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    compact: false,
    showAchievements: true,
  },
  render: (args) => (
    <div className="min-h-screen bg-gradient-to-br from-black via-slate-900 to-black p-6">
      <UserJourney {...args} />
    </div>
  ),
};

export const Compact: Story = {
  args: {
    compact: true,
    showAchievements: false,
  },
  render: (args) => (
    <div className="min-h-screen bg-gradient-to-br from-black via-slate-900 to-black p-6">
      <div className="max-w-md">
        <UserJourney {...args} />
      </div>
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story: 'Compact version suitable for sidebars or dashboard widgets.',
      },
    },
  },
};

export const NoAchievements: Story = {
  args: {
    compact: false,
    showAchievements: false,
  },
  render: (args) => (
    <div className="min-h-screen bg-gradient-to-br from-black via-slate-900 to-black p-6">
      <UserJourney {...args} />
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story: 'Journey component without achievements section.',
      },
    },
  },
};

export const Accessibility: Story = {
  args: {
    compact: false,
    showAchievements: true,
  },
  render: (args) => (
    <div className="min-h-screen bg-gradient-to-br from-black via-slate-900 to-black p-6">
      <div className="mb-4 p-4 bg-blue-500/10 border border-blue-500/30 rounded-lg text-blue-400">
        <h3 className="font-medium mb-2">Accessibility Features:</h3>
        <ul className="text-sm space-y-1">
          <li>• Use Tab or Arrow keys to navigate between steps</li>
          <li>• Press Enter to select and activate a step</li>
          <li>• Screen reader announcements for progress updates</li>
          <li>• Focus indicators for keyboard navigation</li>
          <li>• ARIA labels and roles for assistive technologies</li>
        </ul>
      </div>
      <UserJourney {...args} />
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story: 'Demonstrates accessibility features including keyboard navigation, focus management, and screen reader support.',
      },
    },
  },
};
