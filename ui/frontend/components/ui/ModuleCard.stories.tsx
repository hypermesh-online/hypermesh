// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { Shield, Network, Zap } from 'lucide-react';
import { ModuleCard } from './ModuleCard';

const meta = {
  title: 'UI Components/ModuleCard',
  component: ModuleCard,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component: 'A versatile card component for displaying module information, metrics, and status indicators. Used throughout the HyperMesh interface for system modules and network components.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    title: {
      control: 'text',
      description: 'The main title displayed on the card',
    },
    value: {
      control: 'text',
      description: 'The primary value or metric to display',
    },
    subtitle: {
      control: 'text',
      description: 'Additional context or description text',
    },
    icon: {
      control: false,
      description: 'Lucide React icon component to display',
    },
    status: {
      control: 'select',
      options: ['active', 'inactive', 'warning', 'error'],
      description: 'Status indicator affecting card styling',
    },
    progress: {
      control: { type: 'range', min: 0, max: 100, step: 1 },
      description: 'Progress percentage (0-100) to show progress bar',
    },
  },
} satisfies Meta<typeof ModuleCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    title: 'Node Identity',
    value: 'node-7f8a9b2c',
    subtitle: 'Cryptographically verified',
    icon: Shield,
    iconColor: 'text-cyan-400',
  },
};

export const WithProgress: Story = {
  args: {
    title: 'System Uptime',
    value: '99.9%',
    subtitle: 'All systems operational',
    icon: Shield,
    progress: 99.9,
    iconColor: 'text-green-400',
  },
};

export const StatusVariants: Story = {
  args: {
    title: 'Status Variants',
    value: 'Running',
    icon: Network,
  },
  render: () => (
    <div className="grid grid-cols-2 gap-4 max-w-2xl">
      <ModuleCard
        title="Active Service"
        value="Running"
        subtitle="All systems operational"
        icon={Network}
        status="active"
        iconColor="text-green-400"
      />
      <ModuleCard
        title="Warning State"
        value="Degraded"
        subtitle="Performance issues detected"
        icon={Zap}
        status="warning"
        iconColor="text-yellow-400"
      />
      <ModuleCard
        title="Error State"
        value="Failed"
        subtitle="Connection lost"
        icon={Network}
        status="error"
        iconColor="text-red-400"
      />
      <ModuleCard
        title="Inactive Service"
        value="Stopped"
        subtitle="Service not running"
        icon={Zap}
        status="inactive"
        iconColor="text-cyan-400"
      />
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story: 'Different status variants showing various states a module can be in.',
      },
    },
  },
};

export const ThemeVariants: Story = {
  args: {
    title: 'Theme Variants',
    value: '42.5K',
    icon: Shield,
  },
  render: () => {
    const themes = [
      { key: 'cyan', color: 'text-cyan-400' },
      { key: 'green', color: 'text-green-400' },
      { key: 'purple', color: 'text-purple-400' },
      { key: 'red', color: 'text-red-400' },
      { key: 'yellow', color: 'text-yellow-400' },
    ] as const;
    return (
      <div className="grid grid-cols-3 gap-4 max-w-3xl">
        {themes.map((theme) => (
          <ModuleCard
            key={theme.key}
            title={`${theme.key.charAt(0).toUpperCase() + theme.key.slice(1)} Theme`}
            value="42.5K"
            subtitle="Sample metric"
            icon={Shield}
            iconColor={theme.color}
            progress={75}
          />
        ))}
      </div>
    );
  },
  parameters: {
    docs: {
      description: {
        story: 'All available theme variants for the ModuleCard component.',
      },
    },
  },
};

export const Interactive: Story = {
  args: {
    title: 'Interactive Card',
    value: '1,247',
    subtitle: 'Active connections',
    icon: Network,
    iconColor: 'text-cyan-400',
    progress: 85,
  },
  parameters: {
    docs: {
      description: {
        story: 'Hover over the card to see interactive effects.',
      },
    },
  },
};

export const Accessibility: Story = {
  args: {
    title: 'Accessible Card',
    value: '99.9%',
    subtitle: 'Uptime monitoring',
    icon: Shield,
    iconColor: 'text-green-400',
    progress: 99.9,
  },
  parameters: {
    docs: {
      description: {
        story: 'This card includes proper ARIA labels and keyboard navigation support.',
      },
    },
  },
};
