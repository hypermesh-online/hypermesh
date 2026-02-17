// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Wallet, Power, AlertCircle } from 'lucide-react';
import { useWallet } from '../hooks/useWallet';
import { formatAddress } from '../utils/formatters';
import { SUPPORTED_CHAINS } from '../utils/constants';

const WalletConnection: React.FC = () => {
  const { wallet, isConnecting, connect, disconnect, switchToSepolia } = useWallet();

  const isWrongNetwork = wallet.chainId && wallet.chainId !== SUPPORTED_CHAINS.SEPOLIA.chainId;

  if (!wallet.isConnected) {
    return (
      <button
        onClick={connect}
        disabled={isConnecting}
        className="btn-primary flex items-center gap-2"
      >
        <Wallet size={20} />
        {isConnecting ? 'Connecting...' : 'Connect Wallet'}
      </button>
    );
  }

  return (
    <div className="flex items-center gap-3">
      {isWrongNetwork && (
        <div className="flex items-center gap-2 bg-red-600 px-3 py-2 rounded-lg text-sm">
          <AlertCircle size={16} />
          <span>Wrong Network</span>
          <button
            onClick={switchToSepolia}
            className="text-white hover:underline"
          >
            Switch to Sepolia
          </button>
        </div>
      )}
      
      <div className="bg-caesar-gray px-4 py-2 rounded-lg flex items-center gap-2">
        <div className="w-2 h-2 bg-green-500 rounded-full"></div>
        <span className="text-sm font-medium">
          {formatAddress(wallet.account!)}
        </span>
      </div>
      
      <button
        onClick={disconnect}
        className="p-2 hover:bg-caesar-gray rounded-lg transition-colors"
        title="Disconnect Wallet"
      >
        <Power size={20} />
      </button>
    </div>
  );
};

export default WalletConnection;