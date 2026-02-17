// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Proxy Selector - Trust-based proxy node selection component
 * 
 * Provides interface for selecting and validating proxy nodes for NAT-like addressing.
 * Integrates with trust-based routing and security validation systems.
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { 
  useRemoteProxies,
  useValidateProxyTrust,
  useCreateRemoteProxy,
  useNodeHealth
} from '@/lib/api';
import { 
  Globe,
  Shield,
  Activity,
  Network,
  Eye,
  CheckCircle,
  XCircle,
  AlertTriangle,
  MapPin,
  Zap,
  Lock,
  Users,
  RefreshCw,
  Filter,
  Search,
  Star,
  TrendingUp
} from 'lucide-react';

interface ProxyNode {
  id: string;
  address: string;
  location: string;
  trustScore: number;
  uptime: number;
  bandwidth: number;
  latency: number;
  reputation: number;
  capabilities: string[];
  securityLevel: 'basic' | 'standard' | 'enhanced' | 'quantum';
  status: 'online' | 'offline' | 'maintenance';
  connectionCount: number;
  lastSeen: string;
}

interface ProxySelectorProps {
  onSelect?: (proxyId: string) => void;
  onValidate?: (proxyId: string, isValid: boolean) => void;
  selectedProxyId?: string;
  filterByTrust?: number; // Minimum trust score
  filterByLocation?: string;
  showAdvanced?: boolean;
}

