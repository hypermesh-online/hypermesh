// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { AlertCircle, CheckCircle } from 'lucide-react';

interface NodeSettings {
  ipv6Address: string;
  proxyEnabled: boolean;
  autoDiscovery: boolean;
}

interface NetworkConfigurationProps {
  settings: NodeSettings;
  onSettingsChange: (updates: Partial<NodeSettings>) => void;
  validationErrors: Record<string, string>;
}

function validateIPv6(address: string): boolean {
  const ipv6Regex = /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:)*::([0-9a-fA-F]{1,4}:)*[0-9a-fA-F]{1,4}$|^::([0-9a-fA-F]{1,4}:)*[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:)+::$|^::$/;
  return ipv6Regex.test(address);
}

export function NetworkConfiguration({
  settings,
  onSettingsChange,
  validationErrors
}: NetworkConfigurationProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      {/* IPv6 Configuration */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold">IPv6 Configuration</h3>
        
        <div className="space-y-2">
          <Label htmlFor="ipv6Address">IPv6 Address</Label>
          <Input
            id="ipv6Address"
            value={settings.ipv6Address}
            onChange={(e) => onSettingsChange({ ipv6Address: e.target.value })}
            placeholder="2001:db8::1001"
            className={validationErrors.ipv6Address ? 'border-red-500' : 
                      validateIPv6(settings.ipv6Address) ? 'border-green-500' : ''}
            aria-describedby={validationErrors.ipv6Address ? 'ipv6-error' : 
                            validateIPv6(settings.ipv6Address) ? 'ipv6-success' : undefined}
          />
          {validationErrors.ipv6Address && (
            <p id="ipv6-error" className="text-sm text-red-600 flex items-center">
              <AlertCircle className="h-4 w-4 mr-1" />
              {validationErrors.ipv6Address}
            </p>
          )}
          {!validationErrors.ipv6Address && validateIPv6(settings.ipv6Address) && (
            <p id="ipv6-success" className="text-sm text-green-600 flex items-center">
              <CheckCircle className="h-4 w-4 mr-1" />
              Valid IPv6 format
            </p>
          )}
        </div>

        <div className="flex gap-2">
          <Button variant="outline" size="sm">
            Auto-Configure
          </Button>
          <Button variant="outline" size="sm">
            Import Config
          </Button>
        </div>
      </div>

      {/* Proxy & Discovery Settings */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold">Proxy & Discovery</h3>
        
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label htmlFor="proxyEnabled">NAT-like Proxy</Label>
              <p className="text-sm text-muted-foreground">
                Enable remote resource access through trusted nodes
              </p>
            </div>
            <Switch
              id="proxyEnabled"
              checked={settings.proxyEnabled}
              onCheckedChange={(checked) => onSettingsChange({ proxyEnabled: checked })}
            />
          </div>

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label htmlFor="autoDiscovery">Auto-Discovery</Label>
              <p className="text-sm text-muted-foreground">
                Automatically find and connect to peer nodes
              </p>
            </div>
            <Switch
              id="autoDiscovery"
              checked={settings.autoDiscovery}
              onCheckedChange={(checked) => onSettingsChange({ autoDiscovery: checked })}
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label>Trusted Networks</Label>
          <div className="space-y-2">
            <div className="flex items-center justify-between p-2 bg-muted rounded">
              <code className="text-sm">2001:db8::/32</code>
              <Badge variant="secondary">Default</Badge>
            </div>
            <Button variant="outline" size="sm" className="w-full">
              + Add Network
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}