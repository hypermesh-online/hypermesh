// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { 
  ArrowUpDown, 
  RefreshCw, 
  Settings, 
  Info, 
  AlertTriangle,
  // TrendingUp,
  Zap,
  Clock,
  TrendingDown,
  Shield,
  Target,
  Activity
} from 'lucide-react';
import { Token, Network } from '../types';
import { DEXQuote, dexAggregator } from '../utils/defiProtocols';
import { Card, Typography, Button, Flex, Grid, Badge, Animated, Reveal } from '../design-system';

interface DEXTradingProps {
  tokens: Token[];
  currentNetwork: Network;
  onSwap?: (fromToken: string, toToken: string, amount: string) => void;
}

const DEXTrading: React.FC<DEXTradingProps> = ({ tokens, currentNetwork, onSwap }) => {
  const [fromToken, setFromToken] = useState<Token | null>(tokens[0] || null);
  const [toToken, setToToken] = useState<Token | null>(tokens[1] || null);
  const [fromAmount, setFromAmount] = useState('');
  const [toAmount, setToAmount] = useState('');
  const [quotes, setQuotes] = useState<DEXQuote[]>([]);
  const [selectedQuote, setSelectedQuote] = useState<DEXQuote | null>(null);
  const [loading, setLoading] = useState(false);
  const [slippage, setSlippage] = useState(0.5);
  const [showSettings, setShowSettings] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<number>(Date.now());

  useEffect(() => {
    if (fromToken && toToken && fromAmount && parseFloat(fromAmount) > 0) {
      fetchQuotes();
    } else {
      setQuotes([]);
      setSelectedQuote(null);
      setToAmount('');
    }
  }, [fromToken, toToken, fromAmount, currentNetwork.chainId]);

  // Auto-refresh quotes every 15 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      if (fromToken && toToken && fromAmount && parseFloat(fromAmount) > 0) {
        fetchQuotes();
      }
    }, 15000);

    return () => clearInterval(interval);
  }, [fromToken, toToken, fromAmount]);

  const fetchQuotes = async () => {
    if (!fromToken || !toToken || !fromAmount) return;

    try {
      setLoading(true);
      const chainId = typeof currentNetwork.chainId === 'number' ? currentNetwork.chainId : 1;
      const newQuotes = await dexAggregator.getSwapQuote(
        fromToken.symbol,
        toToken.symbol,
        fromAmount,
        chainId
      );
      
      setQuotes(newQuotes);
      setSelectedQuote(newQuotes[0] || null);
      setToAmount(newQuotes[0]?.outputAmount || '');
      setLastRefresh(Date.now());
    } catch (error) {
      console.error('Error fetching quotes:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSwapTokens = () => {
    const tempToken = fromToken;
    const tempAmount = fromAmount;
    
    setFromToken(toToken);
    setToToken(tempToken);
    setFromAmount(toAmount);
    setToAmount(tempAmount);
  };

  const handleExecuteSwap = async () => {
    if (!selectedQuote || !fromToken || !toToken) return;

    try {
      setLoading(true);
      const txHash = await dexAggregator.executeSwap(selectedQuote, slippage);
      console.log('Swap executed:', txHash);
      onSwap?.(fromToken.symbol, toToken.symbol, fromAmount);
      
      // Reset form after successful swap
      setFromAmount('');
      setToAmount('');
      setQuotes([]);
      setSelectedQuote(null);
    } catch (error) {
      console.error('Error executing swap:', error);
    } finally {
      setLoading(false);
    }
  };

  const formatTimeAgo = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 30) return 'just now';
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes}m ago`;
  };

  const isValidAmount = fromAmount && parseFloat(fromAmount) > 0;
  const hasBalance = fromToken && fromToken.balance && parseFloat(fromToken.balance) >= parseFloat(fromAmount || '0');

  // Caesar-specific trading warnings
  const isCaesarSwap = fromToken?.symbol === 'CAESAR' || toToken?.symbol === 'CAESAR';
  const caesarTradeType = fromToken?.symbol === 'CAESAR' ? 'sell' : 'buy';

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      {/* Header */}
      <Animated animation="slideDown">
        <Flex justify="between" align="center">
          <div>
            <Typography variant="h1" gradient className="font-display">
              DEX Trading
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Instant token swaps with best price execution on {currentNetwork.name}
            </Typography>
          </div>
          <Flex align="center" gap="md">
            <Button
              variant="ghost"
              onClick={() => setShowSettings(!showSettings)}
              icon={<Settings size={18} />}
            >
              Settings
            </Button>
            <Button
              variant="secondary"
              onClick={fetchQuotes}
              loading={loading}
              icon={<RefreshCw size={18} />}
            >
              Refresh
            </Button>
          </Flex>
        </Flex>
      </Animated>

      {/* Caesar Trading Notice */}
      {isCaesarSwap && (
        <Reveal animation="fadeUp">
          <Card variant="outlined" className="p-6 border-caesar-gold/30 bg-caesar-gold/5">
            <Flex align="center" gap="md" className="mb-4">
              <TrendingDown className="text-caesar-gold" size={24} />
              <Typography variant="h3" color="gold">
                Caesar Token Trading
              </Typography>
            </Flex>
            <Typography variant="bodyLarge" color="secondary" className="mb-4">
              {caesarTradeType === 'sell' 
                ? 'Converting Caesar tokens to other assets reduces your utility participation. Consider staking or DeFi instead.'
                : 'Welcome to Caesar economics! These tokens are designed for utility, not speculation.'}
            </Typography>
            <Grid cols={3} gap="md">
              <div className="text-center">
                <TrendingDown className="text-caesar-gold mx-auto mb-2" size={20} />
                <Typography variant="bodySmall" color="secondary">
                  Demurrage Design
                </Typography>
              </div>
              <div className="text-center">
                <Activity className="text-caesar-gold mx-auto mb-2" size={20} />
                <Typography variant="bodySmall" color="secondary">
                  Utility First
                </Typography>
              </div>
              <div className="text-center">
                <Shield className="text-caesar-gold mx-auto mb-2" size={20} />
                <Typography variant="bodySmall" color="secondary">
                  Anti-Speculation
                </Typography>
              </div>
            </Grid>
          </Card>
        </Reveal>
      )}

      {/* Settings Panel */}
      {showSettings && (
        <Reveal animation="fadeUp">
          <Card variant="glass" className="p-6">
            <Typography variant="h3" className="mb-6">Trading Settings</Typography>
            <div>
              <Typography variant="bodyMedium" weight="medium" className="mb-3">
                Slippage Tolerance
              </Typography>
              <Flex gap="sm" className="mb-4">
                {[0.1, 0.5, 1.0, 3.0].map((value) => (
                  <Button
                    key={value}
                    variant={slippage === value ? 'gold' : 'ghost'}
                    onClick={() => setSlippage(value)}
                    size="sm"
                  >
                    {value}%
                  </Button>
                ))}
              </Flex>
              <input
                type="number"
                value={slippage}
                onChange={(e) => setSlippage(parseFloat(e.target.value) || 0.5)}
                className="input-field w-32"
                placeholder="Custom %"
                step="0.1"
                min="0.1"
                max="50"
              />
            </div>
          </Card>
        </Reveal>
      )}

      {/* Trading Interface */}
      <Card variant="glass" className="p-8">
        <div className="space-y-6">
          {/* From Token */}
          <div>
            <Flex justify="between" align="center" className="mb-3">
              <Typography variant="bodyMedium" color="secondary">
                From
              </Typography>
              {fromToken && fromToken.balance && (
                <Typography variant="bodySmall" color="secondary">
                  Balance: {parseFloat(fromToken.balance).toFixed(4)} {fromToken.symbol}
                </Typography>
              )}
            </Flex>
            
            <Card variant="outlined" className="p-4">
              <Flex align="center" gap="md">
                <select
                  value={fromToken?.symbol || ''}
                  onChange={(e) => {
                    const token = tokens.find(t => t.symbol === e.target.value);
                    setFromToken(token || null);
                  }}
                  className="input-field min-w-[140px]"
                >
                  <option value="">Select Token</option>
                  {tokens.map((token) => (
                    <option key={token.symbol} value={token.symbol}>
                      {token.symbol}
                    </option>
                  ))}
                </select>
                
                <input
                  type="number"
                  value={fromAmount}
                  onChange={(e) => setFromAmount(e.target.value)}
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl font-medium outline-none"
                />
                
                {fromToken && fromToken.balance && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setFromAmount(fromToken.balance || '0')}
                  >
                    MAX
                  </Button>
                )}
              </Flex>
              
              {fromToken && fromToken.priceUSD && fromAmount && (
                <Typography variant="bodySmall" color="secondary" className="mt-2">
                  ≈ ${(parseFloat(fromAmount) * parseFloat(fromToken.priceUSD)).toFixed(2)}
                </Typography>
              )}
            </Card>
          </div>

          {/* Swap Button */}
          <div className="flex justify-center">
            <Button
              variant="ghost"
              onClick={handleSwapTokens}
              icon={<ArrowUpDown size={20} />}
              className="p-4 rounded-xl"
            />
          </div>

          {/* To Token */}
          <div>
            <Flex justify="between" align="center" className="mb-3">
              <Typography variant="bodyMedium" color="secondary">
                To
              </Typography>
              {toToken && toToken.balance && (
                <Typography variant="bodySmall" color="secondary">
                  Balance: {parseFloat(toToken.balance).toFixed(4)} {toToken.symbol}
                </Typography>
              )}
            </Flex>
            
            <Card variant="outlined" className="p-4">
              <Flex align="center" gap="md">
                <select
                  value={toToken?.symbol || ''}
                  onChange={(e) => {
                    const token = tokens.find(t => t.symbol === e.target.value);
                    setToToken(token || null);
                  }}
                  className="input-field min-w-[140px]"
                >
                  <option value="">Select Token</option>
                  {tokens.map((token) => (
                    <option key={token.symbol} value={token.symbol}>
                      {token.symbol}
                    </option>
                  ))}
                </select>
                
                <input
                  type="number"
                  value={toAmount}
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl font-medium outline-none"
                  readOnly
                />
              </Flex>
              
              {toToken && toToken.priceUSD && toAmount && (
                <Typography variant="bodySmall" color="secondary" className="mt-2">
                  ≈ ${(parseFloat(toAmount) * parseFloat(toToken.priceUSD)).toFixed(2)}
                </Typography>
              )}
            </Card>
          </div>
        </div>
      </Card>

      {/* Quote Information */}
      {quotes.length > 0 && selectedQuote && (
        <Reveal animation="fadeUp">
          <Card variant="glass" className="p-6">
            <Flex justify="between" align="center" className="mb-6">
              <Typography variant="h3">Best Execution Routes</Typography>
              <Flex align="center" gap="sm">
                <Clock size={16} className="text-neutral-400" />
                <Typography variant="bodySmall" color="secondary">
                  Updated {formatTimeAgo(lastRefresh)}
                </Typography>
              </Flex>
            </Flex>
            
            <div className="space-y-3">
              {quotes.map((quote, index) => (
                <Animated key={index} animation="fadeUp" delay={index * 100}>
                  <Card
                    variant={selectedQuote?.protocol === quote.protocol ? "outlined" : "glass"}
                    className={`p-4 cursor-pointer transition-all duration-300 ${
                      selectedQuote?.protocol === quote.protocol
                        ? 'border-caesar-gold/50 bg-caesar-gold/5'
                        : 'hover:bg-white/5'
                    }`}
                    onClick={() => {
                      setSelectedQuote(quote);
                      setToAmount(quote.outputAmount);
                    }}
                  >
                    <Flex justify="between" align="center" className="mb-3">
                      <Flex align="center" gap="md">
                        <div className="p-2 rounded-lg bg-semantic-info/20">
                          <Zap className="text-semantic-info" size={16} />
                        </div>
                        <div>
                          <Typography variant="h4" weight="semibold">
                            {quote.protocol}
                          </Typography>
                          <Typography variant="bodySmall" color="secondary">
                            {index === 0 ? 'Best Rate' : `Route ${index + 1}`}
                          </Typography>
                        </div>
                      </Flex>
                      <div className="text-right">
                        <Typography variant="h4" weight="bold" color="success">
                          {parseFloat(quote.outputAmount).toFixed(4)} {toToken?.symbol}
                        </Typography>
                        {index === 0 && (
                          <Badge variant="success" className="mt-1">
                            Best
                          </Badge>
                        )}
                      </div>
                    </Flex>
                    
                    <Grid cols={3} gap="md">
                      <div>
                        <Typography variant="caption" color="secondary">
                          Price Impact
                        </Typography>
                        <Typography 
                          variant="bodyMedium" 
                          weight="medium"
                          className={parseFloat(quote.priceImpact) > 5 ? 'text-semantic-danger' : 'text-semantic-success'}
                        >
                          {quote.priceImpact}
                        </Typography>
                      </div>
                      <div>
                        <Typography variant="caption" color="secondary">
                          Fee
                        </Typography>
                        <Typography variant="bodyMedium" weight="medium">
                          {quote.fee}
                        </Typography>
                      </div>
                      <div>
                        <Typography variant="caption" color="secondary">
                          Gas Estimate
                        </Typography>
                        <Typography variant="bodyMedium" weight="medium">
                          {parseInt(quote.gasEstimate).toLocaleString()}
                        </Typography>
                      </div>
                    </Grid>
                    
                    {quote.route.length > 2 && (
                      <div className="mt-3 pt-3 border-t border-neutral-700">
                        <Typography variant="bodySmall" color="secondary">
                          Route: {quote.route.join(' → ')}
                        </Typography>
                      </div>
                    )}
                  </Card>
                </Animated>
              ))}
            </div>
          </Card>
        </Reveal>
      )}

      {/* Swap Execution */}
      <div className="space-y-4">
        <Button
          variant={!isValidAmount || !hasBalance || !selectedQuote ? "secondary" : "gold"}
          fullWidth
          size="lg"
          onClick={handleExecuteSwap}
          disabled={!isValidAmount || !hasBalance || !selectedQuote || loading}
          loading={loading}
          icon={loading ? <RefreshCw size={18} /> : <Target size={18} />}
        >
          {loading ? (
            'Executing Swap...'
          ) : !isValidAmount ? (
            'Enter Amount'
          ) : !hasBalance ? (
            'Insufficient Balance'
          ) : !selectedQuote ? (
            'Select Quote'
          ) : (
            `Swap ${fromToken?.symbol} for ${toToken?.symbol}`
          )}
        </Button>

        {/* Warning Messages */}
        {selectedQuote && parseFloat(selectedQuote.priceImpact.replace('%', '')) > 5 && (
          <Card variant="outlined" className="p-4 border-semantic-warning/30 bg-semantic-warning/5">
            <Flex align="start" gap="sm">
              <AlertTriangle className="text-semantic-warning flex-shrink-0 mt-1" size={20} />
              <div>
                <Typography variant="bodyMedium" weight="medium" color="warning">
                  High Price Impact Warning
                </Typography>
                <Typography variant="bodySmall" color="secondary" className="mt-1">
                  This swap will have a price impact of {selectedQuote.priceImpact}. 
                  Consider breaking your trade into smaller amounts.
                </Typography>
              </div>
            </Flex>
          </Card>
        )}

        {/* Information */}
        <Card variant="outlined" className="p-4 border-semantic-info/30 bg-semantic-info/5">
          <Flex align="start" gap="sm">
            <Info className="text-semantic-info flex-shrink-0 mt-1" size={20} />
            <div>
              <Typography variant="bodyMedium" weight="medium" color="info" className="mb-2">
                Trading Information
              </Typography>
              <ul className="space-y-1">
                <li>
                  <Typography variant="bodySmall" color="secondary">
                    • Quotes are automatically refreshed every 15 seconds
                  </Typography>
                </li>
                <li>
                  <Typography variant="bodySmall" color="secondary">
                    • Slippage tolerance protects against price movements
                  </Typography>
                </li>
                <li>
                  <Typography variant="bodySmall" color="secondary">
                    • Gas fees are estimated and may vary during execution
                  </Typography>
                </li>
                <li>
                  <Typography variant="bodySmall" color="secondary">
                    • Always verify transaction details before confirming
                  </Typography>
                </li>
              </ul>
            </div>
          </Flex>
        </Card>
      </div>
    </div>
  );
};

export default DEXTrading;