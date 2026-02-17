// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AlertCircle, CheckCircle, Network, RefreshCw, Save, TestTube } from 'lucide-react';
import { cn } from '@/lib/utils';
import { NetworkConfiguration } from './NetworkConfiguration';
import { BandwidthConfiguration } from './BandwidthConfiguration';
import { RegionalConfiguration } from './RegionalConfiguration';
import { useNodeConfiguration } from './hooks/useNodeConfiguration';

interface NodeSettings {
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

interface ConfigTestResult {
  success: boolean;
  tests: {
    ipv6Connectivity: boolean;
    proxyAccess: boolean;
    bandwidthTest: {
      upload: number;
      download: number;
    };
    peerDiscovery: number;
  };
  recommendations: string[];
}

interface NodeConfigurationSettingsProps {
  nodeSettings: NodeSettings;
  onSettingsChange: (settings: NodeSettings) => void;
  onTest: () => Promise<ConfigTestResult>;
  onSave: () => Promise<void>;
  onReset: () => void;
  isLoading?: boolean;
  testResults?: ConfigTestResult;
  className?: string;
}


export function NodeConfigurationSettings({
  nodeSettings,
  onSettingsChange,
  onTest,
  onSave,
  onReset,
  isLoading = false,
  testResults,
  className
}: NodeConfigurationSettingsProps) {
  const {
    settings,
    updateSettings,
    activeTab,
    setActiveTab,
    testing,
    saving,
    validationErrors,
    handleTest,
    handleSave,
    handleReset
  } = useNodeConfiguration({
    initialSettings: nodeSettings,
    onSettingsChange,
    onTest,
    onSave,
    onReset
  });

  return (
    <Card className={cn("w-full max-w-4xl mx-auto", className)}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <Network className="h-6 w-6 text-blue-600" />
            <div>
              <CardTitle>Node Configuration Settings</CardTitle>
              <CardDescription>
                Configure your HyperMesh node networking and performance settings
              </CardDescription>
            </div>
          </div>
          <div className="flex items-center space-x-2">
            <Badge variant="outline" className="text-green-600 border-green-600">
              Online
            </Badge>
            <div className="text-sm text-muted-foreground">
              Uptime: 15d 4h 23m
            </div>
          </div>
        </div>
      </CardHeader>

      <CardContent>
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="basic">Basic Setup</TabsTrigger>
            <TabsTrigger value="network">Networking</TabsTrigger>
            <TabsTrigger value="performance">Performance</TabsTrigger>
          </TabsList>

          <TabsContent value="basic" className="space-y-6">
            <RegionalConfiguration
              settings={settings}
              onSettingsChange={updateSettings}
              validationErrors={validationErrors}
            />
            
            {/* Status Information */}
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Node Status</h3>
              
              <div className="p-4 bg-green-50 border border-green-200 rounded-lg">
                <div className="flex items-center space-x-2 mb-2">
                  <div className="w-3 h-3 bg-green-500 rounded-full"></div>
                  <span className="font-medium text-green-800">Online</span>
                </div>
                <div className="text-sm text-green-700 space-y-1">
                  <div>Uptime: 15 days, 4 hours, 23 minutes</div>
                  <div>Last restart: 2024-01-01 08:15:30 UTC</div>
                  <div>Version: v2.1.0-quantum</div>
                </div>
              </div>

              <div className="space-y-3">
                <h4 className="font-medium">Quick Stats</h4>
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <div className="text-muted-foreground">Active Connections</div>
                    <div className="font-medium">247 / {settings.maxConnections}</div>
                  </div>
                  <div>
                    <div className="text-muted-foreground">Network Load</div>
                    <div className="font-medium">34.2%</div>
                  </div>
                  <div>
                    <div className="text-muted-foreground">Data Processed</div>
                    <div className="font-medium">4.2 TB</div>
                  </div>
                  <div>
                    <div className="text-muted-foreground">Peer Score</div>
                    <div className="font-medium">97.8%</div>
                  </div>
                </div>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="network" className="space-y-6">
            <NetworkConfiguration
              settings={settings}
              onSettingsChange={updateSettings}
              validationErrors={validationErrors}
            />
          </TabsContent>

          <TabsContent value="performance" className="space-y-6">
            <BandwidthConfiguration
              settings={settings}
              onSettingsChange={updateSettings}
            />
          </TabsContent>
        </Tabs>

        {/* Test Results */}
        {testResults && (
          <Card className="mt-6">
            <CardHeader>
              <CardTitle className="flex items-center space-x-2">
                <TestTube className="h-5 w-5" />
                <span>Configuration Test Results</span>
                {testResults.success ? (
                  <CheckCircle className="h-5 w-5 text-green-600" />
                ) : (
                  <AlertCircle className="h-5 w-5 text-red-600" />
                )}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span>IPv6 Connectivity</span>
                    {testResults.tests.ipv6Connectivity ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertCircle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex items-center justify-between">
                    <span>Proxy Access</span>
                    {testResults.tests.proxyAccess ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertCircle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex items-center justify-between">
                    <span>Peer Discovery</span>
                    <span className="text-sm">{testResults.tests.peerDiscovery} peers found</span>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span>Upload Speed</span>
                    <span className="text-sm">{testResults.tests.bandwidthTest.upload} Mbps</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span>Download Speed</span>
                    <span className="text-sm">{testResults.tests.bandwidthTest.download} Mbps</span>
                  </div>
                </div>
              </div>
              
              {testResults.recommendations.length > 0 && (
                <div className="mt-4">
                  <h4 className="font-medium mb-2">Recommendations</h4>
                  <ul className="text-sm text-muted-foreground space-y-1">
                    {testResults.recommendations.map((rec, index) => (
                      <li key={index} className="flex items-start">
                        <span className="mr-2">•</span>
                        {rec}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </CardContent>
          </Card>
        )}

        {/* Actions */}
        <div className="flex justify-between items-center mt-6 pt-6 border-t">
          <Button variant="outline" onClick={handleReset}>
            Reset to Defaults
          </Button>
          
          <div className="flex space-x-3">
            <Button
              variant="outline"
              onClick={handleTest}
              disabled={testing || isLoading}
            >
              {testing ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <TestTube className="h-4 w-4 mr-2" />
              )}
              {testing ? 'Testing...' : 'Test Configuration'}
            </Button>
            
            <Button
              onClick={handleSave}
              disabled={saving || isLoading || Object.keys(validationErrors).length > 0}
            >
              {saving ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Save className="h-4 w-4 mr-2" />
              )}
              {saving ? 'Saving...' : 'Save Settings'}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}