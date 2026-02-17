// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState } from 'react';
import { useAccount, useConnect, useDisconnect, useChainId } from 'wagmi';
import { Wallet, ChevronDown, CheckCircle, AlertCircle, ExternalLink } from 'lucide-react';
import { formatAddress, getNetworkName, isSupportedChain } from '../utils/wallet';

interface WalletConnectProps {
  onWalletConnected?: (address: string, chainId: number) => void;
}

const WalletConnect: React.FC<WalletConnectProps> = ({ onWalletConnected }) => {
  const [showConnectors, setShowConnectors] = useState(false);
  const { address, isConnected, connector } = useAccount();
  const { connect, connectors, isPending, error } = useConnect();
  const { disconnect } = useDisconnect();
  const chainId = useChainId();

  const handleConnect = (connector: any) => {
    connect({ connector });
    setShowConnectors(false);
  };

  const handleDisconnect = () => {
    disconnect();
  };

  // Notify parent component when wallet connects
  React.useEffect(() => {
    if (isConnected && address && chainId && onWalletConnected) {
      onWalletConnected(address, chainId);
    }
  }, [isConnected, address, chainId, onWalletConnected]);

  const getConnectorIcon = (connectorName: string) => {
    switch (connectorName.toLowerCase()) {
      case 'metamask':
      case 'injected':
        return '🦊';
      case 'walletconnect':
        return '🔗';
      case 'coinbase wallet':
      case 'coinbasewallet':
        return '🔵';
      default:
        return '👛';
    }
  };

  if (isConnected && address) {
    return (
      <div className="relative">
        <button
          onClick={() => setShowConnectors(!showConnectors)}
          className="flex items-center gap-3 px-4 py-3 bg-white border border-gray-200 rounded-lg shadow-sm min-w-[200px] transition-colors hover:bg-gray-50"
        >
          <div className="flex items-center gap-2">
            {isSupportedChain(chainId) ? (
              <CheckCircle size={16} className="text-green-400" />
            ) : (
              <AlertCircle size={16} className="text-yellow-400" />
            )}
            <div className="text-left">
              <div className="font-medium text-gray-900">{formatAddress(address)}</div>
              <div className="text-xs text-gray-500 flex items-center gap-1">
                {getNetworkName(chainId)}
                {!isSupportedChain(chainId) && (
                  <span className="bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded text-xs">UNSUPPORTED</span>
                )}
              </div>
            </div>
          </div>
          <ChevronDown 
            size={16} 
            className={`text-gray-400 transition-transform ${showConnectors ? 'rotate-180' : ''}`} 
          />
        </button>

        {/* Dropdown */}
        {showConnectors && (
          <div className="absolute top-full left-0 right-0 mt-2 bg-white border border-gray-200 rounded-lg shadow-lg z-50 min-w-[250px]">
            {/* Wallet Info */}
            <div className="p-4 border-b border-gray-200">
              <div className="flex items-center gap-3 mb-3">
                <span className="text-2xl">{getConnectorIcon(connector?.name || '')}</span>
                <div>
                  <p className="font-medium text-gray-900">{connector?.name}</p>
                  <p className="text-sm text-gray-500">Connected</p>
                </div>
              </div>
              
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500">Address:</span>
                  <span className="font-mono text-gray-900">{formatAddress(address)}</span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500">Network:</span>
                  <span className="text-gray-900">{getNetworkName(chainId)}</span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500">Status:</span>
                  <div className="flex items-center gap-1">
                    {isSupportedChain(chainId) ? (
                      <>
                        <CheckCircle size={12} className="text-green-600" />
                        <span className="text-green-600">Supported</span>
                      </>
                    ) : (
                      <>
                        <AlertCircle size={12} className="text-yellow-600" />
                        <span className="text-yellow-600">Switch Network</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="p-4 space-y-2">
              <button
                onClick={() => window.open(`https://etherscan.io/address/${address}`, '_blank')}
                className="w-full flex items-center justify-between p-2 hover:bg-white hover:bg-opacity-5 rounded-lg transition-colors text-left"
              >
                <span>View on Explorer</span>
                <ExternalLink size={14} />
              </button>
              
              <button
                onClick={() => {
                  navigator.clipboard.writeText(address);
                  setShowConnectors(false);
                }}
                className="w-full flex items-center justify-between p-2 hover:bg-white hover:bg-opacity-5 rounded-lg transition-colors text-left"
              >
                <span>Copy Address</span>
                <span>📋</span>
              </button>
              
              <button
                onClick={handleDisconnect}
                className="w-full p-2 text-red-400 hover:bg-red-500 hover:bg-opacity-10 rounded-lg transition-colors text-left"
              >
                Disconnect
              </button>
            </div>
          </div>
        )}

        {/* Overlay */}
        {showConnectors && (
          <div 
            className="fixed inset-0 z-40" 
            onClick={() => setShowConnectors(false)}
          />
        )}
      </div>
    );
  }

  return (
    <div className="relative">
      <button
        onClick={() => setShowConnectors(!showConnectors)}
        className="flex items-center gap-2 px-4 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
      >
        <Wallet size={20} />
        Connect Wallet
      </button>

      {/* Connector Selection Modal */}
      {showConnectors && (
        <>
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
            <div className="bg-white rounded-lg shadow-xl max-w-md w-full">
              {/* Header */}
              <div className="p-6 border-b border-gray-200">
                <h3 className="text-xl font-semibold text-gray-900">Connect Wallet</h3>
                <p className="text-gray-500 text-sm mt-1">
                  Choose your preferred wallet to connect to Satchel
                </p>
              </div>

              {/* Connectors */}
              <div className="p-6 space-y-3">
                {connectors.map((connector) => (
                  <button
                    key={connector.uid}
                    onClick={() => handleConnect(connector)}
                    disabled={isPending}
                    className="w-full flex items-center justify-between p-4 border border-gray-200 hover:border-gray-300 hover:bg-gray-50 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <div className="flex items-center gap-3">
                      <span className="text-2xl">{getConnectorIcon(connector.name)}</span>
                      <div className="text-left">
                        <p className="font-medium text-gray-900">{connector.name}</p>
                        <p className="text-sm text-gray-500">
                          {connector.name === 'MetaMask' && 'Connect using MetaMask browser extension'}
                          {connector.name === 'WalletConnect' && 'Connect using WalletConnect protocol'}
                          {connector.name === 'Coinbase Wallet' && 'Connect using Coinbase Wallet'}
                        </p>
                      </div>
                    </div>
                    {isPending && (
                      <div className="w-4 h-4 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
                    )}
                  </button>
                ))}
              </div>

              {/* Error */}
              {error && (
                <div className="p-6 border-t border-gray-200">
                  <div className="flex items-start gap-3 p-3 bg-red-50 border border-red-200 rounded-lg">
                    <AlertCircle size={16} className="text-red-600 mt-0.5 flex-shrink-0" />
                    <div>
                      <p className="text-red-800 font-medium">Connection Error</p>
                      <p className="text-red-600 text-sm mt-1">{error.message}</p>
                    </div>
                  </div>
                </div>
              )}

              {/* Footer */}
              <div className="p-6 border-t border-gray-200">
                <button
                  onClick={() => setShowConnectors(false)}
                  className="w-full px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default WalletConnect;