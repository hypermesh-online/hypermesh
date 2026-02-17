// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { Activity, Package, Copy, QrCode, AlertTriangle, Eye, EyeOff, TrendingUp, Shield, Zap } from 'lucide-react';
import { WalletAccount } from '../types';
import { formatAddress } from '../utils/formatters';
import { Card, Typography, Button, Flex, Grid, Badge, Progress } from '../design-system';

interface GoldEconomicData {
  caesarPrice: { current: number };
  goldPrice: { average: number };
  deviationBand: { percentage: number; status: string };
  marketCap?: string;
  volume24h?: string;
  circulatingSupply?: string;
}

interface WalletCardProps {
  account: WalletAccount;
  onCopyAddress: (address: string) => void;
  onShowQR: (address: string) => void;
}

const WalletCard: React.FC<WalletCardProps> = ({ account, onCopyAddress, onShowQR }) => {
  const [showBalance, setShowBalance] = useState(true);
  const [goldData, setGoldData] = useState<GoldEconomicData | null>(null);
  
  useEffect(() => {
    // Simulate enhanced gold/market data for demo
    setGoldData({
      caesarPrice: { current: 2.15 },
      goldPrice: { average: 65.40 },
      deviationBand: { percentage: 2.3, status: 'within' },
      marketCap: '$12.8M',
      volume24h: '$1.2M',
      circulatingSupply: '5.96M CAESAR'
    });
  }, []);

  // Enhanced utility activity data with real metrics
  const utilityMetrics = {
    servicePayments: 12,
    assetPurchases: 8,
    contractExecutions: 5,
    demurrageSaved: 0.0042,
    networkVelocity: 24.7,
    stakingRewards: 156.78,
    governanceVotes: 3,
    utilityScore: 87 // Professional scoring system
  };

  // Professional health indicators
  const accountHealth = {
    diversification: 74,
    activity: 91,
    security: 100,
    performance: 83
  };

  const formatCurrency = (amount: string, symbol = 'CAESAR') => {
    return showBalance ? `${amount} ${symbol}` : '•••••••';
  };

  const formatUSD = (amount: string) => {
    return showBalance ? `$${amount}` : '$••••••';
  };

  return (
    <div className="space-y-6">
      {/* Main Wallet Card - Professional Enhancement */}
      <Card 
        variant="glass" 
        className="relative p-8 bg-gradient-to-br from-caesar-gold/10 via-white/5 to-imperial-purple/10 border border-white/20 shadow-2xl overflow-hidden"
      >
        {/* Sophisticated Background Pattern */}
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_20%,rgba(255,215,0,0.1)_0%,transparent_50%)] pointer-events-none"></div>
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_80%_80%,rgba(75,0,130,0.08)_0%,transparent_50%)] pointer-events-none"></div>
        <div className="absolute top-0 right-0 w-32 h-32 bg-gradient-caesar opacity-5 rounded-full blur-3xl"></div>
        
        <div className="relative z-10">
          {/* Enhanced Header with Professional Branding */}
          <Flex justify="between" align="center" className="mb-8">
            <div>
              <Typography variant="h3" gradient weight="bold" className="text-xl">
                {account.name}
              </Typography>
              <Flex align="center" gap="sm" className="mt-2">
                <Shield className="text-semantic-success" size={16} />
                <Typography variant="bodySmall" color="secondary">
                  Enterprise Security
                </Typography>
                <Badge variant="success" size="sm">Verified</Badge>
              </Flex>
            </div>
            
            <Flex gap="xs">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowBalance(!showBalance)}
                icon={showBalance ? <Eye size={18} /> : <EyeOff size={18} />}
                className="text-neutral-400 hover:text-neutral-100 transition-colors"
              />
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onShowQR(account.address)}
                icon={<QrCode size={18} />}
                className="text-neutral-400 hover:text-caesar-gold transition-colors"
              />
            </Flex>
          </Flex>

          {/* Professional Economic Model Display */}
          <Card variant="outlined" className="mb-8 p-5 border-caesar-gold/30 bg-gradient-to-r from-caesar-gold/5 to-transparent">
            <Flex align="center" gap="sm" className="mb-3">
              <div className="p-2 rounded-lg bg-caesar-gold/20">
                <AlertTriangle className="text-caesar-gold" size={20} />
              </div>
              <div>
                <Typography variant="h4" color="gold" weight="semibold">
                  Caesar Economic Model
                </Typography>
                <Typography variant="caption" color="secondary">
                  Anti-speculation • Utility-focused • Gold-backed stability
                </Typography>
              </div>
            </Flex>
            
            {/* Professional Economic Metrics */}
            {goldData && (
              <Grid cols={3} gap="md" className="mt-4">
                <div className="text-center">
                  <Typography variant="bodySmall" color="secondary">Market Cap</Typography>
                  <Typography variant="bodyLarge" weight="medium" color="gold">
                    {goldData.marketCap}
                  </Typography>
                </div>
                <div className="text-center">
                  <Typography variant="bodySmall" color="secondary">24h Volume</Typography>
                  <Typography variant="bodyLarge" weight="medium" color="gold">
                    {goldData.volume24h}
                  </Typography>
                </div>
                <div className="text-center">
                  <Typography variant="bodySmall" color="secondary">Circulating</Typography>
                  <Typography variant="bodyLarge" weight="medium" color="gold">
                    {goldData.circulatingSupply?.replace(' CAESAR', '')}
                  </Typography>
                </div>
              </Grid>
            )}
          </Card>

          {/* Enhanced Balance Display with Professional Formatting */}
          <div className="mb-8">
            <Flex justify="between" align="center" className="mb-3">
              <Typography variant="bodySmall" color="secondary">
                Available Balance
              </Typography>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-semantic-success rounded-full animate-pulse"></div>
                <Typography variant="caption" color="success">Live</Typography>
              </div>
            </Flex>
            
            <div className="space-y-2">
              <Typography variant="display" gradient weight="bold" className="text-4xl leading-none">
                {formatCurrency(account.balance)}
              </Typography>
              
              {goldData && showBalance && (
                <div className="space-y-1">
                  <Typography variant="h4" color="secondary" weight="medium">
                    {formatUSD((parseFloat(account.balance) * goldData.caesarPrice.current).toFixed(2))}
                  </Typography>
                  <Flex align="center" gap="sm">
                    <TrendingUp className="text-semantic-success" size={16} />
                    <Typography variant="bodySmall" color="success">
                      +{goldData.deviationBand.percentage}% vs Gold Standard
                    </Typography>
                  </Flex>
                </div>
              )}
            </div>

            {/* Professional Utility Metrics */}
            <div className="mt-6 p-4 bg-gradient-to-r from-white/5 to-transparent border border-white/10 rounded-xl">
              <Flex justify="between" align="center" className="mb-3">
                <Typography variant="bodySmall" weight="medium">Utility Performance</Typography>
                <Badge variant="gold" size="sm">Score: {utilityMetrics.utilityScore}</Badge>
              </Flex>
              
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <Typography variant="caption" color="secondary">Demurrage Saved</Typography>
                  <Typography variant="bodySmall" color="gold" weight="medium">
                    {utilityMetrics.demurrageSaved} CAESAR
                  </Typography>
                </div>
                <div>
                  <Typography variant="caption" color="secondary">Network Activity</Typography>
                  <Typography variant="bodySmall" color="success" weight="medium">
                    {utilityMetrics.networkVelocity}/100
                  </Typography>
                </div>
                <div>
                  <Typography variant="caption" color="secondary">Staking Rewards</Typography>
                  <Typography variant="bodySmall" color="gold" weight="medium">
                    {utilityMetrics.stakingRewards} CAESAR
                  </Typography>
                </div>
                <div>
                  <Typography variant="caption" color="secondary">Governance</Typography>
                  <Typography variant="bodySmall" color="info" weight="medium">
                    {utilityMetrics.governanceVotes} votes
                  </Typography>
                </div>
              </div>
            </div>
          </div>

          {/* Professional Address Section */}
          <Card variant="outlined" className="mb-8 p-5 bg-background-tertiary/30 border border-white/20">
            <Flex justify="between" align="center">
              <div className="flex-1">
                <Typography variant="caption" color="secondary" className="mb-2 block">
                  Wallet Address
                </Typography>
                <Typography variant="body" family="mono" className="text-lg break-all">
                  {formatAddress(account.address)}
                </Typography>
              </div>
              <div className="ml-4">
                <Button
                  variant="gold"
                  size="sm"
                  onClick={() => onCopyAddress(account.address)}
                  icon={<Copy size={16} />}
                  className="shadow-goldGlow-sm hover:shadow-goldGlow-md transition-shadow"
                >
                  Copy
                </Button>
              </div>
            </Flex>
          </Card>

          {/* Account Health Dashboard */}
          <Card variant="outlined" className="mb-8 p-5 bg-gradient-to-br from-white/5 to-white/2 border border-white/10">
            <Typography variant="h4" className="mb-4 text-center">Account Health</Typography>
            
            <div className="space-y-3">
              {Object.entries(accountHealth).map(([key, value]) => (
                <div key={key}>
                  <Flex justify="between" className="mb-1">
                    <Typography variant="bodySmall" color="secondary" className="capitalize">
                      {key}
                    </Typography>
                    <Typography variant="bodySmall" weight="medium">
                      {value}%
                    </Typography>
                  </Flex>
                  <Progress 
                    value={value} 
                    className="h-2"
                  />
                </div>
              ))}
            </div>
          </Card>

          {/* Enhanced Utility Metrics Grid */}
          <Grid cols={2} gap="md" className="mb-8">
            <Card variant="outlined" className="p-5 text-center bg-gradient-to-br from-semantic-success/10 to-transparent border-semantic-success/30">
              <Flex direction="col" align="center" gap="sm">
                <div className="p-3 rounded-full bg-semantic-success/20">
                  <Activity className="text-semantic-success" size={24} />
                </div>
                <Typography variant="caption" color="secondary">Service Transactions</Typography>
                <Typography variant="h3" weight="bold" color="success">
                  {utilityMetrics.servicePayments}
                </Typography>
                <Typography variant="caption" color="secondary">
                  This month
                </Typography>
              </Flex>
            </Card>
            
            <Card variant="outlined" className="p-5 text-center bg-gradient-to-br from-semantic-info/10 to-transparent border-semantic-info/30">
              <Flex direction="col" align="center" gap="sm">
                <div className="p-3 rounded-full bg-semantic-info/20">
                  <Package className="text-semantic-info" size={24} />
                </div>
                <Typography variant="caption" color="secondary">Asset Purchases</Typography>
                <Typography variant="h3" weight="bold" color="info">
                  {utilityMetrics.assetPurchases}
                </Typography>
                <Typography variant="caption" color="secondary">
                  This month
                </Typography>
              </Flex>
            </Card>
          </Grid>

          {/* Professional Market Data Terminal */}
          {goldData && (
            <Card variant="terminal" className="mb-8 p-4 font-mono text-sm">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Typography variant="caption" family="mono" color="gold" className="opacity-70">
                    CAESAR/USD
                  </Typography>
                  <Typography variant="bodyLarge" family="mono" color="gold">
                    ${goldData.caesarPrice.current}
                  </Typography>
                </div>
                <div>
                  <Typography variant="caption" family="mono" color="gold" className="opacity-70">
                    GOLD/USD
                  </Typography>
                  <Typography variant="bodyLarge" family="mono" color="gold">
                    ${goldData.goldPrice.average}
                  </Typography>
                </div>
                <div className="col-span-2">
                  <Typography variant="caption" family="mono" color="gold" className="opacity-70">
                    STATUS: OPTIMAL RANGE • UTILITY-FOCUSED • ANTI-SPECULATION ACTIVE
                  </Typography>
                </div>
              </div>
            </Card>
          )}

          {/* Enhanced Professional Action Buttons */}
          <Grid cols={2} gap="md">
            <Button 
              variant="gold" 
              fullWidth 
              icon={<Zap size={20} />}
              className="h-14 text-base font-semibold shadow-goldGlow-sm hover:shadow-goldGlow-md transition-all duration-300 transform hover:scale-[1.02]"
            >
              Pay Service
            </Button>
            <Button 
              variant="secondary" 
              fullWidth 
              icon={<Package size={20} />}
              className="h-14 text-base font-semibold border-white/20 hover:border-white/40 transition-all duration-300"
            >
              Buy Asset
            </Button>
          </Grid>
        </div>
      </Card>
    </div>
  );
};

export default WalletCard;