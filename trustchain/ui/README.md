# Web3 Ecosystem UI

A modern, quantum-secure web interface for the Web3 ecosystem infrastructure, built with Svelte 5, ShadCN components, and Routify routing.

## 🌟 Features

### 🔐 **Quantum-Secure Infrastructure Monitoring**
- **TrustChain CA**: Certificate authority with FALCON-1024 post-quantum cryptography
- **STOQ Protocol**: High-performance transport layer with QUIC optimization
- **HyperMesh Network**: Decentralized asset management with NAT-like proxy addressing
- **Caesar Economics**: Token-based economic incentives for resource sharing
- **Four-Proof Consensus**: NKrypt protocol with unified WHERE/WHO/WHAT/WHEN validation

### 🎨 **Modern UI/UX**
- **Svelte 5**: Latest framework with runes for reactive state management
- **ShadCN/Svelte**: Professional, accessible component library
- **Tailwind CSS**: Utility-first styling with custom Web3 color palette
- **Dark/Light Mode**: System preference detection with manual toggle
- **Responsive Design**: Mobile-first approach with adaptive layouts
- **Real-time Updates**: WebSocket connections for live data streams

### 📊 **Comprehensive Dashboards**
- **System Overview**: Real-time status of all ecosystem components
- **Performance Metrics**: Throughput, latency, and utilization monitoring
- **Asset Management**: Resource allocation and privacy configuration
- **Certificate Management**: PKI operations and lifecycle management
- **Economic Tracking**: Rewards, staking, and token distribution
- **Consensus Monitoring**: Block production and validator performance

## 🚀 Quick Start

### Prerequisites
- Node.js 18+ 
- npm or yarn
- Running Web3 ecosystem services (TrustChain, STOQ, HyperMesh, Caesar)

### Installation
```bash
cd /home/persist/repos/projects/web3/trustchain/ui
npm install
```

### Development
```bash
npm run dev
```
The application will be available at `http://localhost:1337` with IPv6 support.

### Production Build
```bash
npm run build
npm run preview
```

## 🏗️ Architecture

### Technology Stack
- **Framework**: Svelte 5 with TypeScript
- **Routing**: Routify 3 (file-based routing)
- **Styling**: Tailwind CSS + ShadCN design system
- **State Management**: Svelte stores with derived state
- **API Client**: Custom REST + WebSocket client
- **Build Tool**: Vite with modern optimizations

### Project Structure
```
src/
├── lib/
│   ├── components/
│   │   ├── ui/           # ShadCN base components
│   │   ├── web3/         # Web3-specific components
│   │   ├── charts/       # Data visualization
│   │   └── icons/        # Custom icons
│   ├── stores/           # Global state management
│   ├── api/             # API client and WebSocket manager
│   ├── types.ts         # TypeScript definitions
│   └── utils.ts         # Utility functions
├── routes/              # File-based routing (Routify)
│   ├── index.svelte     # Dashboard home
│   ├── trustchain.svelte # Certificate authority
│   ├── stoq.svelte      # Protocol monitoring
│   ├── hypermesh.svelte # Asset management
│   ├── caesar.svelte    # Economics platform
│   ├── consensus.svelte # Consensus monitoring
│   └── settings.svelte  # System configuration
└── app.css             # Global styles and themes
```

## 🔧 Configuration

### API Endpoints
The UI connects to IPv6 endpoints for all services:
```typescript
const API_BASE_URLS = {
  trustchain: 'https://[2001:db8::1]:8443',
  stoq: 'https://[2001:db8::2]:8444', 
  hypermesh: 'https://[2001:db8::3]:8445',
  caesar: 'https://[2001:db8::4]:8446',
  consensus: 'https://[2001:db8::5]:8447'
};
```

### WebSocket Connections
Real-time updates via WebSocket connections:
```typescript
const WS_BASE_URLS = {
  trustchain: 'wss://[2001:db8::1]:8443/ws',
  stoq: 'wss://[2001:db8::2]:8444/ws',
  // ... other services
};
```

### Environment Variables
Create `.env` file for configuration:
```env
VITE_API_BASE_URL=https://[::1]:8443
VITE_WS_BASE_URL=wss://[::1]:8443/ws
VITE_DEVELOPMENT_MODE=true
VITE_AUTO_REFRESH_INTERVAL=30000
```

