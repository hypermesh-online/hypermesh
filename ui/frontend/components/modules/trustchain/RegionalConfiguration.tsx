// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { AlertCircle } from 'lucide-react';

interface NodeSettings {
  nodeId: string;
  region: string;
  zone: string;
  maxConnections: number;
}

interface RegionalConfigurationProps {
  settings: NodeSettings;
  onSettingsChange: (updates: Partial<NodeSettings>) => void;
  validationErrors: Record<string, string>;
}

const regions = [
  { value: 'us-west-1', label: 'US West 1 (N. California)' },
  { value: 'us-west-2', label: 'US West 2 (Oregon)' },
  { value: 'us-east-1', label: 'US East 1 (N. Virginia)' },
  { value: 'us-east-2', label: 'US East 2 (Ohio)' },
  { value: 'eu-central-1', label: 'EU Central 1 (Frankfurt)' },
  { value: 'eu-west-1', label: 'EU West 1 (Ireland)' },
  { value: 'ap-southeast-1', label: 'AP Southeast 1 (Singapore)' },
  { value: 'ap-northeast-1', label: 'AP Northeast 1 (Tokyo)' }
];

const zonesByRegion: Record<string, string[]> = {
  'us-west-1': ['us-west-1a', 'us-west-1b', 'us-west-1c'],
  'us-west-2': ['us-west-2a', 'us-west-2b', 'us-west-2c'],
  'us-east-1': ['us-east-1a', 'us-east-1b', 'us-east-1c', 'us-east-1d'],
  'us-east-2': ['us-east-2a', 'us-east-2b', 'us-east-2c'],
  'eu-central-1': ['eu-central-1a', 'eu-central-1b', 'eu-central-1c'],
  'eu-west-1': ['eu-west-1a', 'eu-west-1b', 'eu-west-1c'],
  'ap-southeast-1': ['ap-southeast-1a', 'ap-southeast-1b', 'ap-southeast-1c'],
  'ap-northeast-1': ['ap-northeast-1a', 'ap-northeast-1b', 'ap-northeast-1c']
};

export function RegionalConfiguration({
  settings,
  onSettingsChange,
  validationErrors
}: RegionalConfigurationProps) {
  const availableZones = zonesByRegion[settings.region] || [];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      {/* Node Identity */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold">Node Identity & Location</h3>
        
        <div className="space-y-2">
          <Label htmlFor="nodeId">Node ID</Label>
          <Input
            id="nodeId"
            value={settings.nodeId}
            onChange={(e) => onSettingsChange({ nodeId: e.target.value })}
            placeholder="Enter unique node identifier"
            className={validationErrors.nodeId ? 'border-red-500' : ''}
            aria-describedby={validationErrors.nodeId ? 'nodeId-error' : undefined}
          />
          {validationErrors.nodeId && (
            <p id="nodeId-error" className="text-sm text-red-600 flex items-center">
              <AlertCircle className="h-4 w-4 mr-1" />
              {validationErrors.nodeId}
            </p>
          )}
        </div>

        <div className="space-y-2">
          <Label htmlFor="region">Region</Label>
          <Select
            value={settings.region}
            onValueChange={(value) => {
              const firstZone = zonesByRegion[value]?.[0] || '';
              onSettingsChange({ region: value, zone: firstZone });
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder="Select region" />
            </SelectTrigger>
            <SelectContent>
              {regions.map((region) => (
                <SelectItem key={region.value} value={region.value}>
                  {region.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label htmlFor="zone">Availability Zone</Label>
          <Select
            value={settings.zone}
            onValueChange={(value) => onSettingsChange({ zone: value })}
          >
            <SelectTrigger>
              <SelectValue placeholder="Select zone" />
            </SelectTrigger>
            <SelectContent>
              {availableZones.map((zone) => (
                <SelectItem key={zone} value={zone}>
                  {zone}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Connection Limits */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold">Connection Limits</h3>
        
        <div className="space-y-2">
          <Label htmlFor="maxConnections">Maximum Connections</Label>
          <div className="space-y-2">
            <Input
              id="maxConnections"
              type="number"
              value={settings.maxConnections}
              onChange={(e) => onSettingsChange({ maxConnections: parseInt(e.target.value) || 0 })}
              min={100}
              max={10000}
              className={validationErrors.maxConnections ? 'border-red-500' : ''}
            />
            <Slider
              value={[settings.maxConnections]}
              onValueChange={([value]) => onSettingsChange({ maxConnections: value })}
              max={10000}
              min={100}
              step={100}
              className="w-full"
            />
            <div className="flex justify-between text-sm text-muted-foreground">
              <span>100</span>
              <span>5,000</span>
              <span>10,000</span>
            </div>
          </div>
          {validationErrors.maxConnections && (
            <p className="text-sm text-red-600">{validationErrors.maxConnections}</p>
          )}
          <p className="text-sm text-muted-foreground">
            Current load: 247 connections ({((247 / settings.maxConnections) * 100).toFixed(1)}%)
          </p>
        </div>
      </div>
    </div>
  );
}