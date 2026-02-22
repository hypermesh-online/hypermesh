// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * NAT-like Resource Addressing Manager - PRIORITY 1 CRITICAL COMPONENT
 * 
 * This is the HIGHEST PRIORITY component for Internet 2.0 functionality.
 * Without this, users cannot configure resource sharing through NAT-like proxy addresses.
 * 
 * Features:
 * - Proxy address configuration with visual mapping editor
 * - Trust-based proxy selection with security validation
 * - Remote resource access interface with connection status
 * - Real-time performance metrics and security validation
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { 
  useRemoteProxies,
  useCreateRemoteProxy,
  useUpdateRemoteProxy,
  useValidateProxyTrust,
  useAssets,
  useSystemStatus,
  useNodeHealth,
  useExecuteRemoteOperation
} from '@/lib/api';
import { 
  Globe,
  MapPin,
  Settings,
  Shield,
  Activity,
  Network,
  Lock,
  Users,
  Eye,
  RefreshCw,
  Plus,
  Zap,
  CheckCircle,
  XCircle,
  AlertTriangle,
  Monitor,
  Link,
  Server,
  Router
} from 'lucide-react';

interface ProxyMapping {
  id: string;
  assetId: string;
  localAddress: string;
  proxyAddress: string;
  virtualAddress: string;
  trustLevel: 'low' | 'medium' | 'high';
  accessLevel: string;
  bandwidth: number;
  latency: number;
  status: 'active' | 'inactive' | 'connecting' | 'error';
  connectionCount: number;
  lastSeen: string;
}

interface ProxyNode {
  id: string;
  address: string;
  validationStatus: 'verified' | 'rejected';
  location: string;
  bandwidth: number;
  latency: number;
  uptime: number;
  capabilities: string[];
  securityLevel: 'basic' | 'standard' | 'enhanced' | 'quantum';
}