## 🎯 Component Features

### System Status Cards
- Real-time health monitoring
- Uptime tracking
- Performance metrics
- Color-coded status indicators

### Asset Management
- Resource allocation visualization
- Privacy level configuration
- Consensus proof requirements
- Utilization monitoring

### Certificate Operations
- FALCON-1024 quantum-safe certificates
- Issuance and revocation
- Fingerprint verification
- Extension management

### Performance Monitoring
- STOQ protocol throughput (target: 40 Gbps)
- QUIC optimization status
- Network latency metrics
- Quantum-safe connection tracking

### Economic Dashboard
- CAESAR token balance
- Staking rewards and APY
- Asset-based earnings
- Network value tracking

### Consensus Validation
- Four-proof system monitoring
- Block production metrics
- Validator performance
- Byzantine fault tolerance

## 🔐 Security Features

### Post-Quantum Cryptography
- **FALCON-1024**: Digital signatures
- **Kyber-1024**: Key encapsulation
- **SHAKE-256**: Hash functions
- Certificate transparency and validation

### Privacy Controls
- Configurable privacy levels (Private → Full Public)
- Anonymous connection options
- Data retention policies
- Usage statistics control

### Network Security
- IPv6-only networking
- TLS 1.3 with quantum-safe extensions
- OCSP stapling
- HSTS enforcement

## 📈 Performance Optimization

### Frontend Optimization
- Code splitting and lazy loading
- Tree shaking for minimal bundles
- Service worker for offline capability
- CDN-ready static assets

### Real-time Updates
- Efficient WebSocket reconnection
- Debounced state updates
- Memory-efficient event handling
- Background sync capabilities

### Data Management
- Optimistic UI updates
- Client-side caching
- Pagination and filtering
- Progressive data loading

## 🧪 Development

### Component Development
```bash
# Add new ShadCN component
npx shadcn-svelte@latest add button

# Create Web3-specific component
touch src/lib/components/web3/NewComponent.svelte
```

### State Management
```typescript
// Create new store
export const newStore = writable(initialState);

// Add derived state
export const derivedData = derived(baseStore, ($base) => transform($base));

// Create actions
export const actions = {
  updateState: (data) => newStore.update(state => ({ ...state, ...data }))
};
```

### API Integration
```typescript
// Add new API endpoint
async getNewData(): Promise<ApiResponse<NewDataType>> {
  return this.request('service', '/api/v1/new-endpoint');
}

// Add WebSocket handler
wsManager.connect('service', (data) => {
  if (data.type === 'new_event') {
    // Handle real-time update
  }
});
```

## 🚀 Deployment

### Production Build
```bash
npm run build
```

### Static Hosting
The application builds to static files and can be hosted on:
- Cloudflare Pages
- Vercel
- Netlify
- GitHub Pages
- Any static web server

### IPv6 Configuration
Ensure your hosting platform supports IPv6 for connecting to the Web3 ecosystem services.

## 🔍 Monitoring & Debugging

### Debug Mode
Set `debug-screens` class on body element for Tailwind debug information.

### Performance Monitoring
- Use browser DevTools for performance profiling
- Monitor WebSocket connection health
- Track API response times
- Analyze bundle size and loading performance

### Error Handling
- Centralized error boundary
- API error notifications
- WebSocket reconnection logic
- Graceful degradation for offline mode

## 🤝 Contributing

### Development Workflow
1. Fork and clone the repository
2. Install dependencies: `npm install`
3. Start development server: `npm run dev`
4. Make changes and test thoroughly
5. Build production: `npm run build`
6. Submit pull request

### Code Style
- TypeScript for type safety
- ESLint and Prettier for code formatting
- Svelte best practices with runes
- Accessible component design

### Testing
```bash
# Type checking
npm run check

# Linting
npm run lint

# Format code
npm run format
```

## 📄 License

This project is part of the Web3 Ecosystem infrastructure. See the main repository for licensing information.

## 🆘 Support

For technical support:
1. Check the development console for errors
2. Verify API service connectivity
3. Review WebSocket connection status
4. Consult the main Web3 ecosystem documentation

---

**Built with ❤️ for the quantum-secure, user-sovereign internet**