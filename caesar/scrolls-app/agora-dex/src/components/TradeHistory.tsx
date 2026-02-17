// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { ExternalLink } from 'lucide-react';
import { Trade } from '../types';
import { formatTokenAmount, formatTimestamp } from '../utils/formatters';

const TradeHistory: React.FC = () => {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Generate mock trade data - in real app, fetch from blockchain
    const generateMockTrades = (): Trade[] => {
      const mockTrades: Trade[] = [];
      
      for (let i = 0; i < 50; i++) {
        const timestamp = Date.now() - i * 60000; // 1 minute intervals
        const isBuy = Math.random() > 0.5;
        const amount = (Math.random() * 1000 + 10).toFixed(2);
        const price = (0.15 + (Math.random() - 0.5) * 0.02).toFixed(6);
        
        mockTrades.push({
          id: `trade_${i}`,
          timestamp: Math.floor(timestamp / 1000),
          type: isBuy ? 'buy' : 'sell',
          amount,
          price,
          total: (parseFloat(amount) * parseFloat(price)).toFixed(6),
          txHash: `0x${Math.random().toString(16).substr(2, 64)}`,
        });
      }
      
      return mockTrades.sort((a, b) => b.timestamp - a.timestamp);
    };

    setTimeout(() => {
      setTrades(generateMockTrades());
      setLoading(false);
    }, 1000);
  }, []);

  if (loading) {
    return (
      <div className="card">
        <h3 className="text-xl font-semibold mb-4">Recent Trades</h3>
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="animate-pulse bg-gray-800 h-12 rounded"></div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <h3 className="text-xl font-semibold mb-4">Recent Trades</h3>
      
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="text-gray-400 text-sm border-b border-gray-700">
              <th className="text-left py-2">Time</th>
              <th className="text-left py-2">Type</th>
              <th className="text-right py-2">Amount (CAES)</th>
              <th className="text-right py-2">Price (WETH)</th>
              <th className="text-right py-2">Total</th>
              <th className="text-center py-2">Tx</th>
            </tr>
          </thead>
          <tbody>
            {trades.slice(0, 20).map((trade) => (
              <tr key={trade.id} className="border-b border-gray-800 hover:bg-gray-800/50">
                <td className="py-3 text-sm text-gray-300">
                  {formatTimestamp(trade.timestamp)}
                </td>
                <td className="py-3">
                  <span
                    className={`px-2 py-1 rounded text-xs font-medium ${
                      trade.type === 'buy'
                        ? 'bg-green-500/20 text-green-400'
                        : 'bg-red-500/20 text-red-400'
                    }`}
                  >
                    {trade.type.toUpperCase()}
                  </span>
                </td>
                <td className="py-3 text-right font-mono text-sm">
                  {formatTokenAmount(trade.amount)}
                </td>
                <td className="py-3 text-right font-mono text-sm">
                  {parseFloat(trade.price).toFixed(6)}
                </td>
                <td className="py-3 text-right font-mono text-sm">
                  {parseFloat(trade.total).toFixed(6)} WETH
                </td>
                <td className="py-3 text-center">
                  <a
                    href={`https://sepolia.etherscan.io/tx/${trade.txHash}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-caesar-gold hover:text-yellow-500 transition-colors"
                    title="View on Etherscan"
                  >
                    <ExternalLink size={14} />
                  </a>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      
      <div className="mt-4 pt-4 border-t border-gray-700">
        <button className="text-caesar-gold hover:text-yellow-500 text-sm font-medium">
          View All Trades
        </button>
      </div>
    </div>
  );
};

export default TradeHistory;