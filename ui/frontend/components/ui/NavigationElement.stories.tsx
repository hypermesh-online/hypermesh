// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Meta, StoryObj } from '@storybook/react';
import { NavigationElement } from './NavigationElement';
import { KeyboardNavigationProvider } from './KeyboardNavigationProvider';

const meta = {
  title: 'Molecules/NavigationElement',
  component: NavigationElement,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Keyboard-navigable wrapper. Registers itself with KeyboardNavigationProvider, handles Enter/Space, and renders a focus ring under keyboard navigation.',
      },
    },
  },
  decorators: [
    (Story) => (
      <KeyboardNavigationProvider>
        <Story />
      </KeyboardNavigationProvider>
    ),
  ],
  tags: ['autodocs'],
} satisfies Meta<typeof NavigationElement>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    id: 'nav-default',
    order: 0,
    ariaLabel: 'Default item',
    children: (
      <div className="px-4 py-2 rounded-md bg-cyan-500/10 border border-cyan-500/30 text-cyan-200">
        Activatable item
      </div>
    ),
  },
};

export const Disabled: Story = {
  args: {
    id: 'nav-disabled',
    order: 1,
    disabled: true,
    ariaLabel: 'Disabled item',
    children: (
      <div className="px-4 py-2 rounded-md bg-gray-500/10 border border-gray-500/30 text-gray-400">
        Disabled item
      </div>
    ),
  },
};

export const Group: Story = {
  render: () => (
    <KeyboardNavigationProvider>
      <div className="flex flex-col gap-2 w-72">
        <NavigationElement id="g-1" order={0} ariaLabel="Item one">
          <div className="px-4 py-2 rounded-md bg-cyan-500/10 border border-cyan-500/30 text-cyan-200">
            Item 1
          </div>
        </NavigationElement>
        <NavigationElement id="g-2" order={1} ariaLabel="Item two">
          <div className="px-4 py-2 rounded-md bg-cyan-500/10 border border-cyan-500/30 text-cyan-200">
            Item 2
          </div>
        </NavigationElement>
        <NavigationElement id="g-3" order={2} disabled ariaLabel="Item three locked">
          <div className="px-4 py-2 rounded-md bg-gray-500/10 border border-gray-500/30 text-gray-400">
            Item 3 (locked)
          </div>
        </NavigationElement>
      </div>
    </KeyboardNavigationProvider>
  ),
  args: { id: 'unused', order: 0, children: null },
};
