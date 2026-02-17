// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState } from 'react';
import { Plus, TrendingUp, TrendingDown, ArrowLeftRight, Coins, Star, Filter, Search, Eye } from 'lucide-react';
import { Token, Network } from '../types';
import { formatTokenAmount, formatUSD } from '../utils/formatters';
import { getBridgeInfo } from '../utils/bridges';
import BridgeModal from './BridgeModal';
import { Card, Typography, Button, Flex, Badge, Reveal, Input } from '../design-system';

interface TokenListProps {
  tokens: Token[];
  onAddToken: () => void;
  onTokenSelect: (token: Token) => void;
  currentNetwork: Network;
}

const TokenList: React.FC<TokenListProps> = ({ tokens, onAddToken, onTokenSelect, currentNetwork }) => {
  const [bridgeModalOpen, setBridgeModalOpen] = useState(false);
  const [selectedTokenForBridge, setSelectedTokenForBridge] = useState<Token | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<'balance' | 'value' | 'name' | 'performance'>('value');
  const [showHideZeroBalance, setShowHideZeroBalance] = useState(false);

  const handleBridgeClick = (token: Token, e: React.MouseEvent) => {
    e.stopPropagation();
    setSelectedTokenForBridge(token);
    setBridgeModalOpen(true);
  };

  // Enhanced filtering and sorting logic
  const filteredAndSortedTokens = tokens
    .filter(token => {
      const matchesSearch = token.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
                           token.symbol.toLowerCase().includes(searchQuery.toLowerCase());
      const hasBalance = showHideZeroBalance ? parseFloat(token.balance || '0') > 0 : true;
      return matchesSearch && hasBalance;
    })
    .sort((a, b) => {
      switch (sortBy) {
        case 'balance':
          return parseFloat(b.balance || '0') - parseFloat(a.balance || '0');
        case 'value':
          return parseFloat(b.balanceUSD || '0') - parseFloat(a.balanceUSD || '0');
        case 'name':
          return a.name.localeCompare(b.name);
        case 'performance':
          return (b.priceChange24h || 0) - (a.priceChange24h || 0);
        default:
          return 0;
      }
    });

  // Portfolio summary calculations
  const totalPortfolioValue = tokens.reduce((sum, token) => sum + parseFloat(token.balanceUSD || '0'), 0);
  const totalGains24h = tokens.reduce((sum, token) => {
    const value = parseFloat(token.balanceUSD || '0');
    const change = (token.priceChange24h || 0) / 100;
    return sum + (value * change);
  }, 0);
  const totalGainsPercent = totalPortfolioValue > 0 ? (totalGains24h / totalPortfolioValue) * 100 : 0;

  return (
    <div className="space-y-6">
      {/* Professional Portfolio Overview Card */}
      <Card variant="glass" className="p-6 bg-gradient-to-br from-caesar-gold/5 via-white/5 to-imperial-purple/5 border border-white/20">
        <Flex justify="between" align="center" className="mb-6">
          <Flex align="center" gap="sm">
            <div className="relative p-3 rounded-xl bg-gradient-caesar shadow-goldGlow-sm">
              <div className="absolute inset-0 rounded-xl bg-gradient-caesar opacity-20 blur-lg"></div>
              <Coins className="relative text-background-primary" size={24} />
            </div>
            <div>
              <Typography variant="h3" gradient weight="bold" className="text-xl">
                Assets Portfolio
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                {tokens.length} tokens • {currentNetwork.name}
              </Typography>
            </div>
          </Flex>
          <Button
            variant="gold"
            size="sm"
            onClick={onAddToken}
            icon={<Plus size={18} />}
            className="shadow-goldGlow-sm hover:shadow-goldGlow-md transition-shadow"
          >
            Add Token
          </Button>
        </Flex>

        {/* Professional Portfolio Metrics */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <div className="text-center p-3 bg-white/5 rounded-xl border border-white/10">
            <Typography variant="bodySmall" color="secondary">Total Value</Typography>
            <Typography variant="h4" weight="bold" color="primary">
              {formatUSD(totalPortfolioValue)}
            </Typography>
          </div>
          <div className="text-center p-3 bg-white/5 rounded-xl border border-white/10">
            <Typography variant="bodySmall" color="secondary">24h Change</Typography>
            <Flex align="center" gap="xs" justify="center">
              {totalGainsPercent >= 0 ? 
                <TrendingUp className="text-semantic-success" size={16} /> : 
                <TrendingDown className="text-semantic-danger" size={16} />
              }
              <Typography 
                variant="bodyLarge" 
                weight="medium" 
                color={totalGainsPercent >= 0 ? 'success' : 'danger'}
              >
                {totalGainsPercent >= 0 ? '+' : ''}{totalGainsPercent.toFixed(2)}%
              </Typography>
            </Flex>
          </div>
          <div className="text-center p-3 bg-white/5 rounded-xl border border-white/10">
            <Typography variant="bodySmall" color="secondary">Assets</Typography>
            <Typography variant="h4" weight="bold" color="primary">
              {tokens.length}
            </Typography>
          </div>
          <div className="text-center p-3 bg-white/5 rounded-xl border border-white/10">
            <Typography variant="bodySmall" color="secondary">Networks</Typography>
            <Typography variant="h4" weight="bold" color="primary">
              {new Set(tokens.map(t => t.chainId)).size}
            </Typography>
          </div>
        </div>

        {/* Professional Search and Filter Controls */}
        <div className="flex flex-col sm:flex-row gap-3 mb-6">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-neutral-400" size={16} />
            <Input
              placeholder="Search tokens..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10 bg-white/5 border-white/20 focus:border-caesar-gold/50"
            />
          </div>
          <div className="flex gap-2">
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as any)}
              className="px-3 py-2 bg-white/5 border border-white/20 rounded-lg text-sm text-neutral-200 focus:border-caesar-gold/50 focus:outline-none"
            >
              <option value="value">Sort by Value</option>
              <option value="balance">Sort by Balance</option>
              <option value="name">Sort by Name</option>
              <option value="performance">Sort by Performance</option>
            </select>
            <Button
              variant={showHideZeroBalance ? 'gold' : 'ghost'}
              size="sm"
              onClick={() => setShowHideZeroBalance(!showHideZeroBalance)}
              icon={<Eye size={16} />}
              className="whitespace-nowrap"
            >
              Hide Zero
            </Button>
          </div>
        </div>
      </Card>

      {/* Enhanced Token List */}
      <Card variant="glass" className="bg-gradient-to-br from-white/5 to-white/[0.02] border border-white/10">
        <div className="space-y-2 p-1">
          {filteredAndSortedTokens.map((token, index) => {
            const priceChange = token.priceChange24h || 0;
            const isPositive = priceChange >= 0;
            const bridgeInfo = getBridgeInfo(token);
            const isCaesar = token.symbol === 'CAESAR' || token.symbol === 'CAES';
            const tokenValue = parseFloat(token.balanceUSD || '0');
            const portfolioPercentage = totalPortfolioValue > 0 ? (tokenValue / totalPortfolioValue) * 100 : 0;
            
            return (
              <Reveal key={`${token.address}-${token.chainId}`} animation="slideInLeft" delay={index * 50}>
                <Card
                  variant="outlined"
                  hover
                  className={`
                    group cursor-pointer transition-all duration-300 p-4 m-0
                    ${isCaesar 
                      ? 'border-caesar-gold/40 bg-gradient-to-r from-caesar-gold/10 to-transparent hover:border-caesar-gold/60 shadow-goldGlow-sm' 
                      : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.05] hover:border-white/20'
                    }
                  `}
                  onClick={() => onTokenSelect(token)}
                >
                  <Flex justify="between" align="center">
                    {/* Token Information - Enhanced */}
                    <Flex align="center" gap="md" className="flex-1 min-w-0">
                      <div className="relative flex-shrink-0">
                        <div className={`
                          w-12 h-12 rounded-2xl overflow-hidden bg-gradient-to-br from-white/10 to-white/5 border border-white/20
                          ${isCaesar ? 'ring-2 ring-caesar-gold/50 shadow-goldGlow-sm' : ''}
                        `}>
                          <img
                            src={token.logoURI || '/default-token.png'}
                            alt={token.symbol}
                            className="w-full h-full object-cover"
                          />
                        </div>
                        
                        {/* Enhanced Status Indicators */}
                        {bridgeInfo && (
                          <div className="absolute -bottom-1 -right-1 w-5 h-5 bg-semantic-info rounded-full flex items-center justify-center border-2 border-background-primary">
                            <ArrowLeftRight size={10} className="text-background-primary" />
                          </div>
                        )}
                        {isCaesar && (
                          <div className="absolute -top-1 -right-1 p-1 bg-caesar-gold rounded-full">
                            <Star className="text-background-primary fill-background-primary" size={12} />
                          </div>
                        )}
                        {token.lastPriceUpdate && (
                          <div className="absolute -top-1 -left-1 w-3 h-3 bg-semantic-success rounded-full border-2 border-background-primary animate-pulse"></div>
                        )}
                      </div>
                      
                      <div className="flex-1 min-w-0">
                        <Flex align="center" gap="sm" className="mb-1">
                          <Typography 
                            variant="bodyLarge" 
                            weight="semibold" 
                            color={isCaesar ? 'gold' : 'primary'}
                            className="truncate"
                          >
                            {token.symbol}
                          </Typography>
                          {bridgeInfo && (
                            <Badge variant="info" size="sm" className="flex-shrink-0">
                              {bridgeInfo.protocol}
                            </Badge>
                          )}
                          {isCaesar && (
                            <Badge variant="gold" size="sm" className="flex-shrink-0">
                              Native
                            </Badge>
                          )}
                        </Flex>
                        
                        <div className="space-y-1">
                          <Typography variant="bodySmall" color="secondary" className="truncate">
                            {token.name}
                          </Typography>
                          <Flex align="center" gap="sm">
                            {token.priceUSD && (
                              <Typography variant="caption" color="tertiary">
                                ${parseFloat(token.priceUSD).toFixed(4)}
                              </Typography>
                            )}
                            {portfolioPercentage > 0 && (
                              <Typography variant="caption" color="secondary">
                                {portfolioPercentage.toFixed(1)}% of portfolio
                              </Typography>
                            )}
                          </Flex>
                        </div>
                      </div>
                    </Flex>

                    {/* Token Metrics - Professional Layout */}
                    <div className="text-right flex-shrink-0 ml-4">
                      <Typography variant="bodyLarge" weight="semibold" className="mb-1">
                        {formatTokenAmount(token.balance || '0', token.decimals)}
                      </Typography>
                      
                      <div className="space-y-1">
                        <Typography variant="bodySmall" color="secondary">
                          {formatUSD(tokenValue)}
                        </Typography>
                        
                        <Flex align="center" gap="sm" justify="end">
                          <Flex align="center" gap="xs" className={
                            isPositive ? 'text-semantic-success' : 'text-semantic-danger'
                          }>
                            {isPositive ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                            <Typography 
                              variant="caption" 
                              className={isPositive ? 'text-semantic-success' : 'text-semantic-danger'}
                            >
                              {isPositive ? '+' : ''}{Math.abs(priceChange).toFixed(2)}%
                            </Typography>
                          </Flex>
                          
                          {(bridgeInfo || currentNetwork.features?.includes('hyperlane-endpoint') || currentNetwork.features?.includes('layerzero-endpoint')) && (
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={(e) => handleBridgeClick(token, e)}
                              icon={<ArrowLeftRight size={14} />}
                              className="opacity-0 group-hover:opacity-100 transition-opacity ml-2"
                            />
                          )}
                        </Flex>
                      </div>
                    </div>
                  </Flex>

                  {/* Professional Progress Bar for Portfolio Allocation */}
                  {portfolioPercentage > 0 && (
                    <div className="mt-3 pt-3 border-t border-white/10">
                      <div className="w-full h-1 bg-white/10 rounded-full overflow-hidden">
                        <div 
                          className={`h-full transition-all duration-500 rounded-full ${
                            isCaesar ? 'bg-caesar-gold' : 'bg-semantic-info'
                          }`}
                          style={{ width: `${Math.min(portfolioPercentage, 100)}%` }}
                        />
                      </div>
                    </div>
                  )}
                </Card>
              </Reveal>
            );
          })}

          {/* Professional Empty State */}
          {filteredAndSortedTokens.length === 0 && tokens.length > 0 && (
            <Card variant="outlined" className="text-center py-8 border-white/20 bg-white/[0.02]">
              <div className="space-y-4">
                <div className="p-4 rounded-2xl bg-neutral-800/50 mx-auto w-fit">
                  <Filter className="text-neutral-400" size={32} />
                </div>
                <div>
                  <Typography variant="bodyLarge" color="secondary" className="mb-2">
                    No tokens match your filters
                  </Typography>
                  <Typography variant="bodySmall" color="tertiary">
                    Try adjusting your search or filter criteria
                  </Typography>
                </div>
                <Button
                  variant="secondary"
                  onClick={() => {
                    setSearchQuery('');
                    setShowHideZeroBalance(false);
                  }}
                >
                  Clear Filters
                </Button>
              </div>
            </Card>
          )}

          {tokens.length === 0 && (
            <Card variant="outlined" className="text-center py-12 border-white/20 bg-gradient-to-br from-white/5 to-transparent">
              <Flex direction="col" align="center" gap="lg">
                <div className="relative">
                  <div className="p-6 rounded-3xl bg-gradient-to-br from-caesar-gold/20 to-imperial-purple/20 border border-white/20">
                    <Coins className="text-caesar-gold" size={48} />
                  </div>
                  <div className="absolute -top-2 -right-2 w-6 h-6 bg-semantic-info rounded-full flex items-center justify-center">
                    <Plus size={12} className="text-white" />
                  </div>
                </div>
                <div className="space-y-3 max-w-sm">
                  <Typography variant="h4" color="primary" className="mb-2">
                    Build Your Portfolio
                  </Typography>
                  <Typography variant="bodySmall" color="secondary">
                    Start with Caesar tokens and expand across multiple networks
                  </Typography>
                </div>
                <Button
                  variant="gold"
                  onClick={onAddToken}
                  icon={<Plus size={18} />}
                  className="shadow-goldGlow-sm hover:shadow-goldGlow-md transition-shadow"
                >
                  Add Your First Token
                </Button>
              </Flex>
            </Card>
          )}
        </div>
      </Card>

      {/* Bridge Modal */}
      <BridgeModal
        isOpen={bridgeModalOpen}
        onClose={() => setBridgeModalOpen(false)}
        selectedToken={selectedTokenForBridge || undefined}
        currentNetwork={currentNetwork}
      />
    </div>
  );
};

export default TokenList;