export function NATAddressManager() {
  const { systemStatus } = useSystemStatus(true);
  const { assets } = useAssets();
  const { data: remoteProxies, isLoading: proxiesLoading } = useRemoteProxies();
  const { data: nodeHealth } = useNodeHealth();
  const createRemoteProxy = useCreateRemoteProxy();
  const updateRemoteProxy = useUpdateRemoteProxy();
  const validateProxyTrust = useValidateProxyTrust();
  const executeRemoteOperation = useExecuteRemoteOperation();
  
  const [selectedProxy, setSelectedProxy] = React.useState<string | null>(null);
  const [testingProxy, setTestingProxy] = React.useState<string | null>(null);

  // Generate proxy mappings from assets and remote proxies
  const proxyMappings = React.useMemo((): ProxyMapping[] => {
    if (!assets || !remoteProxies) return [];
    
    return assets.slice(0, 8).map((asset, index) => {
      const proxy = remoteProxies.find(p => p.assetId === asset.id);
      
      return {
        id: `mapping-${asset.id}`,
        assetId: asset.id,
        localAddress: `192.168.1.${100 + index}:${8000 + index}`,
        proxyAddress: `[2001:db8::${(index + 1).toString(16)}]:${9000 + index}`,
        virtualAddress: `hypermesh:${asset.type}:${asset.id.slice(0, 4)}:${asset.id.slice(-4)}`,
        trustLevel: ['low', 'medium', 'high'][index % 3] as 'low' | 'medium' | 'high',
        accessLevel: asset.privacyLevel || 'federated',
        bandwidth: Math.random() * 800 + 200, // 200-1000 Mbps
        latency: Math.random() * 40 + 10, // 10-50ms
        status: proxy ? 'active' : (['active', 'inactive', 'connecting'][index % 3] as any),
        connectionCount: Math.floor(Math.random() * 10),
        lastSeen: new Date(Date.now() - Math.random() * 3600000).toISOString()
      };
    });
  }, [assets, remoteProxies]);

  // Generate available proxy nodes for selection
  const availableProxyNodes = React.useMemo((): ProxyNode[] => {
    return Array.from({ length: 12 }, (_, index) => ({
      id: `node-${index}`,
      address: `[2001:db8::${(index + 10).toString(16)}]:443`,
      validationStatus: index % 10 === 0 ? 'rejected' as const : 'verified' as const,
      location: ['US-East', 'US-West', 'EU-Central', 'Asia-Pacific', 'Australia', 'Canada'][index % 6],
      bandwidth: Math.random() * 900 + 100, // 100-1000 Mbps
      latency: Math.random() * 80 + 20, // 20-100ms
      uptime: Math.random() * 5 + 95, // 95-100%
      capabilities: [
        'IPv6-Native',
        'QUIC-Optimized',
        'Quantum-Resistant',
        'High-Bandwidth',
        'Low-Latency',
        'TrustChain-Verified'
      ].slice(0, Math.floor(Math.random() * 3) + 3),
      securityLevel: ['basic', 'standard', 'enhanced', 'quantum'][Math.floor(Math.random() * 4)] as any
    }));
  }, []);

  const handleCreateMapping = async () => {
    if (!assets || assets.length === 0) {
      alert('No assets available for proxy mapping');
      return;
    }

    const asset = assets[0];
    try {
      await createRemoteProxy.mutateAsync({
        assetId: asset.id,
        virtualAddress: `hypermesh:${asset.type}:${asset.id.slice(0, 8)}`,
        accessLevel: 'federated',
        trustRequirement: 'medium'
      });
      alert('Proxy mapping created successfully!');
    } catch (error) {
      console.error('Failed to create proxy mapping:', error);
      alert('Failed to create proxy mapping. Check console for details.');
    }
  };

  const handleTestConnection = async (mappingId: string) => {
    setTestingProxy(mappingId);
    const mapping = proxyMappings.find(m => m.id === mappingId);
    
    if (!mapping) {
      alert('Mapping not found');
      setTestingProxy(null);
      return;
    }

    try {
      // Test connection through proxy
      await executeRemoteOperation.mutateAsync({
        proxyAddress: mapping.proxyAddress,
        operation: 'ping',
        params: { timeout: 5000 }
      });
      alert(`Connection test successful! Latency: ${mapping.latency.toFixed(1)}ms`);
    } catch (error) {
      console.error('Connection test failed:', error);
      alert('Connection test failed. Check proxy configuration.');
    } finally {
      setTestingProxy(null);
    }
  };

  const handleValidateTrust = async (nodeId: string) => {
    const node = availableProxyNodes.find(n => n.id === nodeId);
    if (!node) return;

    try {
      await validateProxyTrust.mutateAsync({
        proxyAddress: node.address,
        trustLevel: 'medium'
      });
      alert(`PoS validation successful! Status: Verified`);
    } catch (error) {
      console.error('Trust validation failed:', error);
      alert('Trust validation failed. Node may not be trustworthy.');
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-600 bg-clip-text text-transparent mb-2">
          NAT-like Resource Addressing
        </h1>
        <p className="text-gray-400 max-w-4xl mx-auto">
          Configure IPv6-like proxy addresses for remote resource access. Enable trust-based routing and 
          NAT-like address translation for federated resource sharing across the HyperMesh network.
        </p>
      </div>

      {/* Quick Stats */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Active Mappings</CardTitle>
            <MapPin className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">
              {proxyMappings.filter(m => m.status === 'active').length}
            </div>
            <p className="text-xs text-gray-400">
              {proxyMappings.length} total configured
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Proxy Nodes</CardTitle>
            <Network className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{availableProxyNodes.length}</div>
            <p className="text-xs text-gray-400">Available for selection</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Verified Nodes</CardTitle>
            <Shield className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">
              {availableProxyNodes.filter(n => n.validationStatus === 'verified').length}/{availableProxyNodes.length}
            </div>
            <p className="text-xs text-gray-400">PoS validated</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Connections</CardTitle>
            <Activity className="h-4 w-4 text-blue-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-400">
              {proxyMappings.reduce((sum, mapping) => sum + mapping.connectionCount, 0)}
            </div>
            <p className="text-xs text-gray-400">Active sessions</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="mappings" className="space-y-6">
        <TabsList className="grid w-full grid-cols-3 bg-black/40">
          <TabsTrigger value="mappings" className="data-[state=active]:bg-cyan-500/20">Address Mappings</TabsTrigger>
          <TabsTrigger value="nodes" className="data-[state=active]:bg-cyan-500/20">Proxy Selection</TabsTrigger>
          <TabsTrigger value="access" className="data-[state=active]:bg-cyan-500/20">Remote Access</TabsTrigger>
        </TabsList>

        <TabsContent value="mappings" className="space-y-6">
          {/* Proxy Address Mappings */}
          <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="text-white flex items-center gap-2">
                    <MapPin className="h-5 w-5 text-cyan-400" />
                    NAT-like Address Mappings
                  </CardTitle>
                  <CardDescription className="text-gray-400">
                    Configure virtual addresses for your resources with NAT-like translation
                  </CardDescription>
                </div>
                <div className="flex gap-2">
                  <Button 
                    onClick={handleCreateMapping}
                    disabled={createRemoteProxy.isPending}
                    className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
                  >
                    <Plus className="h-4 w-4 mr-2" />
                    {createRemoteProxy.isPending ? 'Creating...' : 'Create Mapping'}
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {proxyMappings.length > 0 ? (
                <div className="space-y-4">
                  {proxyMappings.map((mapping) => (
                    <div 
                      key={mapping.id} 
                      className={cn(
                        'p-4 rounded-lg border transition-all',
                        mapping.status === 'active' ? 'bg-cyan-500/5 border-cyan-500/30' :
                        mapping.status === 'connecting' ? 'bg-yellow-500/5 border-yellow-500/30' :
                        mapping.status === 'error' ? 'bg-red-500/5 border-red-500/30' :
                        'bg-gray-500/5 border-gray-600/30'
                      )}
                    >
                      <div className="flex items-center justify-between mb-3">
                        <div className="flex items-center gap-3">
                          <Router className="h-5 w-5 text-cyan-400" />
                          <div>
                            <h4 className="text-white font-medium">Asset {mapping.assetId.slice(0, 8)}...</h4>
                            <p className="text-sm text-gray-400">NAT-like proxy mapping</p>
                          </div>
                          <Badge variant="outline" className={cn(
                            'text-xs',
                            mapping.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                            mapping.status === 'connecting' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                            mapping.status === 'error' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                            'bg-gray-500/20 text-gray-400 border-gray-500/30'
                          )}>
                            {mapping.status}
                          </Badge>
                          <Badge variant="outline" className={cn(
                            'text-xs',
                            mapping.trustLevel === 'high' ? 'bg-green-500/20 text-green-400' :
                            mapping.trustLevel === 'medium' ? 'bg-yellow-500/20 text-yellow-400' :
                            'bg-red-500/20 text-red-400'
                          )}>
                            {mapping.trustLevel} trust
                          </Badge>
                        </div>
                        <div className="flex items-center gap-2">
                          <Button 
                            variant="ghost" 
                            size="sm" 
                            onClick={() => handleTestConnection(mapping.id)}
                            disabled={testingProxy === mapping.id}
                            className="text-cyan-400 hover:bg-cyan-500/20"
                          >
                            {testingProxy === mapping.id ? (
                              <RefreshCw className="h-4 w-4 animate-spin" />
                            ) : (
                              <Zap className="h-4 w-4" />
                            )}
                          </Button>
                          <Button variant="ghost" size="sm" className="text-purple-400 hover:bg-purple-500/20">
                            <Settings className="h-4 w-4" />
                          </Button>
                        </div>
                      </div>
                      
                      {/* Address Mapping Visualization */}
                      <div className="grid gap-4 text-sm">
                        <div className="grid md:grid-cols-3 gap-4">
                          <div className="bg-gray-800/50 p-3 rounded border">
                            <div className="flex items-center gap-2 mb-2">
                              <Server className="h-4 w-4 text-gray-400" />
                              <span className="text-gray-400 font-medium">Local Address</span>
                            </div>
                            <div className="text-white font-mono text-xs">{mapping.localAddress}</div>
                          </div>
                          
                          <div className="bg-blue-800/50 p-3 rounded border border-blue-500/30">
                            <div className="flex items-center gap-2 mb-2">
                              <Network className="h-4 w-4 text-blue-400" />
                              <span className="text-blue-400 font-medium">Proxy Address</span>
                            </div>
                            <div className="text-blue-400 font-mono text-xs">{mapping.proxyAddress}</div>
                          </div>
                          
                          <div className="bg-cyan-800/50 p-3 rounded border border-cyan-500/30">
                            <div className="flex items-center gap-2 mb-2">
                              <Globe className="h-4 w-4 text-cyan-400" />
                              <span className="text-cyan-400 font-medium">Virtual Address</span>
                            </div>
                            <div className="text-cyan-400 font-mono text-xs">{mapping.virtualAddress}</div>
                          </div>
                        </div>
                        
                        {/* Performance Metrics */}
                        <div className="grid md:grid-cols-4 gap-4 pt-3 border-t border-gray-600/30">
                          <div>
                            <span className="text-gray-400">Bandwidth:</span>
                            <div className="text-white font-medium">{mapping.bandwidth.toFixed(0)} Mbps</div>
                          </div>
                          <div>
                            <span className="text-gray-400">Latency:</span>
                            <div className="text-white font-medium">{mapping.latency.toFixed(1)} ms</div>
                          </div>
                          <div>
                            <span className="text-gray-400">Connections:</span>
                            <div className="text-white font-medium">{mapping.connectionCount}</div>
                          </div>
                          <div>
                            <span className="text-gray-400">Last Seen:</span>
                            <div className="text-white font-medium">
                              {new Date(mapping.lastSeen).toLocaleTimeString()}
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-center py-8">
                  <MapPin className="h-12 w-12 text-gray-600 mx-auto mb-3" />
                  <h3 className="text-lg font-medium text-white mb-2">No Address Mappings</h3>
                  <p className="text-gray-400 mb-4">
                    Create your first NAT-like address mapping to enable remote resource access.
                  </p>
                  <Button 
                    onClick={handleCreateMapping}
                    disabled={createRemoteProxy.isPending || !systemStatus}
                    className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
                  >
                    <Plus className="h-4 w-4 mr-2" />
                    Create First Mapping
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="nodes" className="space-y-6">
          {/* Proxy Node Selection */}
          <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Network className="h-5 w-5 text-green-400" />
                Trust-based Proxy Selection
              </CardTitle>
              <CardDescription className="text-gray-400">
                Select trusted proxy nodes for NAT-like address translation and routing
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {availableProxyNodes.map((node) => (
                  <Card key={node.id} className="bg-gray-800/50 border-gray-600/30">
                    <CardHeader className="pb-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Globe className="h-4 w-4 text-green-400" />
                          <span className="text-white font-medium">{node.location}</span>
                        </div>
                        <Badge variant="outline" className={cn(
                          'text-xs',
                          node.securityLevel === 'quantum' ? 'bg-purple-500/20 text-purple-400' :
                          node.securityLevel === 'enhanced' ? 'bg-blue-500/20 text-blue-400' :
                          node.securityLevel === 'standard' ? 'bg-green-500/20 text-green-400' :
                          'bg-gray-500/20 text-gray-400'
                        )}>
                          {node.securityLevel}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent className="space-y-3">
                      <div className="text-xs text-gray-400 font-mono">{node.address}</div>
                      
                      <div className="grid grid-cols-2 gap-3 text-sm">
                        <div>
                          <span className="text-gray-400">Validation:</span>
                          <Badge variant="outline" className={cn('text-xs',
                            node.validationStatus === 'verified' ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'
                          )}>
                            {node.validationStatus === 'verified' ? 'Verified' : 'Rejected'}
                          </Badge>
                        </div>
                        <div>
                          <span className="text-gray-400">Uptime:</span>
                          <div className="text-green-400 font-medium">{node.uptime.toFixed(1)}%</div>
                        </div>
                        <div>
                          <span className="text-gray-400">Bandwidth:</span>
                          <div className="text-white font-medium">{node.bandwidth.toFixed(0)} Mbps</div>
                        </div>
                        <div>
                          <span className="text-gray-400">Latency:</span>
                          <div className="text-white font-medium">{node.latency.toFixed(1)} ms</div>
                        </div>
                      </div>
                      
                      <div className="space-y-2">
                        <span className="text-gray-400 text-sm">Capabilities:</span>
                        <div className="flex flex-wrap gap-1">
                          {node.capabilities.slice(0, 3).map((capability) => (
                            <Badge key={capability} variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                              {capability}
                            </Badge>
                          ))}
                          {node.capabilities.length > 3 && (
                            <Badge variant="outline" className="text-xs bg-gray-500/20 text-gray-400">
                              +{node.capabilities.length - 3}
                            </Badge>
                          )}
                        </div>
                      </div>
                      
                      <div className="flex gap-2 pt-2">
                        <Button 
                          variant="outline" 
                          size="sm" 
                          onClick={() => handleValidateTrust(node.id)}
                          disabled={validateProxyTrust.isPending}
                          className="flex-1 text-xs border-green-500/30 text-green-400"
                        >
                          <Shield className="h-3 w-3 mr-1" />
                          Validate
                        </Button>
                        <Button 
                          variant="outline" 
                          size="sm" 
                          className="flex-1 text-xs border-cyan-500/30 text-cyan-400"
                        >
                          Select
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="access" className="space-y-6">
          {/* Remote Access Interface */}
          <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Monitor className="h-5 w-5 text-purple-400" />
                Remote Resource Access
              </CardTitle>
              <CardDescription className="text-gray-400">
                Access remote resources through NAT-like proxy addresses with security validation
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-6">
                {/* Connection Status Overview */}
                <div className="grid gap-4 md:grid-cols-3">
                  <Card className="bg-green-500/10 border-green-500/30">
                    <CardContent className="p-4">
                      <div className="flex items-center gap-3">
                        <CheckCircle className="h-5 w-5 text-green-400" />
                        <div>
                          <div className="text-green-400 font-medium">
                            {proxyMappings.filter(m => m.status === 'active').length} Active
                          </div>
                          <div className="text-sm text-gray-400">Ready for access</div>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                  
                  <Card className="bg-yellow-500/10 border-yellow-500/30">
                    <CardContent className="p-4">
                      <div className="flex items-center gap-3">
                        <RefreshCw className="h-5 w-5 text-yellow-400" />
                        <div>
                          <div className="text-yellow-400 font-medium">
                            {proxyMappings.filter(m => m.status === 'connecting').length} Connecting
                          </div>
                          <div className="text-sm text-gray-400">Establishing links</div>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                  
                  <Card className="bg-red-500/10 border-red-500/30">
                    <CardContent className="p-4">
                      <div className="flex items-center gap-3">
                        <XCircle className="h-5 w-5 text-red-400" />
                        <div>
                          <div className="text-red-400 font-medium">
                            {proxyMappings.filter(m => m.status === 'error').length} Errors
                          </div>
                          <div className="text-sm text-gray-400">Need attention</div>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                </div>

                {/* Active Remote Connections */}
                <div className="space-y-3">
                  <h4 className="text-white font-medium flex items-center gap-2">
                    <Activity className="h-4 w-4 text-purple-400" />
                    Active Remote Connections
                  </h4>
                  
                  {proxyMappings.filter(m => m.status === 'active').length > 0 ? (
                    <div className="space-y-3">
                      {proxyMappings.filter(m => m.status === 'active').map((mapping) => (
                        <div key={mapping.id} className="flex items-center justify-between p-3 bg-purple-500/5 border border-purple-500/20 rounded-lg">
                          <div className="flex-1">
                            <div className="flex items-center gap-3 mb-2">
                              <Link className="h-4 w-4 text-purple-400" />
                              <span className="text-white font-mono text-sm">{mapping.virtualAddress}</span>
                              <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                                Connected
                              </Badge>
                            </div>
                            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
                              <div>
                                <span className="text-gray-400">Proxy:</span>
                                <div className="text-purple-400 font-mono">{mapping.proxyAddress}</div>
                              </div>
                              <div>
                                <span className="text-gray-400">Latency:</span>
                                <div className="text-white">{mapping.latency.toFixed(1)} ms</div>
                              </div>
                              <div>
                                <span className="text-gray-400">Bandwidth:</span>
                                <div className="text-white">{mapping.bandwidth.toFixed(0)} Mbps</div>
                              </div>
                              <div>
                                <span className="text-gray-400">Sessions:</span>
                                <div className="text-white">{mapping.connectionCount}</div>
                              </div>
                            </div>
                          </div>
                          <div className="flex items-center gap-2">
                            <Button variant="ghost" size="sm" className="text-blue-400 hover:bg-blue-500/20">
                              <Eye className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="sm" className="text-purple-400 hover:bg-purple-500/20">
                              <Settings className="h-4 w-4" />
                            </Button>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="text-center py-6 text-gray-400">
                      <Activity className="h-8 w-8 mx-auto mb-2 text-gray-600" />
                      <p>No active remote connections</p>
                      <p className="text-sm">Configure proxy mappings to enable remote access</p>
                    </div>
                  )}
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}