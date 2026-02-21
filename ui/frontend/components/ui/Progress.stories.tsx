// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { Progress } from './progress';

const meta = {
  title: 'UI Components/Progress',
  component: Progress,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component: 'A progress bar component for showing completion status or loading states.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    value: {
      control: { type: 'range', min: 0, max: 100, step: 1 },
      description: 'Progress value (0-100)',
    },
    className: {
      control: 'text',
      description: 'Additional CSS classes',
    },
  },
} satisfies Meta<typeof Progress>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    value: 60,
  },
  render: (args) => (
    <div className="w-96">
      <Progress {...args} />
    </div>
  ),
};

export const DifferentValues: Story = {
  render: () => (
    <div className="w-96 space-y-4">
      <div>
        <p className="text-sm text-gray-400 mb-2">25% Complete</p>
        <Progress value={25} />
      </div>
      <div>
        <p className="text-sm text-gray-400 mb-2">50% Complete</p>
        <Progress value={50} />
      </div>
      <div>
        <p className="text-sm text-gray-400 mb-2">75% Complete</p>
        <Progress value={75} />
      </div>
      <div>
        <p className="text-sm text-gray-400 mb-2">100% Complete</p>
        <Progress value={100} />
      </div>
    </div>
  ),
};

export const Themed: Story = {
  render: () => (
    <div className="w-96 space-y-4">
      <div>
        <p className="text-sm text-cyan-400 mb-2">Cyan Theme</p>
        <Progress value={70} className="[&>div]:bg-cyan-400" />
      </div>
      <div>
        <p className="text-sm text-green-400 mb-2">Green Theme</p>
        <Progress value={85} className="[&>div]:bg-green-400" />
      </div>
      <div>
        <p className="text-sm text-purple-400 mb-2">Purple Theme</p>
        <Progress value={45} className="[&>div]:bg-purple-400" />
      </div>
      <div>
        <p className="text-sm text-red-400 mb-2">Red Theme</p>
        <Progress value={30} className="[&>div]:bg-red-400" />
      </div>
    </div>
  ),
};

export const WithLabels: Story = {
  render: () => (
    <div className="w-96 space-y-6">
      <div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-white">Download Progress</span>
          <span className="text-cyan-400">342MB / 500MB</span>
        </div>
        <Progress value={68.4} />
      </div>
      <div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-white">System Uptime</span>
          <span className="text-green-400">99.9%</span>
        </div>
        <Progress value={99.9} className="[&>div]:bg-green-400" />
      </div>
      <div>
        <div className="flex justify-between text-sm mb-2">
          <span className="text-white">Network Sync</span>
          <span className="text-yellow-400">Syncing...</span>
        </div>
        <Progress value={23} className="[&>div]:bg-yellow-400" />
      </div>
    </div>
  ),
};
