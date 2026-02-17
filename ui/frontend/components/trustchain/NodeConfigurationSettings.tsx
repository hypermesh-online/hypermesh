// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Slider } from '@/components/ui/slider';
import { Separator } from '@/components/ui/separator';
import { Network, Save, RefreshCw, TestTube2 } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface NodeSettings {
  nodeId: string;
  ipv6Address: string;
  region: string;
  zone: string;
  proxyEnabled: boolean;
  autoDiscovery: boolean;
  maxConnections: number;
  bandwidth: {
    upload: number;
    download: number;
  };
}

interface NodeConfigurationSettingsProps {
  settings?: NodeSettings;
  onSave?: (settings: NodeSettings) => void;
  onTest?: (settings: NodeSettings) => void;
  onReset?: () => void;
  loading?: boolean;
  className?: string;
}

const defaultSettings: NodeSettings = {
  nodeId: 'node-001',
  ipv6Address: '2001:db8::1001',
  region: 'us-west-2',
  zone: 'us-west-2a',
  proxyEnabled: true,
  autoDiscovery: true,
  maxConnections: 1000,
  bandwidth: {
    upload: 1000,
    download: 1000
  }
};

const regions = [
  { value: 'us-west-2', label: 'US West 2' },
  { value: 'us-east-1', label: 'US East 1' },
  { value: 'eu-central-1', label: 'EU Central 1' },
  { value: 'ap-southeast-1', label: 'AP Southeast 1' },
  { value: 'ap-northeast-1', label: 'AP Northeast 1' },
  { value: 'eu-west-1', label: 'EU West 1' }
];

// IPv6 address validation
const isValidIPv6 = (address: string): boolean => {
  const ipv6Regex = /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^::1$|^::$/;
  const compressedRegex = /^([0-9a-fA-F]{1,4}:)*::([0-9a-fA-F]{1,4}:)*[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:)*::[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:)*::$/;
  return ipv6Regex.test(address) || compressedRegex.test(address);
};

