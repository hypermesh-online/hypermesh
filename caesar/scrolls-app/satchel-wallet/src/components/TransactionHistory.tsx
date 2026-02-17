// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { 
  ExternalLink, 
  Clock, 
  CheckCircle, 
  XCircle, 
  ArrowUpRight, 
  ArrowDownLeft, 
  RotateCcw, 
  // Filter, 
  Search,
  TrendingDown,
  Coins,
  // Activity
} from 'lucide-react';
import { Transaction, Network, WalletAccount } from '../types';
import { transactionHistory } from '../utils/transactionHistory';
import { formatDistanceToNow } from 'date-fns';
import { Card, Typography, Button, Flex, Grid, Badge, Animated, Reveal } from '../design-system';

interface TransactionHistoryProps {
  account: WalletAccount;
  currentNetwork: Network;
  onTransactionClick?: (transaction: Transaction) => void;
}

const TransactionHistory: React.FC<TransactionHistoryProps> = ({
  account,
  currentNetwork,
  onTransactionClick
}) => {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'sent' | 'received' | 'swap'>('all');
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedStatus, setSelectedStatus] = useState<'all' | 'confirmed' | 'pending' | 'failed'>('all');

  useEffect(() => {
    loadTransactions();
  }, [account.address, currentNetwork.chainId]);

  const loadTransactions = async () => {
    if (!account.address || !account.isConnected) {
      setTransactions([]);
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError(null);
      
      const txs = await transactionHistory.getTransactionHistory(account.address, currentNetwork);
      
      // Determine transaction direction and enhance data
      const enhancedTxs = txs.map(tx => ({
        ...tx,
        direction: determineDirection(tx, account.address)
      }));

      setTransactions(enhancedTxs);
    } catch (err) {
      console.error('Error loading transactions:', err);
      setError('Failed to load transaction history');
    } finally {
      setLoading(false);
    }
  };

  const determineDirection = (tx: Transaction, userAddress: string): 'sent' | 'received' | 'swap' => {
    const userAddr = userAddress.toLowerCase();
    const fromAddr = tx.from.toLowerCase();
    const toAddr = tx.to.toLowerCase();

    // Check if it's a swap (DEX interaction)
    if (tx.method?.includes('swap') || tx.to.toLowerCase().includes('dex') || tx.type === 'bridge') {
      return 'swap';
    }

    // Check direction
    if (fromAddr === userAddr && toAddr !== userAddr) {
      return 'sent';
    } else if (toAddr === userAddr && fromAddr !== userAddr) {
      return 'received';
    } else {
      return 'swap'; // Self-transaction or contract interaction
    }
  };

  const formatValue = (value: string, decimals: number = 18): string => {
    try {
      const bigValue = BigInt(value);
      const divisor = BigInt(10 ** decimals);
      const integerPart = bigValue / divisor;
      const fractionalPart = bigValue % divisor;
      
      if (fractionalPart === 0n) {
        return integerPart.toString();
      }
      
      const fractionalStr = fractionalPart.toString().padStart(decimals, '0');
      const trimmedFractional = fractionalStr.replace(/0+$/, '');
      
      if (trimmedFractional === '') {
        return integerPart.toString();
      }
      
      return `${integerPart.toString()}.${trimmedFractional}`;
    } catch (error) {
      console.warn('Error formatting value:', error);
      return '0';
    }
  };

  const getTransactionIcon = (direction: string, status: string) => {
    if (status === 'pending') {
      return <Clock className="text-semantic-warning" size={20} />;
    }
    
    if (status === 'failed') {
      return <XCircle className="text-semantic-danger" size={20} />;
    }

    switch (direction) {
      case 'sent':
        return <ArrowUpRight className="text-semantic-danger" size={20} />;
      case 'received':
        return <ArrowDownLeft className="text-semantic-success" size={20} />;
      case 'swap':
        return <RotateCcw className="text-semantic-info" size={20} />;
      default:
        return <CheckCircle className="text-semantic-success" size={20} />;
    }
  };

  const getStatusBadgeVariant = (status: string) => {
    switch (status) {
      case 'confirmed':
        return 'success';
      case 'pending':
        return 'warning';
      case 'failed':
        return 'danger';
      default:
        return 'default';
    }
  };

  const getDirectionText = (direction: string) => {
    switch (direction) {
      case 'sent':
        return 'Sent';
      case 'received':
        return 'Received';
      case 'swap':
        return 'Swap/Bridge';
      default:
        return 'Transaction';
    }
  };

  const filteredTransactions = transactions.filter(tx => {
    const matchesFilter = filter === 'all' || tx.direction === filter;
    const matchesStatus = selectedStatus === 'all' || tx.status === selectedStatus;
    const matchesSearch = searchTerm === '' || 
      tx.hash.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.from.toLowerCase().includes(searchTerm.toLowerCase()) ||
      tx.to.toLowerCase().includes(searchTerm.toLowerCase()) ||
      (tx.tokenSymbol && tx.tokenSymbol.toLowerCase().includes(searchTerm.toLowerCase()));

    return matchesFilter && matchesStatus && matchesSearch;
  });

  const openInExplorer = (hash: string) => {
    if (currentNetwork.blockExplorer) {
      const url = `${currentNetwork.blockExplorer}/tx/${hash}`;
      window.open(url, '_blank');
    }
  };

  // Calculate demurrage impact for CAESAR transactions
  const calculateDemurrageImpact = (tx: any) => {
    if (tx.tokenSymbol === 'CAESAR' && tx.direction === 'received') {
      const days = Math.floor((Date.now() - new Date(tx.timestamp).getTime()) / (1000 * 60 * 60 * 24));
      const demurrageRate = 0.05 / 365; // 5% annual demurrage
      const impact = parseFloat(tx.value) * demurrageRate * days;
      return impact;
    }
    return 0;
  };

  if (loading) {
    return (
      <Card variant="glass" className="p-8">
        <Flex justify="between" align="center" className="mb-6">
          <Typography variant="h2" gradient className="font-display">
            Transaction History
          </Typography>
        </Flex>
        <div className="flex items-center justify-center py-16">
          <div className="loading-spinner w-8 h-8"></div>
          <Typography variant="bodyLarge" color="secondary" className="ml-4">
            Loading transaction history...
          </Typography>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card variant="glass" className="p-8">
        <Flex justify="between" align="center" className="mb-6">
          <Typography variant="h2" gradient className="font-display">
            Transaction History
          </Typography>
          <Button 
            variant="secondary"
            onClick={loadTransactions}
            icon={<RotateCcw size={18} />}
          >
            Retry
          </Button>
        </Flex>
        <div className="text-center py-16">
          <XCircle className="mx-auto mb-4 text-semantic-danger" size={64} />
          <Typography variant="h3" color="danger" className="mb-4">
            {error}
          </Typography>
          <Button variant="primary" onClick={loadTransactions}>
            Try Again
          </Button>
        </div>
      </Card>
    );
  }

  return (
    <div className="max-w-6xl mx-auto space-y-8">
      {/* Header */}
      <Animated animation="slideDown">
        <Flex justify="between" align="center">
          <div>
            <Typography variant="h1" gradient className="font-display">
              Transaction History
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Your complete Caesar token transaction activity on {currentNetwork.name}
            </Typography>
          </div>
          <Button 
            variant="secondary"
            onClick={loadTransactions}
            loading={loading}
            icon={<RotateCcw size={18} />}
          >
            Refresh
          </Button>
        </Flex>
      </Animated>

      {/* Caesar Economic Insight */}
      <Reveal animation="fadeUp">
        <Card variant="outlined" className="p-6 border-caesar-gold/30 bg-caesar-gold/5">
          <Flex align="center" gap="md" className="mb-4">
            <TrendingDown className="text-caesar-gold" size={24} />
            <Typography variant="h3" color="gold">
              Caesar Economic Activity
            </Typography>
          </Flex>
          <Grid cols={3} gap="lg" responsive={{ md: 1, lg: 3 }}>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {filteredTransactions.filter(tx => tx.tokenSymbol === 'CAESAR').length}
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Caesar Transactions
              </Typography>
            </div>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {filteredTransactions.filter(tx => tx.direction === 'sent' && tx.tokenSymbol === 'CAESAR').length}
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Utility Payments
              </Typography>
            </div>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {(filteredTransactions.reduce((acc, tx) => {
                  return acc + calculateDemurrageImpact(tx);
                }, 0)).toFixed(4)}
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Demurrage Saved (CAESAR)
              </Typography>
            </div>
          </Grid>
        </Card>
      </Reveal>

      {/* Filters */}
      <Reveal animation="fadeUp" delay={100}>
        <Card variant="glass" className="p-6">
          <Grid cols={4} gap="lg" responsive={{ md: 1, lg: 4 }}>
            {/* Search */}
            <div className="lg:col-span-2">
              <Typography variant="bodyLarge" weight="medium" className="mb-3">
                Search Transactions
              </Typography>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-neutral-400" size={18} />
                <input
                  type="text"
                  placeholder="Search by hash, address, or token..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="input-field pl-10"
                />
              </div>
            </div>

            {/* Type Filter */}
            <div>
              <Typography variant="bodyLarge" weight="medium" className="mb-3">
                Transaction Type
              </Typography>
              <select
                value={filter}
                onChange={(e) => setFilter(e.target.value as any)}
                className="input-field"
              >
                <option value="all">All Types</option>
                <option value="sent">Sent</option>
                <option value="received">Received</option>
                <option value="swap">Swaps/Bridges</option>
              </select>
            </div>

            {/* Status Filter */}
            <div>
              <Typography variant="bodyLarge" weight="medium" className="mb-3">
                Status
              </Typography>
              <select
                value={selectedStatus}
                onChange={(e) => setSelectedStatus(e.target.value as any)}
                className="input-field"
              >
                <option value="all">All Status</option>
                <option value="confirmed">Confirmed</option>
                <option value="pending">Pending</option>
                <option value="failed">Failed</option>
              </select>
            </div>
          </Grid>
        </Card>
      </Reveal>

      {/* Transaction List */}
      {filteredTransactions.length === 0 ? (
        <Card variant="glass" className="p-12 text-center">
          <Coins className="mx-auto mb-6 text-neutral-400" size={64} />
          <Typography variant="h3" color="secondary" className="mb-4">
            {transactions.length === 0 ? 'No transactions found' : 'No transactions match your filters'}
          </Typography>
          {searchTerm && (
            <Button 
              variant="ghost"
              onClick={() => setSearchTerm('')}
            >
              Clear search
            </Button>
          )}
        </Card>
      ) : (
        <div className="space-y-4">
          {filteredTransactions.map((tx, index) => (
            <Reveal key={`${tx.hash}-${index}`} animation="slideInLeft" delay={index * 50}>
              <Card
                variant="glass" 
                hover
                className="p-6 cursor-pointer transition-all duration-300"
                onClick={() => onTransactionClick?.(tx)}
              >
                <Flex justify="between" align="center">
                  <Flex align="center" gap="lg">
                    {/* Transaction Icon */}
                    <div className="flex-shrink-0 p-3 rounded-xl bg-neutral-800/50">
                      {getTransactionIcon(tx.direction || 'swap', tx.status)}
                    </div>
                    
                    {/* Transaction Details */}
                    <div className="flex-1 min-w-0">
                      <Flex align="center" gap="md" className="mb-2">
                        <Typography variant="h4" weight="semibold">
                          {getDirectionText(tx.direction || 'swap')}
                        </Typography>
                        <Badge variant={getStatusBadgeVariant(tx.status)}>
                          {tx.status}
                        </Badge>
                        {tx.tokenSymbol === 'CAESAR' && (
                          <Badge variant="gold">
                            <Coins size={14} className="mr-1" />
                            Caesar
                          </Badge>
                        )}
                      </Flex>
                      
                      <div className="space-y-1">
                        <Flex align="center" gap="md" className="text-sm">
                          <Typography variant="bodySmall" color="secondary">
                            Hash:
                          </Typography>
                          <Typography variant="bodySmall" family="mono">
                            {tx.hash.slice(0, 12)}...{tx.hash.slice(-10)}
                          </Typography>
                        </Flex>
                        
                        <Flex align="center" gap="md" className="text-sm">
                          <Typography variant="bodySmall" color="secondary">
                            {tx.direction === 'sent' ? 'To:' : 'From:'}
                          </Typography>
                          <Typography variant="bodySmall" family="mono">
                            {(tx.direction === 'sent' ? tx.to : tx.from).slice(0, 10)}...
                            {(tx.direction === 'sent' ? tx.to : tx.from).slice(-8)}
                          </Typography>
                        </Flex>
                        
                        <Typography variant="bodySmall" color="secondary">
                          {formatDistanceToNow(new Date(tx.timestamp), { addSuffix: true })}
                        </Typography>
                      </div>
                    </div>
                  </Flex>

                  {/* Amount and Actions */}
                  <Flex align="center" gap="lg">
                    <div className="text-right">
                      <Typography variant="h4" weight="bold" className={
                        tx.direction === 'sent' ? 'text-semantic-danger' : 'text-semantic-success'
                      }>
                        {tx.direction === 'sent' ? '-' : '+'}
                        {formatValue(tx.value)} {tx.tokenSymbol}
                      </Typography>
                      {tx.gasUsed && (
                        <Typography variant="bodySmall" color="secondary">
                          Gas: {formatValue(tx.gasUsed, 9)} Gwei
                        </Typography>
                      )}
                      {tx.tokenSymbol === 'CAESAR' && calculateDemurrageImpact(tx) > 0 && (
                        <Typography variant="caption" color="warning">
                          Demurrage: -{calculateDemurrageImpact(tx).toFixed(4)} CAESAR
                        </Typography>
                      )}
                    </div>
                    
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        openInExplorer(tx.hash);
                      }}
                      icon={<ExternalLink size={16} />}
                    />
                  </Flex>
                </Flex>
              </Card>
            </Reveal>
          ))}
          
          {filteredTransactions.length >= 50 && (
            <div className="text-center py-8">
              <Button 
                variant="secondary"
                onClick={loadTransactions}
              >
                Load more transactions
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default TransactionHistory;