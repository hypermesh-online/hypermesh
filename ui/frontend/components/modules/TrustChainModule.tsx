// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Shield } from 'lucide-react';
import { TrustChainRouting } from '../trustchain/TrustChainRouting';

export function TrustChainModule() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-green-400 to-emerald-600">
            <Shield className="h-8 w-8 text-black" />
          </div>
          TrustChain
        </h1>
        <p className="text-gray-400 mt-2">
          Network connections and Proof of State verification for Public, P2P, and Federated networks
        </p>
      </div>

      <TrustChainRouting />
    </div>
  );
}
