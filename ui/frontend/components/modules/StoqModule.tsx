// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route } from 'react-router-dom';
import { Zap } from 'lucide-react';
import {
  StoqOverview,
  ProtocolDetails,
  TransportDashboard,
  PerformanceView,
  SubNavigation,
} from './stoq';


export function StoqModule() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-cyan-400 to-blue-600">
            <Zap className="h-8 w-8 text-black" />
          </div>
          STOQ
        </h1>
        <p className="text-gray-400 mt-2">
          Secure Tokenization Over QUIC - High-performance transport protocol
        </p>
      </div>

      <SubNavigation />

      <Routes>
        <Route path="/" element={<StoqOverview />} />
        <Route path="/protocol" element={<ProtocolDetails />} />
        <Route path="/tunnels" element={<TransportDashboard />} />
        <Route path="/performance" element={<PerformanceView />} />
      </Routes>
    </div>
  );
}
