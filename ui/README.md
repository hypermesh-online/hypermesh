# HyperMesh Protocol Stack

A decentralized infrastructure application built on TrustChain identity and STOQ transport protocols.

## Features

- **TrustChain Identity**: Decentralized identity and trust foundation
- **STOQ Transport**: High-performance P2P transport protocol targeting adaptive network tiers (100 Mbps/1 Gbps/2.5 Gbps)
- **HyperMesh Network**: Multi-network resource management and distribution
- **Caesar Economy**: Anti-speculation currency with demurrage-based stability
- **Catalog Assets**: Asset management and creation platform
- **Ngauge Analytics**: Privacy-first analytics and user onboarding

## Accessibility

This application is built with accessibility in mind:

- **Keyboard Navigation**: Full keyboard navigation support with Tab and Arrow keys
- **Screen Reader Support**: Comprehensive ARIA labels and semantic markup
- **Focus Management**: Visual focus indicators and programmatic focus control
- **High Contrast**: Support for high contrast mode preferences
- **Reduced Motion**: Respects user's motion preferences
- **Color Contrast**: WCAG AA compliant color combinations

### Keyboard Shortcuts

- `Tab` / `Shift+Tab`: Navigate between interactive elements
- `Arrow Keys`: Navigate within component groups
- `Enter` / `Space`: Activate buttons and links
- `Escape`: Close modals and exit navigation modes

## Development

### Prerequisites

- Node.js 18+ 
- npm or yarn

### Getting Started

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Run Storybook
npm run storybook

# Build for production
npm run build
```

### Storybook Documentation

This project includes comprehensive Storybook documentation for all UI components:

```bash
# Run Storybook locally
npm run storybook

# Build Storybook for deployment
npm run build-storybook
```

The Storybook includes:

- **Component Library**: Interactive examples of all UI components
- **Accessibility Testing**: Built-in a11y addon for testing accessibility
- **Design Tokens**: Consistent theming and styling documentation
- **Usage Examples**: Real-world usage patterns and best practices
- **Props Documentation**: Auto-generated prop tables and descriptions

### Component Structure

```
frontend/components/
├── ui/                    # Reusable UI components
│   ├── *.stories.tsx     # Storybook documentation
│   ├── AccessibilityWrapper.tsx
│   ├── KeyboardNavigationProvider.tsx
│   ├── UserJourney.tsx
│   └── ...
├── modules/              # Feature modules
│   ├── TrustChainModule.tsx
│   ├── StoqModule.tsx
│   └── ...
└── charts/              # Data visualization components
    ├── AreaChart.tsx
    ├── LineChart.tsx
    └── ...
```

### User Journey System

The interactive user journey component guides users through the HyperMesh ecosystem:

1. **Identity Setup**: Establish TrustChain identity
2. **Network Access**: Connect to STOQ transport
3. **Resource Sharing**: Join HyperMesh network
4. **Economic Participation**: Access Caesar economy
5. **Asset Management**: Create and manage assets
6. **Analytics Mastery**: Access Ngauge insights

## Architecture

### Protocol Stack

1. **TrustChain**: Identity and trust layer
2. **STOQ**: High-performance transport over QUIC
3. **HyperMesh**: Multi-network interface
4. **Application Layer**: Caesar, Catalog, Ngauge

### Technology Stack

- **Frontend**: React, TypeScript, Vite
- **Styling**: Tailwind CSS v4
- **Components**: Radix UI, Lucide React
- **Documentation**: Storybook
- **Accessibility**: ARIA, Keyboard Navigation
- **Charts**: Custom SVG-based visualizations

## Contributing

1. Follow the established component patterns
2. Ensure all components have proper accessibility attributes
3. Add Storybook stories for new components
4. Test keyboard navigation and screen reader compatibility
5. Maintain WCAG AA compliance

## License

MIT License - see LICENSE file for details