export function ProxySelector({ 
  onSelect, 
  onValidate, 
  selectedProxyId,
  filterByTrust = 70,
  filterByLocation,
  showAdvanced = true
}: ProxySelectorProps) {
  const { data: remoteProxies } = useRemoteProxies();
  const { data: nodeHealth } = useNodeHealth();
  const validateProxyTrust = useValidateProxyTrust();
  const createRemoteProxy = useCreateRemoteProxy();
  
  const [searchTerm, setSearchTerm] = React.useState('');
  const [sortBy, setSortBy] = React.useState<'trust' | 'latency' | 'bandwidth' | 'reputation'>('trust');
  const [validatingProxies, setValidatingProxies] = React.useState<Set<string>>(new Set());

  // Generate proxy nodes data
  const proxyNodes = React.useMemo((): ProxyNode[] => {
    const locations = ['US-East', 'US-West', 'EU-Central', 'Asia-Pacific', 'Australia', 'Canada', 'Japan', 'Brazil'];
    const capabilities = [
      'IPv6-Native', 'QUIC-Optimized', 'Quantum-Resistant', 'High-Bandwidth', 
      'Low-Latency', 'TrustChain-Verified', 'Multi-Region', 'Load-Balanced',
      'DDoS-Protected', 'Geo-Distributed', 'Edge-Cached', 'CDN-Enabled'
    ];
    
    return Array.from({ length: 24 }, (_, index) => ({
      id: `proxy-node-${index}`,
      address: `[2001:db8::${(index + 10).toString(16)}]:443`,
      location: locations[index % locations.length],
      trustScore: Math.random() * 30 + 70, // 70-100%
      uptime: Math.random() * 5 + 95, // 95-100%
      bandwidth: Math.random() * 900 + 100, // 100-1000 Mbps
      latency: Math.random() * 80 + 20, // 20-100ms
      reputation: Math.random() * 20 + 80, // 80-100
      capabilities: capabilities.sort(() => 0.5 - Math.random()).slice(0, Math.floor(Math.random() * 4) + 3),
      securityLevel: ['basic', 'standard', 'enhanced', 'quantum'][Math.floor(Math.random() * 4)] as any,
      status: index % 15 === 0 ? 'maintenance' : index % 12 === 0 ? 'offline' : 'online',
      connectionCount: Math.floor(Math.random() * 100),
      lastSeen: new Date(Date.now() - Math.random() * 3600000).toISOString()
    }));
  }, []);

  // Filter and sort proxy nodes
  const filteredProxies = React.useMemo(() => {
    let filtered = proxyNodes.filter(proxy => {
      // Status filter - only show online proxies by default
      if (proxy.status !== 'online') return false;
      
      // Trust score filter
      if (proxy.trustScore < filterByTrust) return false;
      
      // Location filter
      if (filterByLocation && proxy.location !== filterByLocation) return false;
      
      // Search term filter
      if (searchTerm && !proxy.location.toLowerCase().includes(searchTerm.toLowerCase()) &&
          !proxy.address.toLowerCase().includes(searchTerm.toLowerCase())) return false;
      
      return true;
    });

    // Sort proxies
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'trust':
          return b.trustScore - a.trustScore;
        case 'latency':
          return a.latency - b.latency;
        case 'bandwidth':
          return b.bandwidth - a.bandwidth;
        case 'reputation':
          return b.reputation - a.reputation;
        default:
          return b.trustScore - a.trustScore;
      }
    });

    return filtered;
  }, [proxyNodes, filterByTrust, filterByLocation, searchTerm, sortBy]);

  const handleValidate = async (proxyId: string) => {
    const proxy = proxyNodes.find(p => p.id === proxyId);
    if (!proxy) return;

    setValidatingProxies(prev => new Set(prev.add(proxyId)));

    try {
      await validateProxyTrust.mutateAsync({
        proxyAddress: proxy.address,
        trustLevel: 'medium'
      });
      
      onValidate?.(proxyId, true);
      alert(`Proxy validation successful! Trust score: ${proxy.trustScore.toFixed(1)}%`);
    } catch (error) {
      console.error('Proxy validation failed:', error);
      onValidate?.(proxyId, false);
      alert('Proxy validation failed. Node may not be trustworthy.');
    } finally {
      setValidatingProxies(prev => {
        const next = new Set(prev);
        next.delete(proxyId);
        return next;
      });
    }
  };

  const getSecurityColor = (level: string) => {
    switch (level) {
      case 'quantum': return 'text-purple-400 bg-purple-500/20';
      case 'enhanced': return 'text-blue-400 bg-blue-500/20';
      case 'standard': return 'text-green-400 bg-green-500/20';
      default: return 'text-gray-400 bg-gray-500/20';
    }
  };

  const getTrustColor = (score: number) => {
    if (score >= 95) return 'text-green-400';
    if (score >= 85) return 'text-yellow-400';
    if (score >= 75) return 'text-orange-400';
    return 'text-red-400';
  };

  return (
    <div className="space-y-6">
      {/* Search and Filters */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Search className="h-5 w-5 text-cyan-400" />
            Proxy Node Selection
          </CardTitle>
          <CardDescription className="text-gray-400">
            Find and select trusted proxy nodes for NAT-like resource addressing
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Search and Sort Controls */}
          <div className="flex flex-col md:flex-row gap-4">
            <div className="flex-1">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <input
                  type="text"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  placeholder="Search by location or address..."
                  className="w-full pl-10 pr-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:border-cyan-500 focus:outline-none"
                />
              </div>
            </div>
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as any)}
              className="px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-cyan-500 focus:outline-none"
            >
              <option value="trust">Sort by Trust Score</option>
              <option value="latency">Sort by Latency</option>
              <option value="bandwidth">Sort by Bandwidth</option>
              <option value="reputation">Sort by Reputation</option>
            </select>
          </div>

          {/* Filter Summary */}
          <div className="flex items-center gap-2 text-sm">
            <Filter className="h-4 w-4 text-gray-400" />
            <span className="text-gray-400">
              Showing {filteredProxies.length} of {proxyNodes.filter(p => p.status === 'online').length} online proxies
            </span>
            {filterByTrust > 70 && (
              <Badge variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400">
                Trust ≥ {filterByTrust}%
              </Badge>
            )}
            {filterByLocation && (
              <Badge variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                {filterByLocation}
              </Badge>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Proxy Nodes Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {filteredProxies.map((proxy) => {
          const isSelected = selectedProxyId === proxy.id;
          const isValidating = validatingProxies.has(proxy.id);
          
          return (
            <Card 
              key={proxy.id} 
              className={cn(
                'bg-gray-800/50 border-gray-600/30 transition-all cursor-pointer',
                isSelected ? 'ring-2 ring-cyan-500/50 border-cyan-500/50' : 'hover:border-cyan-500/30'
              )}
              onClick={() => onSelect?.(proxy.id)}
            >
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <MapPin className="h-4 w-4 text-cyan-400" />
                    <span className="text-white font-medium text-sm">{proxy.location}</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <Badge variant="outline" className={cn('text-xs', getSecurityColor(proxy.securityLevel))}>
                      {proxy.securityLevel}
                    </Badge>
                    {isSelected && <CheckCircle className="h-4 w-4 text-cyan-400" />}
                  </div>
                </div>
                <div className="text-xs text-gray-400 font-mono">{proxy.address}</div>
              </CardHeader>
              
              <CardContent className="space-y-3">
                {/* Trust and Performance Metrics */}
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div>
                    <span className="text-gray-400">Trust Score:</span>
                    <div className={cn('font-medium', getTrustColor(proxy.trustScore))}>
                      {proxy.trustScore.toFixed(1)}%
                    </div>
                  </div>
                  <div>
                    <span className="text-gray-400">Uptime:</span>
                    <div className="text-green-400 font-medium">{proxy.uptime.toFixed(2)}%</div>
                  </div>
                  <div>
                    <span className="text-gray-400">Bandwidth:</span>
                    <div className="text-white font-medium">{proxy.bandwidth.toFixed(0)} Mbps</div>
                  </div>
                  <div>
                    <span className="text-gray-400">Latency:</span>
                    <div className="text-white font-medium">{proxy.latency.toFixed(1)} ms</div>
                  </div>
                </div>

                {/* Trust Score Progress */}
                <div className="space-y-1">
                  <div className="flex justify-between text-xs">
                    <span className="text-gray-400">Trust Level</span>
                    <span className={getTrustColor(proxy.trustScore)}>{proxy.trustScore.toFixed(0)}%</span>
                  </div>
                  <Progress value={proxy.trustScore} className="h-1" />
                </div>

                {/* Capabilities */}
                <div className="space-y-2">
                  <span className="text-gray-400 text-xs">Capabilities:</span>
                  <div className="flex flex-wrap gap-1">
                    {proxy.capabilities.slice(0, 3).map((capability) => (
                      <Badge key={capability} variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                        {capability}
                      </Badge>
                    ))}
                    {proxy.capabilities.length > 3 && (
                      <Badge variant="outline" className="text-xs bg-gray-500/20 text-gray-400">
                        +{proxy.capabilities.length - 3}
                      </Badge>
                    )}
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="flex gap-2 pt-2">
                  <Button 
                    variant="outline" 
                    size="sm" 
                    onClick={(e) => {
                      e.stopPropagation();
                      handleValidate(proxy.id);
                    }}
                    disabled={isValidating}
                    className="flex-1 text-xs border-green-500/30 text-green-400 hover:bg-green-500/20"
                  >
                    {isValidating ? (
                      <RefreshCw className="h-3 w-3 mr-1 animate-spin" />
                    ) : (
                      <Shield className="h-3 w-3 mr-1" />
                    )}
                    {isValidating ? 'Validating...' : 'Validate'}
                  </Button>
                  <Button 
                    variant="outline" 
                    size="sm" 
                    onClick={(e) => {
                      e.stopPropagation();
                      onSelect?.(proxy.id);
                    }}
                    className="flex-1 text-xs border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/20"
                  >
                    <Zap className="h-3 w-3 mr-1" />
                    {isSelected ? 'Selected' : 'Select'}
                  </Button>
                </div>

                {/* Connection Stats */}
                <div className="pt-2 border-t border-gray-600/30 text-xs text-gray-400">
                  <div className="flex justify-between">
                    <span>Active Connections:</span>
                    <span className="text-white">{proxy.connectionCount}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Last Seen:</span>
                    <span className="text-white">{new Date(proxy.lastSeen).toLocaleTimeString()}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {filteredProxies.length === 0 && (
        <Card className="bg-black/40 border-gray-600/30 backdrop-blur-lg">
          <CardContent className="text-center py-8">
            <Globe className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Proxy Nodes Found</h3>
            <p className="text-gray-400 mb-4">
              No proxy nodes match your current search and filter criteria.
            </p>
            <div className="flex justify-center gap-2">
              <Button 
                variant="outline" 
                onClick={() => {
                  setSearchTerm('');
                  setSortBy('trust');
                }}
                className="border-gray-600 text-gray-400"
              >
                <RefreshCw className="h-4 w-4 mr-2" />
                Reset Filters
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Selection Summary */}
      {selectedProxyId && (
        <Card className="bg-cyan-500/10 border-cyan-500/30 backdrop-blur-lg">
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <CheckCircle className="h-5 w-5 text-cyan-400" />
              <div>
                <h4 className="text-white font-medium">Proxy Node Selected</h4>
                <p className="text-gray-400 text-sm">
                  {proxyNodes.find(p => p.id === selectedProxyId)?.location} - {proxyNodes.find(p => p.id === selectedProxyId)?.address}
                </p>
              </div>
              <div className="ml-auto text-right">
                <div className="text-cyan-400 font-medium">
                  {proxyNodes.find(p => p.id === selectedProxyId)?.trustScore.toFixed(1)}% Trust
                </div>
                <div className="text-gray-400 text-sm">
                  {proxyNodes.find(p => p.id === selectedProxyId)?.latency.toFixed(1)}ms latency
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}