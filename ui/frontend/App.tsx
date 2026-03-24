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
import { InboxPanel } from './components/sharing/InboxPanel';
import { MessengerPanel } from './components/messaging/MessengerPanel';
import { blockMatrixClient } from './lib/blockmatrix-api';

// Configure React Query client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10_000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: 1,
    },
  },
});

export default function App() {
  const [backendOnline, setBackendOnline] = useState<boolean | null>(null);

  useEffect(() => {
    // Simple liveness check against the real BlockMatrix HTTP API
    blockMatrixClient
      .ping()
      .then(() => setBackendOnline(true))
      .catch(() => setBackendOnline(false));
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <KeyboardNavigationProvider>
        <Router>
          <div className="min-h-screen bg-black text-white">
            {/* Compact connection indicator */}
            {backendOnline === true && (
              <div className="fixed top-4 right-4 z-50 bg-green-600/90 text-white px-3 py-1.5 rounded-lg shadow-lg text-xs font-medium">
                BlockMatrix Connected
              </div>
            )}

            {backendOnline === false && (
              <div className="fixed top-4 right-4 z-50 bg-orange-600/90 text-white px-3 py-1.5 rounded-lg shadow-lg text-xs">
                Daemon offline
              </div>
            )}

            <Layout>
              <Routes>
                <Route path="/" element={<DashboardHome />} />
                <Route path="/monitor" element={<DashboardMonitor />} />
                <Route path="/hypermesh/*" element={<HyperMeshModule />} />
                <Route path="/catalog/*" element={<CatalogModule />} />
                <Route path="/trustchain/*" element={<TrustChainModule />} />
                <Route path="/caesar/*" element={<CaesarModule />} />
                <Route path="/inbox" element={<InboxPanel />} />
                <Route path="/messages" element={<MessengerPanel />} />
              </Routes>
            </Layout>
            <Toaster />
          </div>
        </Router>
      </KeyboardNavigationProvider>
    </QueryClientProvider>
  );
}
