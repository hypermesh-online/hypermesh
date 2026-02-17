// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useState } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from '@/components/ui/toaster';
import { KeyboardNavigationProvider } from '@/components/ui/KeyboardNavigationProvider';
import { Layout } from './components/Layout';
import { DashboardHome } from './components/DashboardHome';
import { HyperMeshModule } from './components/modules/HyperMeshModule';
import { CatalogModule } from './components/modules/CatalogModule';
import { TrustChainModule } from './components/modules/TrustChainModule';
import CaesarModule from './components/modules/CaesarModule';
import { DashboardMonitor } from './components/api/DashboardMonitor';
import { IntegrationTest } from './components/IntegrationTest';
import { StoqNativeDemo } from './components/StoqNativeDemo';
import { initializeWeb3API, connectToAllServices } from './lib/api';

// Configure React Query client for Web3 ecosystem
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30000, // 30 seconds
      retry: (failureCount, error: any) => {
        // Don't retry on authentication errors
        if (error?.status === 401) return false;
        return failureCount < 3;
      },
      refetchOnWindowFocus: false, // Disable for development
    },
    mutations: {
      retry: 1,
    },
  },
});

export default function App() {
  const [apiInitialized, setApiInitialized] = useState(false);
  const [initializationError, setInitializationError] = useState<string | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<Record<string, boolean>>({});
  const [protocol, setProtocol] = useState<string>('unknown');
  const [connectionId, setConnectionId] = useState<string | null>(null);

  useEffect(() => {
    async function initializeAPI() {
      try {
        // Fetch the real TrustChain root certificate from the backend
        console.log('Fetching TrustChain root certificate...');

        let certificate = '';
        try {
          // Try to fetch the root certificate from TrustChain API
          const response = await fetch('/api/v1/trustchain/certificates/root');
          if (response.ok) {
            const data = await response.json();
            if (data.certificate && data.certificate !== '') {
              certificate = data.certificate;
              console.log('Using TrustChain root certificate');
            } else {
              console.log('TrustChain root certificate not initialized yet');
            }
          }
        } catch (error) {
          console.warn('Failed to fetch TrustChain root certificate:', error);
        }

        // If no certificate available, proceed without one (development mode)
        if (!certificate) {
          console.log('Proceeding without certificate (development mode)');
          certificate = ''; // Empty certificate for development
        }

        console.log('Initializing Web3 API client...');
        const result = await initializeWeb3API(certificate);
        
        if (result.success) {
          setProtocol(result.protocol || 'unknown');
          setConnectionId(result.connectionId || null);
          
          if (result.protocol === 'stoq-native') {
            // STOQ native connection successful
            setConnectionStatus({
              trustchain: true,
              stoq: true,
              hypermesh: true,
              integration: true
            });
            setApiInitialized(true);
            console.log(`Web3 API integration complete via STOQ native protocol`);
          } else {
            // HTTP fallback - connect to services
            console.log('Connecting to Web3 services via HTTP...');
            const connectionResult = await connectToAllServices();
            setConnectionStatus({
              trustchain: connectionResult.connected >= 1,
              stoq: connectionResult.connected >= 2,
              hypermesh: connectionResult.connected >= 3,
              integration: connectionResult.connected >= 4
            });
            setApiInitialized(true);
            console.log(`Web3 API integration complete: ${connectionResult.connected}/${connectionResult.total} services connected via HTTP`);
          }
        } else {
          throw new Error(result.error || 'Failed to initialize API');
        }
      } catch (error) {
        console.error('Web3 API initialization failed:', error);
        setInitializationError(error instanceof Error ? error.message : 'Unknown error');
        // Continue without API for development
        setApiInitialized(true);
      }
    }

    initializeAPI();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <KeyboardNavigationProvider>
        <Router>
          <div className="min-h-screen bg-black text-white">
            {/* Show initialization status */}
            {!apiInitialized && (
              <div className="fixed top-4 right-4 z-50 bg-blue-600 text-white px-4 py-2 rounded-lg shadow-lg">
                <div className="flex items-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  Initializing Web3 API...
                </div>
              </div>
            )}
            
            {apiInitialized && !initializationError && (
              <div className="fixed top-4 right-4 z-50 bg-green-600 text-white px-4 py-2 rounded-lg shadow-lg">
                <div className="text-sm font-medium">
                  {protocol === 'stoq-native' ? 'STOQ Native Connected' : 'API Connected'}
                </div>
                <div className="text-xs">
                  {protocol === 'stoq-native' ? (
                    <>Protocol: QUIC/IPv6 {connectionId && `(${connectionId.slice(0, 8)}...)`}</>
                  ) : (
                    <>Services: {Object.values(connectionStatus).filter(Boolean).length}/4 online</>
                  )}
                </div>
              </div>
            )}
            
            {initializationError && (
              <div className="fixed top-4 right-4 z-50 bg-orange-600 text-white px-4 py-2 rounded-lg max-w-md shadow-lg">
                <div className="text-sm font-medium">API Warning</div>
                <div className="text-xs">{initializationError}</div>
                <div className="text-xs mt-1">Running in offline mode</div>
              </div>
            )}
            
            <Layout>
              <Routes>
                <Route path="/" element={<DashboardHome />} />
                <Route path="/monitor" element={<DashboardMonitor />} />
                <Route path="/integration" element={<IntegrationTest />} />
                <Route path="/stoq-demo" element={<StoqNativeDemo />} />
                <Route path="/hypermesh/*" element={<HyperMeshModule />} />
                <Route path="/catalog/*" element={<CatalogModule />} />
                <Route path="/trustchain/*" element={<TrustChainModule />} />
                <Route path="/caesar/*" element={<CaesarModule />} />
              </Routes>
            </Layout>
            <Toaster />
          </div>
        </Router>
      </KeyboardNavigationProvider>
    </QueryClientProvider>
  );
}
