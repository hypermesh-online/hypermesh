// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useState } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from '@/components/ui/toaster';
import { KeyboardNavigationProvider } from '@/components/ui/KeyboardNavigationProvider';
import { ErrorBoundary } from '@/components/ui/ErrorBoundary';
import { Layout } from './components/Layout';
import { DashboardHome } from './components/DashboardHome';
import { HyperMeshModule } from './components/modules/HyperMeshModule';
import { CatalogModule } from './components/modules/CatalogModule';
import { TrustChainModule } from './components/modules/TrustChainModule';
import CaesarModule from './components/modules/CaesarModule';
import NGaugeModule from './components/modules/NGaugeModule';
import { StoqModule } from './components/modules/StoqModule';
import { DashboardMonitor } from './components/api/DashboardMonitor';
import { InboxPanel } from './components/sharing/InboxPanel';
import { MessengerPanel } from './components/messaging/MessengerPanel';
import { blockMatrixClient } from './lib/blockmatrix-api';
import { useSetupWizardGate } from './components/wizard';

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
  // Phase C.3 — gates the first-run wizard. Returns null in non-Tauri builds
  // so the Gateway-served standalone UI is unaffected.
  const wizardModal = useSetupWizardGate();

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
            {wizardModal}
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
                <Route path="/monitor" element={<ErrorBoundary module="Monitor"><DashboardMonitor /></ErrorBoundary>} />
                <Route path="/hypermesh/*" element={<ErrorBoundary module="HyperMesh"><HyperMeshModule /></ErrorBoundary>} />
                <Route path="/catalog/*" element={<ErrorBoundary module="Catalog"><CatalogModule /></ErrorBoundary>} />
                <Route path="/trustchain/*" element={<ErrorBoundary module="TrustChain"><TrustChainModule /></ErrorBoundary>} />
                <Route path="/caesar/*" element={<ErrorBoundary module="Caesar"><CaesarModule /></ErrorBoundary>} />
                <Route path="/ngauge/*" element={<ErrorBoundary module="NGauge"><NGaugeModule /></ErrorBoundary>} />
                <Route path="/stoq/*" element={<ErrorBoundary module="STOQ"><StoqModule /></ErrorBoundary>} />
                <Route path="/inbox" element={<ErrorBoundary module="Inbox"><InboxPanel /></ErrorBoundary>} />
                <Route path="/messages" element={<ErrorBoundary module="Messages"><MessengerPanel /></ErrorBoundary>} />
              </Routes>
            </Layout>
            <Toaster />
          </div>
        </Router>
      </KeyboardNavigationProvider>
    </QueryClientProvider>
  );
}
