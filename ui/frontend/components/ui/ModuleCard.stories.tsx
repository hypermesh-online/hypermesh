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
    theme: {
      control: 'select',
      options: ['cyan', 'green', 'purple', 'red', 'yellow'],
      description: 'Color theme for the card',
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
    theme: 'cyan',
  },
};

export const WithProgress: Story = {
  args: {
    title: 'System Uptime',
    value: '99.9%',
    subtitle: 'All systems operational',
    icon: Shield,
    progress: 99.9,
    theme: 'green',
  },
};

export const StatusVariants: Story = {
  render: () => (
    <div className="grid grid-cols-2 gap-4 max-w-2xl">
      <ModuleCard
        title="Active Service"
        value="Running"
        subtitle="All systems operational"
        icon={Network}
        status="active"
        theme="green"
      />
      <ModuleCard
        title="Warning State"
        value="Degraded"
        subtitle="Performance issues detected"
        icon={Zap}
        status="warning"
        theme="yellow"
      />
      <ModuleCard
        title="Error State"
        value="Failed"
        subtitle="Connection lost"
        icon={Network}
        status="error"
        theme="red"
      />
      <ModuleCard
        title="Inactive Service"
        value="Stopped"
        subtitle="Service not running"
        icon={Zap}
        status="inactive"
        theme="cyan"
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
  render: () => (
    <div className="grid grid-cols-3 gap-4 max-w-3xl">
      {(['cyan', 'green', 'purple', 'red', 'yellow'] as const).map((theme) => (
        <ModuleCard
          key={theme}
          title={`${theme.charAt(0).toUpperCase() + theme.slice(1)} Theme`}
          value="42.5K"
          subtitle="Sample metric"
          icon={Shield}
          theme={theme}
          progress={75}
        />
      ))}
    </div>
  ),
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
    theme: 'cyan',
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
    theme: 'green',
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
