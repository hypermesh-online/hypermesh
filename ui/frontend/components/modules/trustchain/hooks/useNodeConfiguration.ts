// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useState, useEffect } from 'react';

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

interface UseNodeConfigurationProps {
  initialSettings: NodeSettings;
  onSettingsChange: (settings: NodeSettings) => void;
  onTest: () => Promise<ConfigTestResult>;
  onSave: () => Promise<void>;
  onReset: () => void;
}

function validateIPv6(address: string): boolean {
  const ipv6Regex = /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:)*::([0-9a-fA-F]{1,4}:)*[0-9a-fA-F]{1,4}$|^::([0-9a-fA-F]{1,4}:)*[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:)+::$|^::$/;
  return ipv6Regex.test(address);
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

export function useNodeConfiguration({
  initialSettings,
  onSettingsChange,
  onTest,
  onSave,
  onReset
}: UseNodeConfigurationProps) {
  const [settings, setSettings] = useState<NodeSettings>(initialSettings);
  const [activeTab, setActiveTab] = useState('basic');
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    setSettings(initialSettings);
  }, [initialSettings]);

  const updateSettings = (updates: Partial<NodeSettings>) => {
    const newSettings = { ...settings, ...updates };
    setSettings(newSettings);
    onSettingsChange(newSettings);
    
    // Clear validation errors for updated fields
    const newErrors = { ...validationErrors };
    Object.keys(updates).forEach(key => {
      delete newErrors[key];
    });
    setValidationErrors(newErrors);
  };

  const validateSettings = (): boolean => {
    const errors: Record<string, string> = {};
    
    if (!settings.nodeId || settings.nodeId.trim().length < 3) {
      errors.nodeId = 'Node ID must be at least 3 characters';
    }
    
    if (!validateIPv6(settings.ipv6Address)) {
      errors.ipv6Address = 'Invalid IPv6 address format';
    }
    
    if (settings.maxConnections < 100 || settings.maxConnections > 10000) {
      errors.maxConnections = 'Max connections must be between 100 and 10,000';
    }
    
    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleTest = async () => {
    if (!validateSettings()) return;
    
    setTesting(true);
    try {
      return await onTest();
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async () => {
    if (!validateSettings()) return;
    
    setSaving(true);
    try {
      await onSave();
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    setSettings(defaultSettings);
    setValidationErrors({});
    onReset();
  };

  return {
    settings,
    updateSettings,
    activeTab,
    setActiveTab,
    testing,
    saving,
    validationErrors,
    handleTest,
    handleSave,
    handleReset,
    validateSettings
  };
}