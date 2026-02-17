// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Crown, Package, FileText, Zap } from 'lucide-react';
import WalletConnection from './components/WalletConnection';
import RealAssetExchange from './components/RealAssetExchange';
import NetworkUtilityDashboard from './components/NetworkUtilityDashboard';
import EconomicActivityHistory from './components/EconomicActivityHistory';

function App() {
  return (
    <div className="min-h-screen bg-caesar-dark">
      {/* Header */}
      <header className="border-b border-gray-800 p-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Crown className="text-caesar-gold" size={32} />
            <div>
              <h1 className="text-2xl font-bold text-caesar-gold">Agora Exchange</h1>
              <p className="text-sm text-gray-400">Real Asset & Utility Transaction Platform</p>
            </div>
          </div>
          
          <nav className="hidden md:flex items-center gap-6">
            <a href="#exchange" className="flex items-center gap-2 text-gray-300 hover:text-white transition-colors">
              <Package size={20} />
              Exchange
            </a>
            <a href="#contracts" className="flex items-center gap-2 text-gray-300 hover:text-white transition-colors">
              <FileText size={20} />
              Contracts
            </a>
            <a href="#utility" className="flex items-center gap-2 text-gray-300 hover:text-white transition-colors">
              <Zap size={20} />
              Utility
            </a>
          </nav>
          
          <WalletConnection />
        </div>
      </header>

      {/* Anti-Speculation Banner */}
      <div className="bg-gradient-to-r from-yellow-900/50 to-orange-900/50 border-b border-yellow-700/50">
        <div className="max-w-7xl mx-auto p-4">
          <div className="flex items-center gap-3">
            <Zap className="text-yellow-400" size={24} />
            <div>
              <h2 className="font-semibold text-yellow-200">Anti-Speculative Utility Platform</h2>
              <p className="text-yellow-100 text-sm">
                Caesar tokens include demurrage (time decay) to encourage real economic activity over speculation. 
                Use tokens for actual services, contracts, and asset purchases.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto p-4">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Real Asset Exchange */}
          <div className="lg:col-span-1">
            <RealAssetExchange />
          </div>
          
          {/* Network Utility Dashboard */}
          <div className="lg:col-span-2">
            <NetworkUtilityDashboard />
          </div>
        </div>
        
        {/* Economic Activity History */}
        <div className="mt-6">
          <EconomicActivityHistory />
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-gray-800 mt-12 p-6">
        <div className="max-w-7xl mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
            <div>
              <h4 className="font-semibold text-caesar-gold mb-3">Agora Exchange</h4>
              <p className="text-gray-400 text-sm">
                Utility-focused platform for real-world asset transactions and professional services 
                using Caesar Token's anti-speculative design and Hypermesh integration.
              </p>
            </div>
            
            <div>
              <h4 className="font-semibold mb-3">Utility Services</h4>
              <ul className="space-y-2 text-sm text-gray-400">
                <li><a href="#" className="hover:text-white transition-colors">Service Payments</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Asset Purchases</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Work Contracts</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Fiat Conversion</a></li>
              </ul>
            </div>
            
            <div>
              <h4 className="font-semibold mb-3">Real Assets</h4>
              <ul className="space-y-2 text-sm text-gray-400">
                <li><a href="#" className="hover:text-white transition-colors">Professional Services</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Physical Goods</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Digital Assets</a></li>
                <li><a href="#" className="hover:text-white transition-colors">IP Licensing</a></li>
              </ul>
            </div>
            
            <div>
              <h4 className="font-semibold mb-3">Hypermesh Integration</h4>
              <ul className="space-y-2 text-sm text-gray-400">
                <li><a href="#" className="hover:text-white transition-colors">Tensor-Mesh Architecture</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Cross-Chain Assets</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Real-World Tracking</a></li>
                <li><a href="#" className="hover:text-white transition-colors">Contract Execution</a></li>
              </ul>
            </div>
          </div>
          
          <div className="border-t border-gray-800 mt-6 pt-6 text-center text-gray-400 text-sm">
            <p>&copy; 2025 Caesar Token Ecosystem. Anti-speculative utility platform with Hypermesh integration.</p>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;