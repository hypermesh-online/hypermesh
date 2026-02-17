// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useState, useEffect } from 'react';
import { 
  Wallet, CreditCard, Clock, TrendingUp, Settings, Server, Network, Zap,
  Activity, Shield, Globe, ChevronRight, Gauge, Cpu, Database, HardDrive,
  Eye, Users, ArrowUpRight, ArrowDownRight, Menu, X
} from 'lucide-react';
import { useAccount } from 'wagmi';
import WalletConnect from './components/WalletConnect';
import { WalletLoadingSkeleton, LoadingButton } from './components/LoadingStates';
import { ErrorCard, WalletError } from './components/ErrorStates';
import { SimpleAssetBrowser } from './components/SimpleAssetBrowser';
import MatrixChainInterface from './components/MatrixChainInterface';
import PerformanceComparison from './components/PerformanceComparison';
import { useToastContext } from './providers/ToastProvider';
import { useAsyncState } from './hooks/useAsyncState';
import { createCaesarUIFramework, EcosystemState } from './caesar/CaesarUIFramework';

interface WalletData {
  balance: string;
  assets: Array<{
    symbol: string;
    name: string;
    amount: string;
    value: string;
    change: string;
    logo: string;
  }>;
}

function App() {
  const [activeView, setActiveView] = useState<'dashboard' | 'assets' | 'matrix' | 'performance' | 'network'>('dashboard');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [caesarFramework] = useState(() => createCaesarUIFramework());
  const [ecosystemState, setEcosystemState] = useState<EcosystemState | null>(null);
  const [, setAllocatedAssets] = useState<any[]>([]);
  const { address, isConnected } = useAccount();
  const { showSuccess, showError, showInfo } = useToastContext();
  const { data: walletData, loading: walletLoading, error: walletError, execute: fetchWalletData } = useAsyncState<WalletData>();

  // Simulate wallet data fetching
  const loadWalletData = async (): Promise<WalletData> => {
    await new Promise(resolve => setTimeout(resolve, 2000)); // Simulate API delay
    
    if (Math.random() < 0.1) { // 10% chance of error for demo
      throw new Error('Failed to fetch wallet data from the network');
    }

    return {
      balance: '$2,847.32',
      assets: [
        {
          symbol: 'ETH',
          name: 'Ethereum',
          amount: '0.847 ETH',
          value: '$2,156.32',
          change: '+2.4%',
          logo: 'ETH'
        },
        {
          symbol: 'BTC',
          name: 'Bitcoin',
          amount: '0.021 BTC',
          value: '$691.00',
          change: '-1.2%',
          logo: 'BTC'
        }
      ]
    };
  };

  // Initialize Caesar ecosystem
  useEffect(() => {
    let mounted = true;
    
    const initializeEcosystem = async () => {
      try {
        await caesarFramework.initialize();
        if (mounted) {
          setEcosystemState(caesarFramework.getState());
          showSuccess('Caesar Ecosystem initialized');
        }
      } catch (error) {
        if (mounted) {
          showError(`Failed to initialize Caesar ecosystem: ${error.message}`);
        }
      }
    };

    initializeEcosystem();

    // Subscribe to ecosystem state changes
    const unsubscribe = caesarFramework.onStateChange((state) => {
      if (mounted) {
        setEcosystemState(state);
      }
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, []);

  // Load wallet data when connected
  useEffect(() => {
    if (isConnected && address) {
      showInfo('Loading wallet data...');
      fetchWalletData(loadWalletData)
        .then(() => {
          showSuccess('Wallet data loaded successfully');
        })
        .catch((error) => {
          showError(`Failed to load wallet data: ${error.message}`);
        });
    }
  }, [isConnected, address]);

  const handleRefreshWallet = () => {
    if (isConnected) {
      fetchWalletData(loadWalletData)
        .then(() => {
          showSuccess('Wallet refreshed successfully');
        })
        .catch((error) => {
          showError(`Failed to refresh wallet: ${error.message}`);
        });
    }
  };

  const handleAssetAllocated = (asset: any) => {
    setAllocatedAssets(prev => [...prev, asset]);
    showSuccess(`Successfully allocated ${asset.type} asset`);
    showInfo(`Proxy address: ${asset.proxy_address}`);
  };

  // Caesar Performance Metrics Component
  const CaesarMetrics = () => {
    if (!ecosystemState) return null;
    
    const { stoq, ecosystem } = ecosystemState;
    
    return (
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 sm:gap-6 mb-6 sm:mb-8">
        <div className="bg-gradient-to-br from-blue-600 to-blue-700 rounded-xl p-6 text-white">
          <div className="flex items-center justify-between mb-4">
            <Zap className="h-8 w-8" />
            <span className="text-blue-200 text-sm font-medium">STOQ Transport</span>
          </div>
          <div className="text-3xl font-bold mb-1">{stoq.performance.throughput_gbps.toFixed(1)}</div>
          <div className="text-blue-200 text-sm">Gbps Throughput</div>
          <div className="mt-2 text-xs text-blue-200">
            Target: 40 Gbps • Latency: {stoq.performance.latency_ms}ms
          </div>
        </div>

        <div className="bg-gradient-to-br from-green-600 to-green-700 rounded-xl p-6 text-white">
          <div className="flex items-center justify-between mb-4">
            <Network className="h-8 w-8" />
            <span className="text-green-200 text-sm font-medium">Matrix Chain</span>
          </div>
          <div className="text-3xl font-bold mb-1">{ecosystem.total_entities}</div>
          <div className="text-green-200 text-sm">Connected Entities</div>
          <div className="mt-2 text-xs text-green-200">
            Trust Score: {(ecosystem.overall_trust_score * 100).toFixed(0)}%
          </div>
        </div>

        <div className="bg-gradient-to-br from-purple-600 to-purple-700 rounded-xl p-6 text-white">
          <div className="flex items-center justify-between mb-4">
            <Server className="h-8 w-8" />
            <span className="text-purple-200 text-sm font-medium">HyperMesh Assets</span>
          </div>
          <div className="text-3xl font-bold mb-1">{ecosystem.total_assets}</div>
          <div className="text-purple-200 text-sm">Available Assets</div>
          <div className="mt-2 text-xs text-purple-200">
            Active Connections: {ecosystem.active_connections}
          </div>
        </div>

        <div className="bg-gradient-to-br from-orange-600 to-orange-700 rounded-xl p-6 text-white">
          <div className="flex items-center justify-between mb-4">
            <Gauge className="h-8 w-8" />
            <span className="text-orange-200 text-sm font-medium">Performance</span>
          </div>
          <div className="text-3xl font-bold mb-1">{ecosystem.performance_score.toFixed(0)}</div>
          <div className="text-orange-200 text-sm">Performance Score</div>
          <div className="mt-2 text-xs text-orange-200">
            7x-23x faster than traditional APIs
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Modern Header */}
      <header className="bg-white border-b border-gray-200 sticky top-0 z-50">
        <div className="px-6 py-4">
          <div className="flex items-center justify-between max-w-7xl mx-auto">
            <div className="flex items-center space-x-8">
              <div className="flex items-center space-x-3">
                <div className="w-8 h-8 bg-gradient-to-br from-blue-600 to-purple-600 rounded-lg flex items-center justify-center">
                  <span className="text-white font-bold text-sm">C</span>
                </div>
                <h1 className="text-xl font-bold text-gray-900">Caesar Wallet</h1>
              </div>
              
              {/* Navigation Pills */}
              <nav className="hidden md:flex space-x-1">
                {[
                  { key: 'dashboard', label: 'Dashboard', icon: Activity },
                  { key: 'assets', label: 'Assets', icon: Server },
                  { key: 'matrix', label: 'Matrix', icon: Network },
                  { key: 'performance', label: 'Performance', icon: Zap },
                  { key: 'network', label: 'Network', icon: Globe },
                ].map((item) => (
                  <button
                    key={item.key}
                    onClick={() => setActiveView(item.key as any)}
                    className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                      activeView === item.key
                        ? 'bg-blue-100 text-blue-700'
                        : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'
                    }`}
                  >
                    <item.icon size={16} />
                    <span>{item.label}</span>
                  </button>
                ))}
              </nav>
              
              {/* Mobile Menu Button */}
              <button
                onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
                className="md:hidden p-2 text-gray-600 hover:text-gray-900 rounded-lg hover:bg-gray-100"
              >
                {mobileMenuOpen ? <X size={20} /> : <Menu size={20} />}
              </button>
            </div>
            
            <div className="flex items-center space-x-4">
              {ecosystemState && (
                <div className="hidden lg:flex items-center space-x-4 text-sm text-gray-600">
                  <div className="flex items-center space-x-1">
                    <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                    <span>STOQ: {ecosystemState.stoq.performance.throughput_gbps.toFixed(1)} Gbps</span>
                  </div>
                  <div className="flex items-center space-x-1">
                    <div className="w-2 h-2 bg-blue-500 rounded-full"></div>
                    <span>{ecosystemState.ecosystem.total_entities} Entities</span>
                  </div>
                </div>
              )}
              <WalletConnect />
              <button className="p-2 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100">
                <Settings size={20} />
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Mobile Menu Overlay */}
      {mobileMenuOpen && (
        <div className="md:hidden fixed inset-0 z-50 bg-black bg-opacity-50" onClick={() => setMobileMenuOpen(false)}>
          <div className="bg-white w-80 h-full shadow-xl" onClick={(e) => e.stopPropagation()}>
            <div className="p-6 border-b border-gray-200">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 bg-gradient-to-br from-blue-600 to-purple-600 rounded-lg flex items-center justify-center">
                    <span className="text-white font-bold text-sm">C</span>
                  </div>
                  <h1 className="text-xl font-bold text-gray-900">Caesar Wallet</h1>
                </div>
                <button
                  onClick={() => setMobileMenuOpen(false)}
                  className="p-2 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100"
                >
                  <X size={20} />
                </button>
              </div>
            </div>
            
            <nav className="p-6">
              <div className="space-y-2">
                {[
                  { key: 'dashboard', label: 'Dashboard', icon: Activity },
                  { key: 'assets', label: 'Assets', icon: Server },
                  { key: 'matrix', label: 'Matrix', icon: Network },
                  { key: 'performance', label: 'Performance', icon: Zap },
                  { key: 'network', label: 'Network', icon: Globe },
                ].map((item) => (
                  <button
                    key={item.key}
                    onClick={() => {
                      setActiveView(item.key as any);
                      setMobileMenuOpen(false);
                    }}
                    className={`w-full flex items-center space-x-3 px-4 py-3 rounded-lg text-left font-medium transition-all ${
                      activeView === item.key
                        ? 'bg-blue-100 text-blue-700'
                        : 'text-gray-700 hover:text-gray-900 hover:bg-gray-100'
                    }`}
                  >
                    <item.icon size={20} />
                    <span>{item.label}</span>
                  </button>
                ))}
              </div>
            </nav>

            {/* Mobile Network Status */}
            {ecosystemState && (
              <div className="p-6 border-t border-gray-200">
                <h3 className="text-sm font-semibold text-gray-900 mb-3">Network Status</h3>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-gray-600">STOQ Transport</span>
                    <div className="flex items-center space-x-1">
                      <div className="w-1.5 h-1.5 bg-green-500 rounded-full"></div>
                      <span className="text-xs font-medium text-gray-900">
                        {ecosystemState.stoq.performance.throughput_gbps.toFixed(1)} Gbps
                      </span>
                    </div>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-gray-600">Entities</span>
                    <span className="text-xs font-medium text-gray-900">
                      {ecosystemState.ecosystem.total_entities}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-gray-600">Assets</span>
                    <span className="text-xs font-medium text-gray-900">
                      {ecosystemState.ecosystem.total_assets}
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 py-4 sm:py-8">
        {activeView === 'dashboard' && (
          <>
            {/* Caesar Performance Metrics */}
            <CaesarMetrics />

            {/* Wallet Overview */}
            {!isConnected ? (
              <div className="bg-white rounded-xl p-8 text-center">
                <WalletError 
                  error="not_connected"
                  onConnect={() => showInfo('Please use the Connect Wallet button in the header')}
                />
              </div>
            ) : (
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 lg:gap-8">
                {/* Portfolio Balance */}
                <div className="lg:col-span-2">
                  <div className="bg-white rounded-xl p-6 mb-6">
                    <div className="flex items-center justify-between mb-6">
                      <h3 className="text-lg font-semibold text-gray-900">Portfolio Balance</h3>
                      <LoadingButton
                        isLoading={walletLoading}
                        onClick={handleRefreshWallet}
                        variant="secondary"
                        className="px-3 py-1 text-sm"
                      >
                        Refresh
                      </LoadingButton>
                    </div>
                    
                    {walletLoading ? (
                      <WalletLoadingSkeleton />
                    ) : walletError ? (
                      <ErrorCard
                        title="Failed to Load Wallet"
                        message={walletError.message}
                        type="error"
                        onRetry={handleRefreshWallet}
                      />
                    ) : walletData ? (
                      <div>
                        <div className="mb-6">
                          <p className="text-sm text-gray-500 mb-2">Total Balance</p>
                          <div className="flex items-baseline space-x-4">
                            <h2 className="text-4xl font-bold text-gray-900">{walletData.balance}</h2>
                            <span className="text-green-600 text-sm font-medium flex items-center">
                              <ArrowUpRight size={16} className="mr-1" />
                              +12.5% (24h)
                            </span>
                          </div>
                        </div>

                        {/* Action Buttons */}
                        <div className="flex space-x-3 mb-6">
                          <LoadingButton
                            isLoading={false}
                            onClick={() => showInfo('Send functionality coming soon')}
                            variant="primary"
                            className="flex-1"
                          >
                            <ArrowUpRight size={16} className="mr-2" />
                            Send
                          </LoadingButton>
                          <LoadingButton
                            isLoading={false}
                            onClick={() => showInfo('Receive functionality coming soon')}
                            variant="secondary"
                            className="flex-1"
                          >
                            <ArrowDownRight size={16} className="mr-2" />
                            Receive
                          </LoadingButton>
                          <LoadingButton
                            isLoading={false}
                            onClick={() => showInfo('Buy functionality coming soon')}
                            variant="secondary"
                            className="flex-1"
                          >
                            <CreditCard size={16} className="mr-2" />
                            Buy
                          </LoadingButton>
                        </div>

                        {/* Assets List */}
                        <div className="space-y-3">
                          {walletData.assets.map((asset, index) => (
                            <div key={index} className="flex items-center justify-between p-4 rounded-lg border border-gray-100 hover:bg-gray-50 transition-colors">
                              <div className="flex items-center space-x-4">
                                <div className={`w-12 h-12 ${asset.symbol === 'ETH' ? 'bg-blue-100' : 'bg-orange-100'} rounded-full flex items-center justify-center`}>
                                  <span className={`${asset.symbol === 'ETH' ? 'text-blue-600' : 'text-orange-600'} font-medium`}>
                                    {asset.logo}
                                  </span>
                                </div>
                                <div>
                                  <p className="font-medium text-gray-900">{asset.name}</p>
                                  <p className="text-sm text-gray-500">{asset.amount}</p>
                                </div>
                              </div>
                              <div className="text-right">
                                <p className="font-medium text-gray-900">{asset.value}</p>
                                <p className={`text-sm ${asset.change.startsWith('+') ? 'text-green-600' : 'text-red-600'}`}>
                                  {asset.change}
                                </p>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : (
                      <ErrorCard
                        title="No Data Available"
                        message="Unable to load wallet data"
                        type="warning"
                        onRetry={handleRefreshWallet}
                      />
                    )}
                  </div>
                </div>

                {/* Recent Activity Sidebar */}
                <div className="space-y-6">
                  {/* Quick Actions */}
                  <div className="bg-white rounded-xl p-6">
                    <h3 className="text-lg font-semibold text-gray-900 mb-4">Quick Actions</h3>
                    <div className="space-y-3">
                      <button 
                        onClick={() => setActiveView('assets')}
                        className="w-full flex items-center justify-between p-3 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors"
                      >
                        <div className="flex items-center space-x-3">
                          <Server className="h-5 w-5 text-gray-400" />
                          <span className="text-sm font-medium text-gray-900">Browse Assets</span>
                        </div>
                        <ChevronRight className="h-4 w-4 text-gray-400" />
                      </button>
                      <button 
                        onClick={() => setActiveView('matrix')}
                        className="w-full flex items-center justify-between p-3 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors"
                      >
                        <div className="flex items-center space-x-3">
                          <Network className="h-5 w-5 text-gray-400" />
                          <span className="text-sm font-medium text-gray-900">Matrix Chain</span>
                        </div>
                        <ChevronRight className="h-4 w-4 text-gray-400" />
                      </button>
                      <button 
                        onClick={() => setActiveView('performance')}
                        className="w-full flex items-center justify-between p-3 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors"
                      >
                        <div className="flex items-center space-x-3">
                          <Zap className="h-5 w-5 text-gray-400" />
                          <span className="text-sm font-medium text-gray-900">Performance</span>
                        </div>
                        <ChevronRight className="h-4 w-4 text-gray-400" />
                      </button>
                    </div>
                  </div>

                  {/* Live Network Status */}
                  {ecosystemState && (
                    <div className="bg-white rounded-xl p-6">
                      <h3 className="text-lg font-semibold text-gray-900 mb-4">Live Network Status</h3>
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <span className="text-sm text-gray-600">STOQ Transport</span>
                          <div className="flex items-center space-x-2">
                            <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                            <span className="text-sm font-medium text-gray-900">
                              {ecosystemState.stoq.performance.throughput_gbps.toFixed(1)} Gbps
                            </span>
                          </div>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-sm text-gray-600">Consensus Time</span>
                          <span className="text-sm font-medium text-gray-900">
                            {ecosystemState.matrix.consensus_metrics.avg_validation_time}ms
                          </span>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-sm text-gray-600">Active Entities</span>
                          <span className="text-sm font-medium text-gray-900">
                            {ecosystemState.ecosystem.total_entities}
                          </span>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-sm text-gray-600">Available Assets</span>
                          <span className="text-sm font-medium text-gray-900">
                            {ecosystemState.ecosystem.total_assets}
                          </span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </>
        )}

        {activeView === 'assets' && (
          <div className="bg-white rounded-xl min-h-[600px]">
            <SimpleAssetBrowser
              userId={address || 'anonymous'}
              onAssetAllocated={handleAssetAllocated}
              className="h-full"
            />
          </div>
        )}

        {activeView === 'matrix' && (
          <div className="bg-white rounded-xl min-h-[600px] overflow-auto">
            <MatrixChainInterface />
          </div>
        )}

        {activeView === 'performance' && (
          <div className="bg-white rounded-xl min-h-[600px] overflow-auto">
            <PerformanceComparison />
          </div>
        )}

        {activeView === 'network' && (
          <div className="bg-white rounded-xl p-8">
            <h2 className="text-2xl font-bold text-gray-900 mb-6">Network Overview</h2>
            <div className="text-center text-gray-500">
              Network monitoring interface coming soon...
            </div>
          </div>
        )}
      </main>

    </div>
  );
}

export default App;