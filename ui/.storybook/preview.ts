// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { Preview } from '@storybook/react';
import '../frontend/index.css';

const preview: Preview = {
  parameters: {
    actions: { argTypesRegex: '^on[A-Z].*' },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
    backgrounds: {
      default: 'hypermesh-dark',
      values: [
        {
          name: 'hypermesh-dark',
          value: 'linear-gradient(135deg, #000000 0%, #1e293b 50%, #000000 100%)',
        },
        {
          name: 'black',
          value: '#000000',
        },
        {
          name: 'gray',
          value: '#1f2937',
        },
      ],
    },
    viewport: {
      viewports: {
        mobile: {
          name: 'Mobile',
          styles: { width: '375px', height: '667px' },
        },
        tablet: {
          name: 'Tablet',
          styles: { width: '768px', height: '1024px' },
        },
        desktop: {
          name: 'Desktop',
          styles: { width: '1440px', height: '900px' },
        },
        ultrawide: {
          name: 'Ultra Wide',
          styles: { width: '1920px', height: '1080px' },
        },
      },
    },
    docs: {
      theme: {
        base: 'dark',
        colorPrimary: '#22d3ee',
        colorSecondary: '#06b6d4',
        appBg: '#000000',
        appContentBg: '#1f2937',
        barBg: '#111827',
        inputBg: '#374151',
        inputBorder: '#4b5563',
        inputTextColor: '#f9fafb',
        textColor: '#f9fafb',
        textInverseColor: '#111827',
      },
    },
  },
  globalTypes: {
    theme: {
      description: 'Global theme for components',
      defaultValue: 'cyan',
      toolbar: {
        title: 'Theme',
        icon: 'paintbrush',
        items: [
          { value: 'cyan', title: 'Cyan', icon: 'circle' },
          { value: 'green', title: 'Green', icon: 'circle' },
          { value: 'purple', title: 'Purple', icon: 'circle' },
          { value: 'red', title: 'Red', icon: 'circle' },
          { value: 'yellow', title: 'Yellow', icon: 'circle' },
        ],
        dynamicTitle: true,
      },
    },
  },
};

export default preview;
