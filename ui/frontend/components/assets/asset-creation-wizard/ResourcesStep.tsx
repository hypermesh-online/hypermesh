// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Cpu, MemoryStick, HardDrive, Network } from 'lucide-react';
import type { AssetConfiguration } from './types';

interface ResourcesStepProps {
  config: AssetConfiguration;
  setConfig: React.Dispatch<React.SetStateAction<AssetConfiguration>>;
}

export function ResourcesStep({ config, setConfig }: ResourcesStepProps) {
  return (
    <div className="space-y-6">
      <h3 className="text-white font-medium text-lg">Resource Allocation Limits</h3>
      <div className="grid gap-6 md:grid-cols-2">
        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
            <Cpu className="h-4 w-4 text-blue-400" />
            CPU Cores
          </label>
          <input
            type="number"
            value={config.resourceLimits.cpu}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, cpu: parseInt(e.target.value) || 1 }
            }))}
            min="1"
            max="32"
            className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
          />
          <p className="text-xs text-gray-400">Number of CPU cores to allocate</p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
            <MemoryStick className="h-4 w-4 text-green-400" />
            Memory
          </label>
          <select
            value={config.resourceLimits.memory}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, memory: e.target.value }
            }))}
            className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-green-500 focus:outline-none focus:ring-2 focus:ring-green-500/20"
          >
            <option value="1GB">1 GB</option>
            <option value="2GB">2 GB</option>
            <option value="4GB">4 GB</option>
            <option value="8GB">8 GB</option>
            <option value="16GB">16 GB</option>
            <option value="32GB">32 GB</option>
            <option value="64GB">64 GB</option>
          </select>
          <p className="text-xs text-gray-400">Maximum memory allocation</p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
            <HardDrive className="h-4 w-4 text-purple-400" />
            Storage
          </label>
          <select
            value={config.resourceLimits.storage}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, storage: e.target.value }
            }))}
            className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-purple-500 focus:outline-none focus:ring-2 focus:ring-purple-500/20"
          >
            <option value="10GB">10 GB</option>
            <option value="25GB">25 GB</option>
            <option value="50GB">50 GB</option>
            <option value="100GB">100 GB</option>
            <option value="250GB">250 GB</option>
            <option value="500GB">500 GB</option>
            <option value="1TB">1 TB</option>
          </select>
          <p className="text-xs text-gray-400">Storage space allocation</p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
            <Network className="h-4 w-4 text-cyan-400" />
            Network Bandwidth
          </label>
          <select
            value={config.resourceLimits.network}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, network: e.target.value }
            }))}
            className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-cyan-500 focus:outline-none focus:ring-2 focus:ring-cyan-500/20"
          >
            <option value="50Mbps">50 Mbps</option>
            <option value="100Mbps">100 Mbps</option>
            <option value="250Mbps">250 Mbps</option>
            <option value="500Mbps">500 Mbps</option>
            <option value="1Gbps">1 Gbps</option>
            <option value="10Gbps">10 Gbps</option>
          </select>
          <p className="text-xs text-gray-400">Network bandwidth limit</p>
        </div>
      </div>
    </div>
  );
}
