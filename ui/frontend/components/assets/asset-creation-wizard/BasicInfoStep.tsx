// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { AssetConfiguration } from './types';

interface BasicInfoStepProps {
  config: AssetConfiguration;
  setConfig: React.Dispatch<React.SetStateAction<AssetConfiguration>>;
}

export function BasicInfoStep({ config, setConfig }: BasicInfoStepProps) {
  return (
    <div className="space-y-6">
      <h3 className="text-white font-medium text-lg">Basic Information</h3>
      <div className="space-y-4">
        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-300">Asset Name</label>
          <input
            type="text"
            value={config.name}
            onChange={(e) => setConfig(prev => ({ ...prev, name: e.target.value }))}
            placeholder="Enter a descriptive name for your asset..."
            className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
          />
          <p className="text-xs text-gray-400">
            Choose a name that clearly identifies the purpose of this asset
          </p>
        </div>

        {config.type === 'vm' && (
          <div className="space-y-2">
            <label className="text-sm font-medium text-gray-300">Runtime Environment</label>
            <select
              value={config.vmConfig?.runtime || 'julia'}
              onChange={(e) => setConfig(prev => ({
                ...prev,
                vmConfig: {
                  ...prev.vmConfig,
                  runtime: e.target.value as any,
                  environmentVariables: prev.vmConfig?.environmentVariables || {}
                }
              }))}
              className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
            >
              <option value="julia">Julia Runtime</option>
              <option value="python">Python Runtime</option>
              <option value="javascript">JavaScript Runtime</option>
              <option value="rust">Rust Runtime</option>
            </select>
          </div>
        )}
      </div>
    </div>
  );
}