export function NodeConfigurationSettings({
  settings = defaultSettings,
  onSave,
  onTest,
  onReset,
  loading = false,
  className
}: NodeConfigurationSettingsProps) {
  const [nodeSettings, setNodeSettings] = useState<NodeSettings>(settings);
  const [ipv6Error, setIpv6Error] = useState<string>('');
  const [isDirty, setIsDirty] = useState(false);

  const handleSettingChange = useCallback(<K extends keyof NodeSettings>(
    key: K,
    value: NodeSettings[K]
  ) => {
    setNodeSettings(prev => ({ ...prev, [key]: value }));
    setIsDirty(true);
  }, []);

  const handleBandwidthChange = useCallback((
    type: 'upload' | 'download',
    value: number[]
  ) => {
    setNodeSettings(prev => ({
      ...prev,
      bandwidth: { ...prev.bandwidth, [type]: value[0] }
    }));
    setIsDirty(true);
  }, []);

  const handleIPv6Change = useCallback((value: string) => {
    setNodeSettings(prev => ({ ...prev, ipv6Address: value }));
    setIsDirty(true);
    
    if (value && !isValidIPv6(value)) {
      setIpv6Error('Invalid IPv6 address format');
    } else {
      setIpv6Error('');
    }
  }, []);

  const handleSave = useCallback(() => {
    if (ipv6Error) return;
    onSave?.(nodeSettings);
    setIsDirty(false);
  }, [nodeSettings, onSave, ipv6Error]);

  const handleTest = useCallback(() => {
    if (ipv6Error) return;
    onTest?.(nodeSettings);
  }, [nodeSettings, onTest, ipv6Error]);

  const handleReset = useCallback(() => {
    setNodeSettings(defaultSettings);
    setIpv6Error('');
    setIsDirty(false);
    onReset?.();
  }, [onReset]);

  return (
    <Card className={cn("bg-black/40 border-green-500/30 backdrop-blur-lg", className)}>
      <CardHeader className="pb-4">
        <div className="flex items-center space-x-2">
          <Network className="h-5 w-5 text-blue-400" />
          <CardTitle className="text-white">Node Configuration</CardTitle>
          {isDirty && (
            <Badge variant="outline" className="text-amber-400 border-amber-400/50">
              Unsaved Changes
            </Badge>
          )}
        </div>
        <CardDescription className="text-gray-400">
          Configure your Web3 ecosystem node settings and network preferences
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-6">
        {/* Node Identity */}
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="nodeId" className="text-white">Node ID</Label>
              <Input
                id="nodeId"
                value={nodeSettings.nodeId}
                onChange={(e) => handleSettingChange('nodeId', e.target.value)}
                className="bg-black/20 border-green-500/30 text-white"
                placeholder="Enter unique node identifier"
              />
              <p className="text-xs text-gray-400">
                Unique identifier for this node in the network
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="ipv6Address" className="text-white">IPv6 Address</Label>
              <Input
                id="ipv6Address"
                value={nodeSettings.ipv6Address}
                onChange={(e) => handleIPv6Change(e.target.value)}
                className={cn(
                  "bg-black/20 border-green-500/30 text-white",
                  ipv6Error && "border-red-500/50"
                )}
                placeholder="2001:db8::1001"
              />
              {ipv6Error && (
                <p className="text-xs text-red-400">{ipv6Error}</p>
              )}
              <p className="text-xs text-gray-400">
                IPv6 address for network communication
              </p>
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Location Settings */}
        <div className="space-y-4">
          <h4 className="text-white font-medium">Location & Zone</h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="region" className="text-white">Region</Label>
              <Select 
                value={nodeSettings.region}
                onValueChange={(value) => handleSettingChange('region', value)}
              >
                <SelectTrigger className="bg-black/20 border-green-500/30 text-white">
                  <SelectValue placeholder="Select region" />
                </SelectTrigger>
                <SelectContent className="bg-black border-green-500/30">
                  {regions.map((region) => (
                    <SelectItem key={region.value} value={region.value} className="text-white">
                      {region.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="zone" className="text-white">Availability Zone</Label>
              <Input
                id="zone"
                value={nodeSettings.zone}
                onChange={(e) => handleSettingChange('zone', e.target.value)}
                className="bg-black/20 border-green-500/30 text-white"
                placeholder="us-west-2a"
              />
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Network Settings */}
        <div className="space-y-4">
          <h4 className="text-white font-medium">Network Configuration</h4>
          
          <div className="space-y-2">
            <Label htmlFor="maxConnections" className="text-white">Maximum Connections</Label>
            <Input
              id="maxConnections"
              type="number"
              min="100"
              max="10000"
              value={nodeSettings.maxConnections}
              onChange={(e) => handleSettingChange('maxConnections', parseInt(e.target.value) || 1000)}
              className="bg-black/20 border-green-500/30 text-white"
            />
            <p className="text-xs text-gray-400">
              Maximum concurrent network connections (100-10,000)
            </p>
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-white">Enable NAT-like Proxy</Label>
                <p className="text-xs text-gray-400">
                  Enable proxy addressing for resource sharing
                </p>
              </div>
              <Switch
                checked={nodeSettings.proxyEnabled}
                onCheckedChange={(checked) => handleSettingChange('proxyEnabled', checked)}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-white">Auto-discovery</Label>
                <p className="text-xs text-gray-400">
                  Automatically discover and connect to nearby nodes
                </p>
              </div>
              <Switch
                checked={nodeSettings.autoDiscovery}
                onCheckedChange={(checked) => handleSettingChange('autoDiscovery', checked)}
              />
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Bandwidth Allocation */}
        <div className="space-y-4">
          <h4 className="text-white font-medium">Bandwidth Allocation</h4>
          
          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex justify-between">
                <Label className="text-white">Upload Bandwidth</Label>
                <span className="text-sm text-gray-400">{nodeSettings.bandwidth.upload} Mbps</span>
              </div>
              <Slider
                value={[nodeSettings.bandwidth.upload]}
                onValueChange={(value) => handleBandwidthChange('upload', value)}
                max={10000}
                min={10}
                step={10}
                className="w-full"
              />
              <div className="flex justify-between text-xs text-gray-400">
                <span>10 Mbps</span>
                <span>10 Gbps</span>
              </div>
            </div>

            <div className="space-y-2">
              <div className="flex justify-between">
                <Label className="text-white">Download Bandwidth</Label>
                <span className="text-sm text-gray-400">{nodeSettings.bandwidth.download} Mbps</span>
              </div>
              <Slider
                value={[nodeSettings.bandwidth.download]}
                onValueChange={(value) => handleBandwidthChange('download', value)}
                max={10000}
                min={10}
                step={10}
                className="w-full"
              />
              <div className="flex justify-between text-xs text-gray-400">
                <span>10 Mbps</span>
                <span>10 Gbps</span>
              </div>
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Action Buttons */}
        <div className="flex items-center justify-between pt-4">
          <Button
            variant="outline"
            onClick={handleReset}
            disabled={loading || !isDirty}
            className="border-gray-500/30 text-gray-400 hover:bg-gray-500/20"
          >
            Reset to Defaults
          </Button>

          <div className="flex items-center space-x-3">
            <Button
              variant="outline"
              onClick={handleTest}
              disabled={loading || !!ipv6Error}
              className="border-blue-500/30 text-blue-400 hover:bg-blue-500/20"
            >
              <TestTube2 className="h-4 w-4 mr-2" />
              Test Configuration
            </Button>
            
            <Button
              onClick={handleSave}
              disabled={loading || !isDirty || !!ipv6Error}
              className="bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-400 hover:to-emerald-500 text-black"
            >
              {loading ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Save className="h-4 w-4 mr-2" />
              )}
              {loading ? 'Saving...' : 'Save Settings'}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}