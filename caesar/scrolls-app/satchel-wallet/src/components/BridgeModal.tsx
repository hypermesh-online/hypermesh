// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { X, ArrowDown, AlertCircle, Clock, CheckCircle, Loader } from 'lucide-react';
import { Network, Token } from '../types';
import { SUPPORTED_NETWORKS } from '../utils/networks';
// import { getBridgeInfo } from '../utils/bridges';

interface BridgeModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedToken?: Token;
  currentNetwork: Network;
}

interface BridgeQuote {
  protocol: 'hyperlane' | 'layerzero';
  fromChain: Network;
  toChain: Network;
  token: Token;
  amount: string;
  estimatedTime: string;
  fees: {
    gas: string;
    bridge: string;
    total: string;
  };
  route: string[];
}

interface BridgeTransaction {
  id: string;
  status: 'pending' | 'confirming' | 'bridging' | 'completed' | 'failed';
  txHash?: string;
  destinationTxHash?: string;
  quote: BridgeQuote;
  timestamp: number;
}

const BridgeModal: React.FC<BridgeModalProps> = ({
  isOpen,
  onClose,
  selectedToken,
  currentNetwork
}) => {
  const [fromNetwork] = useState<Network>(currentNetwork);
  const [toNetwork, setToNetwork] = useState<Network>(SUPPORTED_NETWORKS[0]);
  const [amount, setAmount] = useState('');
  const [quotes, setQuotes] = useState<BridgeQuote[]>([]);
  const [selectedQuote, setSelectedQuote] = useState<BridgeQuote | null>(null);
  const [bridgeTransaction, setBridgeTransaction] = useState<BridgeTransaction | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  // Get compatible bridge networks for selected token
  const getCompatibleNetworks = () => {
    if (!selectedToken) return SUPPORTED_NETWORKS;
    
    // For EVM tokens, show all EVM networks that support bridges
    if (typeof fromNetwork.chainId === 'number') {
      return SUPPORTED_NETWORKS.filter(network => 
        typeof network.chainId === 'number' && 
        network.chainId !== fromNetwork.chainId &&
        (network.features?.includes('hyperlane-endpoint') || 
         network.features?.includes('layerzero-endpoint'))
      );
    }
    
    return SUPPORTED_NETWORKS.filter(network => network.chainId !== fromNetwork.chainId);
  };

  // Fetch bridge quotes
  const fetchBridgeQuotes = async () => {
    if (!selectedToken || !amount || parseFloat(amount) <= 0) {
      setQuotes([]);
      return;
    }

    setIsLoading(true);
    
    try {
      // Mock bridge quotes - in production, these would come from bridge APIs
      const mockQuotes: BridgeQuote[] = [];

      // Hyperlane quote (if supported)
      if (fromNetwork.features?.includes('hyperlane-endpoint') && 
          toNetwork.features?.includes('hyperlane-endpoint')) {
        mockQuotes.push({
          protocol: 'hyperlane',
          fromChain: fromNetwork,
          toChain: toNetwork,
          token: selectedToken,
          amount,
          estimatedTime: '2-5 minutes',
          fees: {
            gas: '0.003 ETH',
            bridge: '0.1%',
            total: '~$8.50'
          },
          route: [fromNetwork.name, 'Hyperlane Bridge', toNetwork.name]
        });
      }

      // LayerZero quote (if supported)
      if (fromNetwork.features?.includes('layerzero-endpoint') && 
          toNetwork.features?.includes('layerzero-endpoint')) {
        mockQuotes.push({
          protocol: 'layerzero',
          fromChain: fromNetwork,
          toChain: toNetwork,
          token: selectedToken,
          amount,
          estimatedTime: '5-15 minutes',
          fees: {
            gas: '0.004 ETH',
            bridge: '0.05%',
            total: '~$12.30'
          },
          route: [fromNetwork.name, 'LayerZero Bridge', toNetwork.name]
        });
      }

      setQuotes(mockQuotes);
      if (mockQuotes.length > 0) {
        setSelectedQuote(mockQuotes[0]); // Select best quote by default
      }
    } catch (error) {
      console.error('Error fetching bridge quotes:', error);
      setQuotes([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Execute bridge transaction
  const executeBridge = async () => {
    if (!selectedQuote) return;

    const transaction: BridgeTransaction = {
      id: `bridge_${Date.now()}`,
      status: 'pending',
      quote: selectedQuote,
      timestamp: Date.now()
    };

    setBridgeTransaction(transaction);

    // Mock bridge execution
    try {
      // Step 1: Pending
      await new Promise(resolve => setTimeout(resolve, 2000));
      setBridgeTransaction(prev => prev ? { ...prev, status: 'confirming', txHash: '0x1234...abcd' } : null);

      // Step 2: Confirming
      await new Promise(resolve => setTimeout(resolve, 3000));
      setBridgeTransaction(prev => prev ? { ...prev, status: 'bridging' } : null);

      // Step 3: Bridging
      await new Promise(resolve => setTimeout(resolve, 8000));
      setBridgeTransaction(prev => prev ? { 
        ...prev, 
        status: 'completed', 
        destinationTxHash: '0x5678...efgh' 
      } : null);

    } catch (error) {
      setBridgeTransaction(prev => prev ? { ...prev, status: 'failed' } : null);
    }
  };

  // Fetch quotes when parameters change
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      fetchBridgeQuotes();
    }, 500);

    return () => clearTimeout(timeoutId);
  }, [fromNetwork, toNetwork, amount, selectedToken]);

  // Reset when modal closes
  useEffect(() => {
    if (!isOpen) {
      setBridgeTransaction(null);
      setSelectedQuote(null);
      setAmount('');
    }
  }, [isOpen]);

  if (!isOpen || !selectedToken) return null;

  const compatibleNetworks = getCompatibleNetworks();

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="glass-card max-w-lg w-full max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-white border-opacity-10">
          <h3 className="text-xl font-semibold">Bridge {selectedToken.symbol}</h3>
          <button
            onClick={onClose}
            className="p-2 hover:bg-white hover:bg-opacity-10 rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Bridge Transaction Status */}
        {bridgeTransaction && (
          <div className="p-6 border-b border-white border-opacity-10">
            <div className="flex items-center gap-3 mb-4">
              {bridgeTransaction.status === 'pending' && <Loader className="animate-spin text-blue-400" size={20} />}
              {bridgeTransaction.status === 'confirming' && <Clock className="text-yellow-400" size={20} />}
              {bridgeTransaction.status === 'bridging' && <ArrowDown className="text-blue-400" size={20} />}
              {bridgeTransaction.status === 'completed' && <CheckCircle className="text-green-400" size={20} />}
              {bridgeTransaction.status === 'failed' && <AlertCircle className="text-red-400" size={20} />}
              
              <div>
                <p className="font-medium capitalize">{bridgeTransaction.status}</p>
                <p className="text-sm text-gray-400">
                  {bridgeTransaction.status === 'pending' && 'Preparing transaction...'}
                  {bridgeTransaction.status === 'confirming' && 'Confirming on source chain...'}
                  {bridgeTransaction.status === 'bridging' && 'Bridging assets...'}
                  {bridgeTransaction.status === 'completed' && 'Bridge completed successfully!'}
                  {bridgeTransaction.status === 'failed' && 'Bridge transaction failed'}
                </p>
              </div>
            </div>

            {bridgeTransaction.txHash && (
              <div className="space-y-2">
                <p className="text-sm">
                  <span className="text-gray-400">Source Tx:</span>
                  <span className="ml-2 font-mono text-blue-400">{bridgeTransaction.txHash}</span>
                </p>
                {bridgeTransaction.destinationTxHash && (
                  <p className="text-sm">
                    <span className="text-gray-400">Destination Tx:</span>
                    <span className="ml-2 font-mono text-blue-400">{bridgeTransaction.destinationTxHash}</span>
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {/* Bridge Form */}
        {!bridgeTransaction && (
          <div className="p-6 space-y-6">
            {/* From Network */}
            <div>
              <label className="block text-sm font-medium mb-2">From</label>
              <div className="flex items-center gap-3 p-3 bg-gray-800 rounded-lg">
                <img 
                  src={fromNetwork.logoUrl || '/eth-logo.png'} 
                  alt={fromNetwork.name}
                  className="w-8 h-8 rounded-full"
                />
                <div>
                  <p className="font-medium">{fromNetwork.name}</p>
                  <p className="text-sm text-gray-400">{selectedToken.symbol}</p>
                </div>
              </div>
            </div>

            {/* Arrow */}
            <div className="flex justify-center">
              <div className="p-2 bg-blue-500 bg-opacity-20 rounded-full">
                <ArrowDown size={20} className="text-blue-400" />
              </div>
            </div>

            {/* To Network */}
            <div>
              <label className="block text-sm font-medium mb-2">To</label>
              <select
                value={toNetwork.chainId}
                onChange={(e) => {
                  const network = SUPPORTED_NETWORKS.find(n => n.chainId.toString() === e.target.value);
                  if (network) setToNetwork(network);
                }}
                className="w-full p-3 bg-gray-800 rounded-lg border border-gray-700 focus:border-blue-500 focus:outline-none"
              >
                {compatibleNetworks.map(network => (
                  <option key={network.chainId} value={network.chainId}>
                    {network.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Amount */}
            <div>
              <label className="block text-sm font-medium mb-2">Amount</label>
              <div className="relative">
                <input
                  type="number"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="0.0"
                  className="w-full p-3 bg-gray-800 rounded-lg border border-gray-700 focus:border-blue-500 focus:outline-none pr-20"
                />
                <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
                  <span className="text-gray-400">{selectedToken.symbol}</span>
                </div>
              </div>
            </div>

            {/* Bridge Quotes */}
            {quotes.length > 0 && (
              <div>
                <label className="block text-sm font-medium mb-2">Bridge Route</label>
                <div className="space-y-3">
                  {quotes.map((quote, index) => (
                    <div
                      key={index}
                      onClick={() => setSelectedQuote(quote)}
                      className={`p-4 border rounded-lg cursor-pointer transition-colors ${
                        selectedQuote === quote
                          ? 'border-blue-500 bg-blue-500 bg-opacity-10'
                          : 'border-gray-700 hover:border-gray-600'
                      }`}
                    >
                      <div className="flex items-center justify-between mb-2">
                        <div className="flex items-center gap-2">
                          <span className="font-medium capitalize">{quote.protocol}</span>
                          <span className="text-xs px-2 py-1 bg-gray-700 rounded-full">
                            {quote.estimatedTime}
                          </span>
                        </div>
                        <span className="font-medium">{quote.fees.total}</span>
                      </div>
                      <div className="text-sm text-gray-400">
                        <p>Gas: {quote.fees.gas} • Bridge: {quote.fees.bridge}</p>
                        <p className="mt-1">Route: {quote.route.join(' → ')}</p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Execute Button */}
            <button
              onClick={executeBridge}
              disabled={!selectedQuote || isLoading || !amount || parseFloat(amount) <= 0}
              className="w-full btn-primary py-4 text-lg disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isLoading ? 'Loading Quotes...' : `Bridge ${amount} ${selectedToken.symbol}`}
            </button>

            {/* Bridge Info */}
            <div className="p-4 bg-blue-500 bg-opacity-10 border border-blue-500 border-opacity-20 rounded-lg">
              <div className="flex items-start gap-3">
                <AlertCircle size={16} className="text-blue-400 mt-0.5 flex-shrink-0" />
                <div className="text-sm text-blue-200">
                  <p className="font-medium mb-1">Bridge Safety</p>
                  <ul className="space-y-1 text-blue-300">
                    <li>• Transactions are irreversible</li>
                    <li>• Bridge times vary by network congestion</li>
                    <li>• Always verify destination address</li>
                    <li>• Keep transaction hashes for tracking</li>
                  </ul>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Success Actions */}
        {bridgeTransaction?.status === 'completed' && (
          <div className="p-6 border-t border-white border-opacity-10">
            <div className="flex gap-3">
              <button
                onClick={onClose}
                className="flex-1 btn-primary"
              >
                Done
              </button>
              <button
                onClick={() => {
                  // In real app, this would open block explorer
                  window.open(`${toNetwork.blockExplorer}/tx/${bridgeTransaction.destinationTxHash}`, '_blank');
                }}
                className="flex-1 btn-secondary"
              >
                View Transaction
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default BridgeModal;