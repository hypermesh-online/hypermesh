# HyperMesh UI

Dashboard for monitoring and managing the HyperMesh ecosystem. Built with React, TypeScript, and Vite.

**Status**: In Development (44% complete) | 272 files | ~43,300 lines | 11 tests

## Tech Stack

- **Framework**: React 18 + TypeScript
- **Build**: Vite
- **Routing**: React Router
- **Styling**: Tailwind CSS v4
- **Components**: Radix UI primitives, Lucide React icons
- **Data**: TanStack React Query
- **Charts**: Custom SVG-based visualizations (Line, Bar, Pie, Area, Gauge, Sparkline, Topology, Network)
- **Testing**: Vitest (unit) + Playwright (E2E)
- **Documentation**: Storybook (5 stories)

## Features

### Working
- Dashboard home with ecosystem overview
- Module pages: STOQ, TrustChain, Catalog, Caesar, Engauge, HyperMesh
- TrustChain management (certificates, security, Proof of State, node config)
- Asset management views (creation wizard, advanced management)
- Proxy and NAT address management
- API status monitoring and performance dashboards
- Global search across ecosystem modules
- Accessibility: keyboard navigation, ARIA labels, focus management, high contrast, reduced motion
- React Router page routing with sidebar navigation

### In Development
- E2E test suite (Playwright)
- Component unit tests (Vitest)
- STOQ native demo (WebAssembly integration)
- Integration test harness page

### Planned
- Live STOQ WebSocket data connections
- Real-time metrics streaming
- Multi-node cluster topology visualization
- Caesar wallet and transaction management UI
- Engauge analytics and reward distribution dashboard
- Native desktop dashboard (Tauri: Linux/macOS/Windows)
- First-run onboarding flow (sovereign node setup, network join)

## Development

```bash
cd ui/frontend

# Install dependencies
npm install

# Start development server
npm run dev

# Run unit tests
npm run test

# Run E2E tests
npm run test:e2e

# Build for production
npm run build

# Start Storybook
npm run storybook
```

## Project Structure

```
ui/frontend/
    |- components/
    |   |- ui/           # Reusable primitives (Radix-based)
    |   |- modules/      # Feature pages (TrustChain, STOQ, Caesar, etc.)
    |   '- charts/       # Data visualization components
    |- src/
    |   '- test/         # Unit test files
    |- tests/
    |   '- e2e/          # Playwright E2E specs
    |- App.tsx           # Root component with routing
    |- main.tsx          # Entry point
    '- index.css         # Tailwind styles
```

## License

Business Source License 1.1
