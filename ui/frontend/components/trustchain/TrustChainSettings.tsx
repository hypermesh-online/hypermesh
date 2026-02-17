// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { NodeConfigurationSettings } from './NodeConfigurationSettings';
import { QuantumSecuritySettings } from './QuantumSecuritySettings';

export function TrustChainSettings() {
  const [saving, setSaving] = React.useState(false);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white">TrustChain Settings</h2>
        <p className="text-gray-400 mt-1">
          Configure node settings, quantum security, and system preferences
        </p>
      </div>

      <Tabs defaultValue="node" className="w-full">
        <TabsList className="grid w-full grid-cols-2 bg-black/20">
          <TabsTrigger value="node" className="text-white">Node Configuration</TabsTrigger>
          <TabsTrigger value="security" className="text-white">Quantum Security</TabsTrigger>
        </TabsList>

        <TabsContent value="node" className="mt-6">
          <NodeConfigurationSettings
            onSave={(settings) => {
              setSaving(true);
              setTimeout(() => {
                setSaving(false);
                console.log('Node settings saved:', settings);
              }, 1000);
            }}
            onTest={(settings) => {
              console.log('Testing node configuration:', settings);
            }}
            onReset={() => {
              console.log('Reset node settings to defaults');
            }}
            loading={saving}
          />
        </TabsContent>

        <TabsContent value="security" className="mt-6">
          <QuantumSecuritySettings
            onSave={(settings) => {
              setSaving(true);
              setTimeout(() => {
                setSaving(false);
                console.log('Security settings saved:', settings);
              }, 1000);
            }}
            onTest={(settings) => {
              console.log('Testing security configuration:', settings);
            }}
            onReset={() => {
              console.log('Reset security settings to defaults');
            }}
            loading={saving}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}