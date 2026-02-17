// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { 
  TrendingUp, 
  TrendingDown, 
  DollarSign, 
  Percent, 
  ArrowUpRight, 
  ArrowDownLeft,
  Zap,
  Shield,
  AlertTriangle,
  ExternalLink,
  RefreshCw,
  Coins,
  Target,
  Clock,
  Users
} from 'lucide-react';
import { WalletAccount, Network } from '../types';
import { 
  DeFiPosition, 
  DeFiProtocol, 
  portfolioManager, 
  getProtocolsByNetwork,
  getProtocolsByType 
} from '../utils/defiProtocols';
import { Card, Typography, Button, Flex, Grid, Badge, Animated, Reveal } from '../design-system';

interface DeFiDashboardProps {
  account: WalletAccount;
  currentNetwork: Network;
}

const DeFiDashboard: React.FC<DeFiDashboardProps> = ({ account, currentNetwork }) => {
  const [positions, setPositions] = useState<DeFiPosition[]>([]);
  const [protocols, setProtocols] = useState<DeFiProtocol[]>([]);
  const [totalValueLocked, setTotalValueLocked] = useState('$0.00');
  const [averageAPY, setAverageAPY] = useState('0%');
  const [loading, setLoading] = useState(true);
  const [selectedTab, setSelectedTab] = useState<'overview' | 'positions' | 'protocols' | 'farming'>('overview');

  useEffect(() => {
    loadDeFiData();
  }, [account.address, currentNetwork.chainId]);

  const loadDeFiData = async () => {
    if (!account.address) {
      setPositions([]);
      setTotalValueLocked('$0.00');
      setAverageAPY('0%');
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      
      // Load user positions
      const userPositions = await portfolioManager.getPositions(
        account.address, 
        typeof currentNetwork.chainId === 'number' ? currentNetwork.chainId : 1
      );
      
      // Load available protocols for current network
      const networkProtocols = getProtocolsByNetwork(
        typeof currentNetwork.chainId === 'number' ? currentNetwork.chainId : 1
      );
      
      // Calculate portfolio metrics
      const tvl = await portfolioManager.getTotalValueLocked(account.address);
      const avgAPY = await portfolioManager.getAverageAPY(account.address);
      
      setPositions(userPositions);
      setProtocols(networkProtocols);
      setTotalValueLocked(tvl);
      setAverageAPY(avgAPY);
    } catch (error) {
      console.error('Error loading DeFi data:', error);
    } finally {
      setLoading(false);
    }
  };

  const getRiskBadgeVariant = (level: string) => {
    switch (level) {
      case 'low': return 'success';
      case 'medium': return 'warning';
      case 'high': return 'danger';
      default: return 'default';
    }
  };

  const getRiskIcon = (level: string) => {
    switch (level) {
      case 'low': return <Shield size={16} />;
      case 'medium': return <AlertTriangle size={16} />;
      case 'high': return <AlertTriangle size={16} />;
      default: return <Shield size={16} />;
    }
  };

  const getPositionIcon = (type: string) => {
    switch (type) {
      case 'lending': return <ArrowUpRight className="text-semantic-success" size={20} />;
      case 'borrowing': return <ArrowDownLeft className="text-semantic-danger" size={20} />;
      case 'staking': return <Zap className="text-imperial-purple" size={20} />;
      case 'liquidity': return <TrendingUp className="text-semantic-info" size={20} />;
      case 'farming': return <Target className="text-caesar-gold" size={20} />;
      default: return <DollarSign className="text-neutral-400" size={20} />;
    }
  };

  // Calculate Caesar-specific DeFi insights
  const caesarDeFiMetrics = {
    utilityStaking: positions.filter(p => p.tokenSymbol === 'CAESAR' && p.type === 'staking').length,
    demurrageAvoided: positions
      .filter(p => p.tokenSymbol === 'CAESAR')
      .reduce((acc, p) => acc + (parseFloat(p.amount.replace(/[^\d.-]/g, '')) * 0.05 / 365 * 30), 0), // 30 days
    economicVelocity: positions.filter(p => p.type === 'farming' || p.type === 'liquidity').length
  };

  if (loading) {
    return (
      <Card variant="glass" className="p-8">
        <Flex justify="between" align="center" className="mb-6">
          <Typography variant="h2" gradient className="font-display">
            DeFi Dashboard
          </Typography>
        </Flex>
        <div className="flex items-center justify-center py-16">
          <div className="loading-spinner w-8 h-8"></div>
          <Typography variant="bodyLarge" color="secondary" className="ml-4">
            Loading DeFi portfolio...
          </Typography>
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
              DeFi Dashboard
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Caesar token ecosystem DeFi positions and opportunities on {currentNetwork.name}
            </Typography>
          </div>
          <Button 
            variant="secondary"
            onClick={loadDeFiData}
            loading={loading}
            icon={<RefreshCw size={18} />}
          >
            Refresh
          </Button>
        </Flex>
      </Animated>

      {/* Caesar DeFi Economic Insights */}
      <Reveal animation="fadeUp">
        <Card variant="outlined" className="p-6 border-caesar-gold/30 bg-caesar-gold/5">
          <Flex align="center" gap="md" className="mb-6">
            <TrendingDown className="text-caesar-gold" size={24} />
            <Typography variant="h3" color="gold">
              Caesar DeFi Economics
            </Typography>
          </Flex>
          <Grid cols={4} gap="lg" responsive={{ md: 2, lg: 4 }}>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {caesarDeFiMetrics.utilityStaking}
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Utility Staking
              </Typography>
            </div>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {caesarDeFiMetrics.demurrageAvoided.toFixed(4)}
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Demurrage Avoided (CAESAR)
              </Typography>
            </div>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {caesarDeFiMetrics.economicVelocity}
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Economic Velocity
              </Typography>
            </div>
            <div className="text-center">
              <Typography variant="h2" gradient weight="bold">
                {((caesarDeFiMetrics.utilityStaking + caesarDeFiMetrics.economicVelocity) * 2.5).toFixed(1)}%
              </Typography>
              <Typography variant="bodySmall" color="secondary">
                Anti-Speculation Score
              </Typography>
            </div>
          </Grid>
        </Card>
      </Reveal>

      {/* Portfolio Overview Cards */}
      <Grid cols={3} gap="lg" responsive={{ md: 1, lg: 3 }}>
        <Reveal animation="slideInLeft">
          <Card variant="glass" className="p-6 text-center">
            <div className="p-3 rounded-xl bg-semantic-success/20 mx-auto w-16 h-16 flex items-center justify-center mb-4">
              <DollarSign className="text-semantic-success" size={28} />
            </div>
            <Typography variant="h2" weight="bold" gradient className="mb-2">
              {totalValueLocked}
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Total Value Locked
            </Typography>
          </Card>
        </Reveal>

        <Reveal animation="slideInLeft" delay={100}>
          <Card variant="glass" className="p-6 text-center">
            <div className="p-3 rounded-xl bg-semantic-info/20 mx-auto w-16 h-16 flex items-center justify-center mb-4">
              <Percent className="text-semantic-info" size={28} />
            </div>
            <Typography variant="h2" weight="bold" gradient className="mb-2">
              {averageAPY}
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Average APY
            </Typography>
          </Card>
        </Reveal>

        <Reveal animation="slideInLeft" delay={200}>
          <Card variant="glass" className="p-6 text-center">
            <div className="p-3 rounded-xl bg-imperial-purple/20 mx-auto w-16 h-16 flex items-center justify-center mb-4">
              <Target className="text-imperial-purple" size={28} />
            </div>
            <Typography variant="h2" weight="bold" gradient className="mb-2">
              {positions.length}
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Active Positions
            </Typography>
          </Card>
        </Reveal>
      </Grid>

      {/* Tab Navigation */}
      <Card variant="glass" className="p-2">
        <Flex gap="xs">
          {[
            { id: 'overview', label: 'Overview', icon: <TrendingUp size={16} /> },
            { id: 'positions', label: 'Positions', icon: <Coins size={16} /> },
            { id: 'protocols', label: 'Protocols', icon: <Shield size={16} /> },
            { id: 'farming', label: 'Yield Farming', icon: <Target size={16} /> }
          ].map((tab) => (
            <Button
              key={tab.id}
              variant={selectedTab === tab.id ? 'gold' : 'ghost'}
              onClick={() => setSelectedTab(tab.id as any)}
              icon={tab.icon}
              className="flex-1"
            >
              {tab.label}
            </Button>
          ))}
        </Flex>
      </Card>

      {/* Tab Content */}
      {selectedTab === 'overview' && (
        <Animated animation="fadeIn">
          {positions.length === 0 ? (
            <Card variant="glass" className="p-12 text-center">
              <Coins className="mx-auto mb-6 text-neutral-400" size={64} />
              <Typography variant="h3" color="secondary" className="mb-4">
                No DeFi positions found
              </Typography>
              <Typography variant="bodyLarge" color="secondary" className="mb-8">
                Start earning yield on your Caesar tokens with our recommended protocols
              </Typography>
              <Button 
                variant="primary"
                onClick={() => setSelectedTab('protocols')}
                icon={<ExternalLink size={18} />}
              >
                Explore Protocols
              </Button>
            </Card>
          ) : (
            <Grid cols={2} gap="lg" responsive={{ md: 1, lg: 2 }}>
              {positions.slice(0, 6).map((position, index) => (
                <Reveal key={index} animation="slideInLeft" delay={index * 100}>
                  <Card variant="glass" className="p-6">
                    <Flex justify="between" align="start" className="mb-4">
                      <Flex align="center" gap="md">
                        <div className="p-2 rounded-lg bg-neutral-800/50">
                          {getPositionIcon(position.type)}
                        </div>
                        <div>
                          <Typography variant="h4" weight="semibold">
                            {position.protocolName}
                          </Typography>
                          <Typography variant="bodySmall" color="secondary">
                            {position.type.charAt(0).toUpperCase() + position.type.slice(1)} • {position.tokenSymbol}
                          </Typography>
                        </div>
                      </Flex>
                      <Badge variant="success">
                        {position.apy}
                      </Badge>
                    </Flex>
                    
                    <Grid cols={2} gap="md" className="mb-4">
                      <div>
                        <Typography variant="caption" color="secondary">
                          Amount
                        </Typography>
                        <Typography variant="bodyLarge" weight="semibold">
                          {position.amount}
                        </Typography>
                      </div>
                      <div>
                        <Typography variant="caption" color="secondary">
                          Value
                        </Typography>
                        <Typography variant="bodyLarge" weight="semibold" color="success">
                          {position.valueUSD}
                        </Typography>
                      </div>
                    </Grid>

                    <Button variant="secondary" fullWidth size="sm">
                      Manage Position
                    </Button>
                  </Card>
                </Reveal>
              ))}
            </Grid>
          )}
        </Animated>
      )}

      {selectedTab === 'positions' && (
        <Animated animation="fadeIn">
          {positions.length === 0 ? (
            <Card variant="glass" className="p-12 text-center">
              <Typography variant="h3" color="secondary">
                No active positions
              </Typography>
            </Card>
          ) : (
            <div className="space-y-4">
              {positions.map((position, index) => (
                <Reveal key={index} animation="slideInLeft" delay={index * 50}>
                  <Card variant="glass" className="p-6">
                    <Flex justify="between" align="center">
                      <Flex align="center" gap="lg">
                        <div className="p-3 rounded-xl bg-neutral-800/50">
                          {getPositionIcon(position.type)}
                        </div>
                        <div>
                          <Typography variant="h4" weight="semibold">
                            {position.protocolName}
                          </Typography>
                          <Typography variant="bodyLarge" color="secondary">
                            {position.type.charAt(0).toUpperCase() + position.type.slice(1)} • {position.tokenSymbol}
                          </Typography>
                        </div>
                      </Flex>
                      
                      <Flex align="center" gap="lg">
                        <div className="text-right">
                          <Typography variant="h4" weight="bold" color="success">
                            {position.valueUSD}
                          </Typography>
                          <Typography variant="bodySmall" color="success">
                            {position.apy} APY
                          </Typography>
                        </div>
                      </Flex>
                    </Flex>
                    
                    <div className="mt-6 pt-4 border-t border-neutral-700">
                      <Grid cols={4} gap="md">
                        <div>
                          <Typography variant="caption" color="secondary">
                            Amount
                          </Typography>
                          <Typography variant="bodyLarge" weight="medium">
                            {position.amount}
                          </Typography>
                        </div>
                        <div>
                          <Typography variant="caption" color="secondary">
                            Rewards
                          </Typography>
                          <Typography variant="bodyLarge" weight="medium" color="success">
                            {position.rewards}
                          </Typography>
                        </div>
                        <div>
                          <Typography variant="caption" color="secondary">
                            Time Locked
                          </Typography>
                          <Typography variant="bodyLarge" weight="medium">
                            <Clock size={14} className="inline mr-1" />
                            Active
                          </Typography>
                        </div>
                        <div className="flex justify-end">
                          <Button variant="secondary" size="sm">
                            Manage
                          </Button>
                        </div>
                      </Grid>
                    </div>
                  </Card>
                </Reveal>
              ))}
            </div>
          )}
        </Animated>
      )}

      {selectedTab === 'protocols' && (
        <Animated animation="fadeIn">
          <Grid cols={2} gap="lg" responsive={{ md: 1, lg: 2 }}>
            {protocols.map((protocol, index) => (
              <Reveal key={protocol.id} animation="slideInLeft" delay={index * 100}>
                <Card variant="glass" className="p-6">
                  <Flex justify="between" align="start" className="mb-4">
                    <Flex align="center" gap="md">
                      <img 
                        src={protocol.logoUrl} 
                        alt={protocol.name}
                        className="w-12 h-12 rounded-full bg-neutral-800"
                        onError={(e) => {
                          e.currentTarget.src = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNDgiIGhlaWdodD0iNDgiIGZpbGw9IiNGRkQ3MDAiIHZpZXdCb3g9IjAgMCAyNCAyNCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMiIgY3k9IjEyIiByPSIxMCIvPjwvc3ZnPg==';
                        }}
                      />
                      <div>
                        <Typography variant="h4" weight="semibold">
                          {protocol.name}
                        </Typography>
                        <Typography variant="bodySmall" color="secondary" className="capitalize">
                          {protocol.type}
                        </Typography>
                      </div>
                    </Flex>
                    <Badge variant={getRiskBadgeVariant(protocol.riskLevel)}>
                      {getRiskIcon(protocol.riskLevel)}
                      <span className="ml-1 capitalize">{protocol.riskLevel}</span>
                    </Badge>
                  </Flex>
                  
                  <Typography variant="bodyLarge" color="secondary" className="mb-4">
                    {protocol.description}
                  </Typography>
                  
                  <Grid cols={2} gap="md" className="mb-6">
                    <div>
                      <Typography variant="caption" color="secondary">
                        Total Value Locked
                      </Typography>
                      <Typography variant="bodyLarge" weight="bold">
                        {protocol.tvl}
                      </Typography>
                    </div>
                    <div>
                      <Typography variant="caption" color="secondary">
                        APY
                      </Typography>
                      <Typography variant="bodyLarge" weight="bold" color="success">
                        {protocol.apy}
                      </Typography>
                    </div>
                  </Grid>
                  
                  <Flex gap="sm">
                    <Button variant="primary" fullWidth>
                      Use Protocol
                    </Button>
                    <Button 
                      variant="ghost"
                      onClick={() => window.open(protocol.website, '_blank')}
                      icon={<ExternalLink size={16} />}
                    />
                  </Flex>
                </Card>
              </Reveal>
            ))}
          </Grid>
        </Animated>
      )}

      {selectedTab === 'farming' && (
        <Animated animation="fadeIn">
          <div className="space-y-8">
            {/* Yield Farming Header */}
            <Card variant="outlined" className="p-6 border-caesar-gold/30 bg-caesar-gold/5">
              <Flex align="center" gap="md" className="mb-4">
                <Target className="text-caesar-gold" size={24} />
                <Typography variant="h3" color="gold">
                  Caesar Yield Farming
                </Typography>
              </Flex>
              <Typography variant="bodyLarge" color="secondary">
                Optimize your Caesar tokens for maximum utility while earning yield through anti-speculation farming
              </Typography>
            </Card>

            {/* Farming Opportunities */}
            <Grid cols={2} gap="lg" responsive={{ md: 1, lg: 2 }}>
              {getProtocolsByType('yield').map((protocol, index) => (
                <Reveal key={protocol.id} animation="slideInLeft" delay={index * 100}>
                  <Card variant="glass" className="p-6">
                    <Flex align="center" gap="md" className="mb-4">
                      <img 
                        src={protocol.logoUrl} 
                        alt={protocol.name}
                        className="w-10 h-10 rounded-full bg-neutral-800"
                      />
                      <div>
                        <Typography variant="h4" weight="semibold">
                          {protocol.name}
                        </Typography>
                        <Typography variant="bodySmall" color="secondary">
                          Utility-First Farming
                        </Typography>
                      </div>
                    </Flex>
                    
                    <Typography variant="bodyLarge" color="secondary" className="mb-4">
                      {protocol.description}
                    </Typography>
                    
                    <Flex justify="between" align="center" className="mb-4">
                      <div>
                        <Typography variant="caption" color="secondary">
                          APY
                        </Typography>
                        <Typography variant="h3" weight="bold" color="success">
                          {protocol.apy}
                        </Typography>
                      </div>
                      <div className="text-right">
                        <Typography variant="caption" color="secondary">
                          Participants
                        </Typography>
                        <Flex align="center" gap="xs">
                          <Users size={14} className="text-neutral-400" />
                          <Typography variant="bodyLarge">
                            {Math.floor(Math.random() * 1000) + 100}
                          </Typography>
                        </Flex>
                      </div>
                    </Flex>
                    
                    <Button variant="gold" fullWidth>
                      Start Farming
                    </Button>
                  </Card>
                </Reveal>
              ))}
            </Grid>
          </div>
        </Animated>
      )}
    </div>
  );
};

export default DeFiDashboard;