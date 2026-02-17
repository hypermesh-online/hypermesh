// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Label } from '@/components/ui/label';
import { Slider } from '@/components/ui/slider';

interface NodeSettings {
  bandwidth: {
    upload: number;
    download: number;
  };
}

interface BandwidthConfigurationProps {
  settings: NodeSettings;
  onSettingsChange: (updates: Partial<NodeSettings>) => void;
}

export function BandwidthConfiguration({
  settings,
  onSettingsChange
}: BandwidthConfigurationProps) {
  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold">Bandwidth Allocation</h3>
      
      <div className="space-y-4">
        <div className="space-y-2">
          <Label>Upload Bandwidth</Label>
          <div className="flex items-center space-x-2">
            <Slider
              value={[settings.bandwidth.upload]}
              onValueChange={([value]) => onSettingsChange({ 
                bandwidth: { ...settings.bandwidth, upload: value } 
              })}
              max={10000}
              min={100}
              step={100}
              className="flex-1"
            />
            <span className="text-sm font-medium w-20">
              {settings.bandwidth.upload} Mbps
            </span>
          </div>
        </div>

        <div className="space-y-2">
          <Label>Download Bandwidth</Label>
          <div className="flex items-center space-x-2">
            <Slider
              value={[settings.bandwidth.download]}
              onValueChange={([value]) => onSettingsChange({ 
                bandwidth: { ...settings.bandwidth, download: value } 
              })}
              max={10000}
              min={100}
              step={100}
              className="flex-1"
            />
            <span className="text-sm font-medium w-20">
              {settings.bandwidth.download} Mbps
            </span>
          </div>
        </div>

        <div className="p-3 bg-blue-50 border border-blue-200 rounded-lg">
          <div className="text-sm text-blue-800">
            <div className="font-medium">Detected: 10 Gbps available</div>
            <div>Allocated: {((settings.bandwidth.upload + settings.bandwidth.download) / 1000).toFixed(1)} Gbps</div>
          </div>
        </div>
      </div>
    </div>
  );
}