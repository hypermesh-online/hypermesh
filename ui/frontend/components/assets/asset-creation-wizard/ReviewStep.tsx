// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { AssetConfiguration } from './types';

interface ReviewStepProps {
  config: AssetConfiguration;
}

export function ReviewStep({ config }: ReviewStepProps) {
  return (
    <div className="space-y-6">
      <h3 className="text-white font-medium text-lg">Review Configuration</h3>
      <div className="bg-gray-800/50 rounded-lg p-6 space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <span className="text-gray-400 text-sm">Asset Name:</span>
            <div className="text-white font-medium">{config.name || 'Unnamed Asset'}</div>
          </div>
          <div>
            <span className="text-gray-400 text-sm">Asset Type:</span>
            <div className="text-white font-medium capitalize">{config.type}</div>
          </div>
          <div>
            <span className="text-gray-400 text-sm">Privacy Level:</span>
            <div className="text-white font-medium capitalize">{config.privacyLevel}</div>
          </div>
          <div>
            <span className="text-gray-400 text-sm">Resource Limits:</span>
            <div className="text-white font-medium">
              {config.resourceLimits.cpu} CPU, {config.resourceLimits.memory}, {config.resourceLimits.storage}, {config.resourceLimits.network}
            </div>
          </div>
          {config.type === 'vm' && config.vmConfig && (
            <div className="md:col-span-2">
              <span className="text-gray-400 text-sm">VM Runtime:</span>
              <div className="text-white font-medium capitalize">{config.vmConfig.runtime}</div>
            </div>
          )}
        </div>
      </div>

      <div className="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
        <h4 className="text-blue-400 font-medium text-sm mb-2">Ready to Create</h4>
        <p className="text-gray-300 text-sm">
          Your asset will be created with the configuration above. Once created, it will be
          available for allocation and use according to the privacy level you selected.
        </p>
      </div>
    </div>
  );
}
