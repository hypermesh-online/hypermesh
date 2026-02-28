// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Simplified HyperMesh Asset Browser - Demo Implementation
// Demonstrates native protocol access without complex dependencies

import React, { useState, useEffect } from 'react';
import { 
  CpuChipIcon, 
  ServerIcon, 
  CloudIcon,
  MagnifyingGlassIcon,
  CheckCircleIcon,
  XCircleIcon
} from '@heroicons/react/24/outline';
import { clsx } from 'clsx';

interface SimpleAsset {
  id: string;
  type: 'CPU' | 'GPU' | 'Memory' | 'Storage';
  cost_per_hour: number;
  is_authenticated: boolean;
  location: string;
  state: 'Available' | 'Allocated';
  privacy_level: string;
  proxy_address: string;
  resource_usage?: {
    utilization_percent: number;
    used_amount: string;
  };
}

interface SimpleAssetBrowserProps {
  userId: string;
  onAssetAllocated?: (asset: SimpleAsset) => void;
  className?: string;
}

export const SimpleAssetBrowser: React.FC<SimpleAssetBrowserProps> = ({
  userId,
  onAssetAllocated,
  className
}) => {
  const [isConnected, setIsConnected] = useState(false);
  const [assets, setAssets] = useState<SimpleAsset[]>([]);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedType, setSelectedType] = useState<string>('all');
  const [allocatingAsset, setAllocatingAsset] = useState<string | null>(null);

  // Simulate connection to HyperMesh protocol
  useEffect(() => {
    const connect = async () => {
      console.log('Connecting to HyperMesh native protocol...');
      // Simulate connection delay
      await new Promise(resolve => setTimeout(resolve, 2000));
      setIsConnected(true);
      console.log('Connected to HyperMesh protocol');
    };

    connect();
  }, []);

  // Simulate asset discovery via native protocol
  const discoverAssets = async () => {
    if (!isConnected) return;

    setIsDiscovering(true);
    console.log('Starting native asset discovery with four-proof consensus...');

    // Simulate streaming asset discovery
    const mockAssets: SimpleAsset[] = [
      {
        id: 'cpu_001',
        type: 'CPU',
        cost_per_hour: 15,
        is_authenticated: true,
        location: 'San Francisco, CA',
        state: 'Available',
        privacy_level: 'PublicNetwork',
        proxy_address: 'hypermesh:4a2b:8c1f:0003:0001::cpu:a1b2c3d4',
        resource_usage: { utilization_percent: 25, used_amount: '2/8 cores' }
      },
      {
        id: 'gpu_002',
        type: 'GPU',
        cost_per_hour: 85,
        is_authenticated: true,
        location: 'Austin, TX',
        state: 'Available',
        privacy_level: 'P2P',
        proxy_address: 'hypermesh:7f3e:1a4b:0004:0002::gpu:e5f6g7h8',
        resource_usage: { utilization_percent: 0, used_amount: '0/24GB VRAM' }
      },
      {
        id: 'mem_003',
        type: 'Memory',
        cost_per_hour: 8,
        is_authenticated: true,
        location: 'Seattle, WA',
        state: 'Available',
        privacy_level: 'PublicNetwork',
        proxy_address: 'hypermesh:2d8a:5b9c:0005:0003::memory:i9j0k1l2',
        resource_usage: { utilization_percent: 40, used_amount: '32/64GB' }
      },
      {
        id: 'storage_004',
        type: 'Storage',
        cost_per_hour: 3,
        is_authenticated: true,
        location: 'New York, NY',
        state: 'Available',
        privacy_level: 'FullPublic',
        proxy_address: 'hypermesh:9e4d:3c7f:0006:0004::storage:m3n4o5p6',
        resource_usage: { utilization_percent: 65, used_amount: '650/1000GB' }
      },
      {
        id: 'cpu_005',
        type: 'CPU',
        cost_per_hour: 22,
        is_authenticated: true,
        location: 'London, UK',
        state: 'Available',
        privacy_level: 'PrivateNetwork',
        proxy_address: 'hypermesh:8b7c:4f2e:0007:0005::cpu:q7r8s9t0',
        resource_usage: { utilization_percent: 10, used_amount: '1/16 cores' }
      },
      {
        id: 'gpu_006',
        type: 'GPU',
        cost_per_hour: 120,
        is_authenticated: true,
        location: 'Tokyo, JP',
        state: 'Available',
        privacy_level: 'P2P',
        proxy_address: 'hypermesh:6c8d:9a1b:0008:0006::gpu:u1v2w3x4',
        resource_usage: { utilization_percent: 5, used_amount: '2/48GB VRAM' }
      }
    ];

    // Simulate streaming discovery with delays
    for (let i = 0; i < mockAssets.length; i++) {
      await new Promise(resolve => setTimeout(resolve, 800));
      setAssets(prev => [...prev, mockAssets[i]]);
    }

    setIsDiscovering(false);
    console.log(`Discovered ${mockAssets.length} assets with consensus validation`);
  };

  // Simulate asset allocation with consensus
  const allocateAsset = async (asset: SimpleAsset) => {
    setAllocatingAsset(asset.id);
    console.log(`Allocating ${asset.type} asset ${asset.id} with consensus validation...`);

    try {
      // Simulate consensus validation (PoSpace + PoStake + PoWork + PoTime)
      await new Promise(resolve => setTimeout(resolve, 3000));

      const allocation = {
        ...asset,
        allocation_id: `alloc_${Date.now()}`,
        allocated_at: Date.now(),
        state: 'Allocated' as const
      };

      // Remove from available assets
      setAssets(prev => prev.filter(a => a.id !== asset.id));
      
      onAssetAllocated?.(asset);
      
      console.log(`Successfully allocated ${asset.type} asset: ${allocation.allocation_id}`);
      alert(`Asset allocated successfully!\nProxy Address: ${asset.proxy_address}\nConsensus validation: PASSED (PoSpace ✓ PoStake ✓ PoWork ✓ PoTime ✓)`);

    } catch (error) {
      console.error('Asset allocation failed:', error);
      alert('Asset allocation failed');
    } finally {
      setAllocatingAsset(null);
    }
  };

  // Filter assets based on search and type
  const filteredAssets = assets.filter(asset => {
    const matchesSearch = asset.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         asset.location.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesType = selectedType === 'all' || asset.type === selectedType;
    return matchesSearch && matchesType;
  });

  // Asset type icon helper
  const getAssetIcon = (type: SimpleAsset['type']) => {
    const iconClass = "h-5 w-5";
    switch (type) {
      case 'CPU': return <CpuChipIcon className={iconClass} />;
      case 'GPU': return <ServerIcon className={iconClass} />;
      case 'Memory': return <ServerIcon className={iconClass} />;
      case 'Storage': return <CloudIcon className={iconClass} />;
      default: return <ServerIcon className={iconClass} />;
    }
  };

  return (
    <div className={clsx("h-full flex flex-col bg-gray-50", className)}>
      {/* Header */}
      <div className="bg-white border-b border-gray-200 p-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold text-gray-900">
            HyperMesh Asset Browser
          </h2>
          <div className="flex items-center space-x-2 text-sm">
            {isConnected ? (
              <>
                <CheckCircleIcon className="h-4 w-4 text-green-500" />
                <span className="text-green-600">Native Protocol Connected</span>
              </>
            ) : (
              <>
                <div className="animate-spin h-4 w-4 border-2 border-blue-500 border-t-transparent rounded-full" />
                <span className="text-blue-600">Connecting...</span>
              </>
            )}
          </div>
        </div>

        <div className="mt-4 text-sm text-gray-600">
          User: {userId} | Privacy Level: PublicNetwork | Consensus: Four-Proof Validation
        </div>
      </div>

      {/* Controls */}
      <div className="bg-white border-b border-gray-200 p-4 space-y-4">
        {/* Search */}
        <div className="relative">
          <MagnifyingGlassIcon className="absolute left-3 top-3 h-4 w-4 text-gray-400" />
          <input
            type="text"
            placeholder="Search assets by ID or location..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg"
          />
        </div>

        {/* Type filter */}
        <div className="flex space-x-2">
          {['all', 'CPU', 'GPU', 'Memory', 'Storage'].map(type => (
            <button
              key={type}
              onClick={() => setSelectedType(type)}
              className={clsx(
                "px-3 py-1 text-sm rounded-md border",
                selectedType === type
                  ? "bg-blue-100 border-blue-300 text-blue-700"
                  : "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
              )}
            >
              {type === 'all' ? 'All Assets' : `${type} Assets`}
            </button>
          ))}
        </div>

        {/* Discover button */}
        <button
          onClick={discoverAssets}
          disabled={!isConnected || isDiscovering}
          className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isDiscovering ? 'Discovering Assets...' : 'Discover Assets via Native Protocol'}
        </button>
      </div>

      {/* Asset grid */}
      <div className="flex-1 p-4 overflow-y-auto">
        {isDiscovering && assets.length === 0 ? (
          <div className="text-center py-8">
            <div className="animate-spin h-8 w-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-4" />
            <p className="text-gray-600">Discovering assets via native HyperMesh protocol...</p>
            <p className="text-sm text-gray-500 mt-2">
              Validating consensus proofs (PoSpace + PoStake + PoWork + PoTime)
            </p>
          </div>
        ) : filteredAssets.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredAssets.map(asset => (
              <div key={asset.id} className="bg-white rounded-lg border border-gray-200 p-4 shadow-sm hover:shadow-md transition-shadow">
                <div className="flex items-start justify-between">
                  <div className="flex items-center space-x-3">
                    {getAssetIcon(asset.type)}
                    <div>
                      <h3 className="font-medium text-gray-900">{asset.type} Asset</h3>
                      <p className="text-sm text-gray-500">ID: {asset.id}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="font-semibold text-gray-900">{asset.cost_per_hour} CAESAR/hr</p>
                    <p className={`text-sm ${asset.is_authenticated ? 'text-green-500' : 'text-red-500'}`}>{asset.is_authenticated ? 'Authenticated' : 'Not Authenticated'}</p>
                  </div>
                </div>

                <div className="mt-3 space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-500">Privacy:</span>
                    <span className="font-medium">{asset.privacy_level}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-500">Location:</span>
                    <span className="font-medium">{asset.location}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-500">State:</span>
                    <span className="font-medium text-green-600">{asset.state}</span>
                  </div>
                </div>

                {asset.resource_usage && (
                  <div className="mt-3 p-2 bg-gray-50 rounded text-sm">
                    <div className="text-gray-600 font-medium mb-1">Resource Usage:</div>
                    <div>Utilization: {asset.resource_usage.utilization_percent}%</div>
                    <div>Used: {asset.resource_usage.used_amount}</div>
                  </div>
                )}

                <div className="mt-3 p-2 bg-blue-50 rounded text-sm">
                  <div className="text-blue-600 font-medium mb-1">Proxy Address (NAT-like):</div>
                  <div className="font-mono text-xs break-all">{asset.proxy_address}</div>
                </div>

                <button
                  onClick={() => allocateAsset(asset)}
                  disabled={allocatingAsset === asset.id}
                  className="w-full mt-4 px-3 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  {allocatingAsset === asset.id ? 'Validating Consensus...' : 'Allocate Asset'}
                </button>
              </div>
            ))}
          </div>
        ) : assets.length === 0 && !isDiscovering ? (
          <div className="text-center py-8">
            <ServerIcon className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600">No assets discovered yet</p>
            <p className="text-sm text-gray-500 mt-1">
              Click "Discover Assets" to find available resources via native protocol
            </p>
          </div>
        ) : (
          <div className="text-center py-8">
            <XCircleIcon className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600">No assets match your filters</p>
            <p className="text-sm text-gray-500 mt-1">
              Try adjusting your search or type filter
            </p>
          </div>
        )}

        {isDiscovering && assets.length > 0 && (
          <div className="mt-4 text-center py-4 border-t border-gray-200">
            <div className="text-sm text-blue-600">
              Still discovering... Found {assets.length} assets so far
            </div>
          </div>
        )}
      </div>
    </div>
  );
};