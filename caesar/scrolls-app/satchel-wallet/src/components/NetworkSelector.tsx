// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState } from 'react';
import { ChevronDown, Wifi, WifiOff } from 'lucide-react';
import { Network } from '../types';
import { SUPPORTED_NETWORKS } from '../utils/networks';

interface NetworkSelectorProps {
  currentNetwork: Network | null;
  onNetworkChange: (network: Network) => void;
}

const NetworkSelector: React.FC<NetworkSelectorProps> = ({
  currentNetwork,
  onNetworkChange,
}) => {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 bg-caesar-gray px-3 py-2 rounded-lg hover:bg-gray-600 transition-colors"
      >
        {currentNetwork ? (
          <>
            <img
              src={currentNetwork.logoUrl || '/default-network.png'}
              alt={currentNetwork.name}
              className="w-5 h-5 rounded-full"
            />
            <span className="font-medium">{currentNetwork.name}</span>
            <Wifi size={16} className="text-green-500" />
          </>
        ) : (
          <>
            <WifiOff size={16} className="text-red-500" />
            <span className="font-medium">No Network</span>
          </>
        )}
        <ChevronDown size={16} className={`transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </button>

      {isOpen && (
        <div className="absolute top-full left-0 mt-2 w-64 bg-caesar-gray border border-gray-700 rounded-lg shadow-lg z-50">
          <div className="p-2">
            <p className="text-gray-400 text-sm mb-2 px-2">Select Network</p>
            {SUPPORTED_NETWORKS.map((network) => (
              <button
                key={network.chainId}
                onClick={() => {
                  onNetworkChange(network);
                  setIsOpen(false);
                }}
                className={`w-full flex items-center gap-3 p-3 rounded-lg hover:bg-gray-700 transition-colors ${
                  currentNetwork?.chainId === network.chainId ? 'bg-gray-700' : ''
                }`}
              >
                <img
                  src={network.logoUrl || '/default-network.png'}
                  alt={network.name}
                  className="w-6 h-6 rounded-full"
                />
                <div className="text-left flex-1">
                  <p className="font-medium">{network.name}</p>
                  <p className="text-gray-400 text-sm">{network.symbol}</p>
                </div>
                {network.isTestnet && (
                  <span className="bg-orange-500/20 text-orange-400 px-2 py-1 rounded text-xs">
                    Testnet
                  </span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}

      {isOpen && (
        <div
          className="fixed inset-0 z-40"
          onClick={() => setIsOpen(false)}
        />
      )}
    </div>
  );
};

export default NetworkSelector;