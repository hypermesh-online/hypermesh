// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState } from 'react';
import { ChevronDown, Check, Zap, Globe, Link, Layers } from 'lucide-react';
import { Network } from '../types';
import { 
  SUPPORTED_NETWORKS, 
  getNetworksByType, 
  supports0xProtocol, 
  supportsLayerZero, 
  supportsIBC,
  supportsHyperlane
} from '../utils/networks';
import { Card, Typography, Button, Flex, Badge, Animated } from '../design-system';

interface MultiChainSelectorProps {
  currentNetwork: Network;
  onNetworkChange: (network: Network) => void;
  className?: string;
}

const MultiChainSelector: React.FC<MultiChainSelectorProps> = ({
  currentNetwork,
  onNetworkChange,
  className = ''
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<'all' | 'evm' | 'cosmos' | 'solana' | 'layerzero'>('all');

  const getNetworkIcon = (network: Network) => {
    switch (network.chainType) {
      case 'cosmos':
        return <Globe size={16} className="text-purple-400" />;
      case 'solana':
        return <Zap size={16} className="text-green-400" />;
      case 'layerzero':
        return <Link size={16} className="text-blue-400" />;
      default:
        return <Layers size={16} className="text-gray-400" />;
    }
  };

  const getNetworkFeatures = (network: Network) => {
    const features = [];
    if (supports0xProtocol(network.chainId)) features.push('0x');
    if (supportsLayerZero(network.chainId)) features.push('LZ');
    if (supportsHyperlane(network.chainId)) features.push('HL');
    if (supportsIBC(network.chainId)) features.push('IBC');
    return features;
  };

  const getFilteredNetworks = () => {
    if (activeTab === 'all') return SUPPORTED_NETWORKS;
    return getNetworksByType(activeTab);
  };

  const filteredNetworks = getFilteredNetworks();

  return (
    <div className={`relative ${className}`}>
      {/* Current Network Display */}
      <Button
        variant="glass"
        onClick={() => setIsOpen(!isOpen)}
        className="min-w-[200px] justify-start hover:scale-[1.02] transition-all duration-300"
      >
        <Flex align="center" gap="sm" className="w-full">
          <Flex align="center" gap="sm" className="flex-1">
            {getNetworkIcon(currentNetwork)}
            <div className="text-left">
              <Typography variant="bodyLarge" weight="medium" className="text-left">
                {currentNetwork.name}
              </Typography>
              <Flex align="center" gap="xs" className="text-left">
                <Typography variant="caption" color="secondary">
                  {currentNetwork.symbol}
                </Typography>
                {currentNetwork.isTestnet && (
                  <Badge variant="warning" size="sm">TESTNET</Badge>
                )}
                {getNetworkFeatures(currentNetwork).map(feature => (
                  <Badge key={feature} variant="success" size="sm">{feature}</Badge>
                ))}
              </Flex>
            </div>
          </Flex>
          <ChevronDown 
            size={16} 
            className={`text-neutral-400 transition-transform ${isOpen ? 'rotate-180' : ''}`} 
          />
        </Flex>
      </Button>

      {/* Dropdown */}
      {isOpen && (
        <Animated animation="slideDown">
          <Card variant="glass" className="absolute top-full left-0 right-0 mt-2 z-50 max-h-[500px] overflow-hidden border border-white/10">
            {/* Tabs */}
            <div className="border-b border-white/10 p-4">
              <Flex gap="sm" className="overflow-x-auto">
                {[
                  { key: 'all', label: 'All Networks', count: SUPPORTED_NETWORKS.length },
                  { key: 'evm', label: 'EVM', count: getNetworksByType('evm').length },
                  { key: 'cosmos', label: 'Cosmos', count: getNetworksByType('cosmos').length },
                  { key: 'solana', label: 'Solana', count: getNetworksByType('solana').length },
                  { key: 'layerzero', label: 'LayerZero', count: getNetworksByType('layerzero').length },
                ].map(tab => (
                  <Button
                    key={tab.key}
                    variant={activeTab === tab.key ? 'primary' : 'ghost'}
                    size="sm"
                    onClick={() => setActiveTab(tab.key as any)}
                    className="whitespace-nowrap"
                  >
                    {tab.label} ({tab.count})
                  </Button>
                ))}
              </Flex>
            </div>

            {/* Network List */}
            <div className="max-h-[350px] overflow-y-auto p-2">
              {filteredNetworks.map((network) => (
                <Button
                  key={network.chainId}
                  variant="ghost"
                  onClick={() => {
                    onNetworkChange(network);
                    setIsOpen(false);
                  }}
                  className="w-full justify-start p-3 h-auto hover:bg-white/5"
                >
                  <Flex justify="between" align="center" className="w-full">
                    <Flex align="center" gap="md">
                      {getNetworkIcon(network)}
                      <div className="text-left">
                        <Flex align="center" gap="sm">
                          <Typography variant="bodyLarge" weight="medium">
                            {network.name}
                          </Typography>
                          {network.chainId === currentNetwork.chainId && (
                            <Check size={14} className="text-semantic-success" />
                          )}
                        </Flex>
                        <Flex align="center" gap="xs">
                          <Typography variant="caption" color="secondary">
                            {network.symbol} • {network.chainType?.toUpperCase()}
                          </Typography>
                          {network.isTestnet && (
                            <Badge variant="warning" size="sm">TEST</Badge>
                          )}
                        </Flex>
                      </div>
                    </Flex>

                    {/* Features */}
                    <Flex align="center" gap="xs">
                      {getNetworkFeatures(network).map(feature => (
                        <Badge key={feature} variant="success" size="sm">
                          {feature}
                        </Badge>
                      ))}
                    </Flex>
                  </Flex>
                </Button>
              ))}
            </div>

            {/* Legend */}
            <div className="border-t border-white/10 p-4">
              <Typography variant="caption" weight="medium" color="secondary" className="mb-2 block">
                Features:
              </Typography>
              <div className="flex flex-wrap gap-3">
                <Typography variant="caption" color="tertiary">0x = 0x Protocol DEX aggregation</Typography>
                <Typography variant="caption" color="tertiary">LZ = LayerZero cross-chain</Typography>
                <Typography variant="caption" color="tertiary">HL = Hyperlane cross-chain</Typography>
                <Typography variant="caption" color="tertiary">IBC = Inter-Blockchain Communication</Typography>
              </div>
            </div>
          </Card>
        </Animated>
      )}

      {/* Overlay */}
      {isOpen && (
        <div 
          className="fixed inset-0 z-40" 
          onClick={() => setIsOpen(false)}
        />
      )}
    </div>
  );
};

export default MultiChainSelector;