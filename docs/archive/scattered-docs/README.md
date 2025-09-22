# Caesar Token Scrolls App

Complete DeFi ecosystem for Caesar Token featuring professional trading, multi-chain wallet, and real-time analytics.

## 🏛️ Applications

### Agora DEX (Port 3001)
Professional trading platform for Caesar Token with:
- Real-time price charts and market data
- Advanced trading interface with slippage control
- Trade history and order book
- LayerZero V2 cross-chain integration
- Demurrage-aware calculations

**Features:**
- ✅ Professional trading UI
- ✅ Real-time price updates
- ✅ Wallet integration with MetaMask
- ✅ Sepolia testnet support
- ✅ Trading fee calculations
- ✅ Responsive design

### Satchel Wallet (Port 3002)
Multi-chain wallet interface supporting:
- Multiple blockchain networks
- Token portfolio management
- Transaction history
- QR code generation for addresses
- Hardware wallet support preparation

**Features:**
- ✅ Multi-chain network switching
- ✅ Token balance tracking
- ✅ Portfolio overview
- ✅ Transaction history
- ✅ Security features
- ✅ Mobile-responsive design

### Tablets UI (Port 3003)
Comprehensive analytics dashboard with:
- Real-time token metrics
- Demurrage tracking and visualization
- Liquidity pool analytics
- Cross-chain activity monitoring
- Yield farming statistics

**Features:**
- ✅ Real-time metrics dashboard
- ✅ Advanced demurrage analytics
- ✅ Liquidity pool tracking
- ✅ Cross-chain overview
- ✅ Interactive charts
- ✅ Yield farming data

## 🚀 Quick Start

### Prerequisites
- Node.js 18+ 
- MetaMask or compatible Web3 wallet
- Access to Sepolia testnet

### Installation & Development

```bash
# Install all dependencies
npm run install:all

# Start all applications
npm run dev

# Or start individual applications
npm run dev:agora    # Agora DEX on port 3001
npm run dev:satchel  # Satchel Wallet on port 3002
npm run dev:tablets  # Tablets UI on port 3003
```

### Accessing Applications

- **Agora DEX**: http://localhost:3001
- **Satchel Wallet**: http://localhost:3002  
- **Tablets UI**: http://localhost:3003

## 🔧 Technology Stack

### Frontend
- **React 18** with TypeScript
- **Vite** for fast development and building
- **TailwindCSS** for styling
- **Recharts** for data visualization
- **Lucide React** for icons

### Blockchain Integration
- **Ethers.js v6** for Web3 connectivity
- **LayerZero V2** for cross-chain functionality
- **MetaMask** wallet integration
- **Sepolia testnet** for development

### Architecture
- **Modular design** with shared components
- **Type-safe** development with TypeScript
- **Responsive design** for all screen sizes
- **Real-time updates** for live data

## 📋 Contract Integration

### Deployed Contracts (Sepolia)
```
Caesar Token: 0x6299744254422aadb6a57183f47eaae1678cf86cc58a0c78dfc4fd2caa3ba2a4
DEX Factory:  0xAe0DfF19f44D3544139d900a3f9f6c03C6764538
WETH:         0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9
```

### Supported Networks
- Ethereum Sepolia (Testnet) ✅
- Ethereum Mainnet (Planned)
- Polygon (Planned)
- Arbitrum (Planned)
- Base (Planned)

## 🎯 Key Features

### Trading (Agora DEX)
- Spot trading with real-time prices
- Advanced order types
- Slippage protection
- Trading history
- Market depth visualization

### Wallet (Satchel)
- Multi-chain asset management
- Transaction tracking
- Address QR codes
- Portfolio analytics
- Security features

### Analytics (Tablets)
- Token metrics dashboard
- Demurrage visualization
- Liquidity analytics
- Cross-chain tracking
- Yield farming data

## 🔒 Security Features

- **Client-side wallet integration** - Private keys never leave user's device
- **Network validation** - Automatic network switching
- **Transaction simulation** - Preview before execution
- **Slippage protection** - Configurable slippage tolerance
- **Rate limiting** - Protection against API abuse

## 🛠️ Development

### Project Structure
```
scrolls-app/
├── agora-dex/          # Trading interface
├── satchel-wallet/     # Multi-chain wallet
├── tablets-ui/         # Analytics dashboard
├── shared/             # Shared components and utilities
├── package.json        # Root package configuration
└── README.md          # This file
```

### Build for Production
```bash
npm run build
```

### Environment Variables
Each application supports environment variables for:
- RPC endpoints
- API keys
- Contract addresses
- Network configurations

## 📊 Metrics & Monitoring

- **Real-time price feeds** from multiple sources
- **Transaction monitoring** across all chains
- **Liquidity tracking** for all pools
- **Demurrage calculations** with precision
- **Cross-chain synchronization** status

## 🌐 Cross-Chain Features

- **LayerZero V2 integration** for seamless bridging
- **Multi-chain asset tracking** in single interface
- **Unified transaction history** across all networks
- **Cross-chain yield opportunities** identification

## 🎨 Design System

- **Caesar Gold (#FFD700)** - Primary accent color
- **Dark theme** - Optimized for traders
- **Responsive grid** - Works on all devices
- **Accessibility** - WCAG compliant
- **Consistent components** - Shared design language

## 📝 License

MIT License - see LICENSE file for details

---

**🏛️ Built for the Caesar Token ecosystem - Where DeFi meets innovation**