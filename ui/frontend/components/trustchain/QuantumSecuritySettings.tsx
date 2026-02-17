// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Separator } from '@/components/ui/separator';
import { Shield, Lock, Key, Save, RefreshCw, TestTube2 } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface SecuritySettings {
  quantumSafe: boolean;
  falconSigning: boolean;
  kyberKeyExchange: boolean;
  tlsVersion: '1.2' | '1.3';
  certificateValidation: 'strict' | 'moderate' | 'permissive';
  ocspStapling: boolean;
  hsts: boolean;
  cipherSuites: string[];
}

interface QuantumSecuritySettingsProps {
  settings?: SecuritySettings;
  onSave?: (settings: SecuritySettings) => void;
  onTest?: (settings: SecuritySettings) => void;
  onReset?: () => void;
  loading?: boolean;
  className?: string;
}

const defaultSettings: SecuritySettings = {
  quantumSafe: true,
  falconSigning: true,
  kyberKeyExchange: true,
  tlsVersion: '1.3',
  certificateValidation: 'strict',
  ocspStapling: true,
  hsts: true,
  cipherSuites: ['FALCON-1024', 'Kyber-768', 'AES-256-GCM']
};

const tlsVersions = [
  { value: '1.2', label: 'TLS 1.2', deprecated: true },
  { value: '1.3', label: 'TLS 1.3', recommended: true }
];

const validationLevels = [
  { 
    value: 'strict', 
    label: 'Strict', 
    description: 'Full certificate chain validation with OCSP checking',
    recommended: true 
  },
  { 
    value: 'moderate', 
    label: 'Moderate', 
    description: 'Standard validation with relaxed OCSP requirements' 
  },
  { 
    value: 'permissive', 
    label: 'Permissive', 
    description: 'Basic validation for development environments',
    warning: true 
  }
];

export function QuantumSecuritySettings({
  settings = defaultSettings,
  onSave,
  onTest,
  onReset,
  loading = false,
  className
}: QuantumSecuritySettingsProps) {
  const [securitySettings, setSecuritySettings] = useState<SecuritySettings>(settings);
  const [isDirty, setIsDirty] = useState(false);

  const handleSettingChange = useCallback(<K extends keyof SecuritySettings>(
    key: K,
    value: SecuritySettings[K]
  ) => {
    setSecuritySettings(prev => {
      const newSettings = { ...prev, [key]: value };
      
      // Auto-disable dependent features when quantum-safe is disabled
      if (key === 'quantumSafe' && !value) {
        newSettings.falconSigning = false;
        newSettings.kyberKeyExchange = false;
      }
      
      return newSettings;
    });
    setIsDirty(true);
  }, []);

  const handleSave = useCallback(() => {
    onSave?.(securitySettings);
    setIsDirty(false);
  }, [securitySettings, onSave]);

  const handleTest = useCallback(() => {
    onTest?.(securitySettings);
  }, [securitySettings, onTest]);

  const handleReset = useCallback(() => {
    setSecuritySettings(defaultSettings);
    setIsDirty(false);
    onReset?.();
  }, [onReset]);

  const getSecurityLevel = () => {
    if (securitySettings.quantumSafe && securitySettings.falconSigning && securitySettings.kyberKeyExchange) {
      return { level: 'Maximum', color: 'text-green-400', bgColor: 'bg-green-500/20' };
    }
    if (securitySettings.quantumSafe) {
      return { level: 'High', color: 'text-blue-400', bgColor: 'bg-blue-500/20' };
    }
    return { level: 'Standard', color: 'text-yellow-400', bgColor: 'bg-yellow-500/20' };
  };

  const securityLevel = getSecurityLevel();

  return (
    <Card className={cn("bg-black/40 border-green-500/30 backdrop-blur-lg", className)}>
      <CardHeader className="pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Shield className="h-5 w-5 text-purple-400" />
            <CardTitle className="text-white">Quantum Security Settings</CardTitle>
            {isDirty && (
              <Badge variant="outline" className="text-amber-400 border-amber-400/50">
                Unsaved Changes
              </Badge>
            )}
          </div>
          <Badge className={cn(securityLevel.bgColor, securityLevel.color, "border-current/30")}>
            {securityLevel.level} Security
          </Badge>
        </div>
        <CardDescription className="text-gray-400">
          Configure post-quantum cryptography and security protocols
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-6">
        {/* Quantum-Safe Cryptography */}
        <div className="space-y-4">
          <div className="flex items-center justify-between p-4 border border-purple-500/20 rounded-lg bg-purple-500/5">
            <div className="space-y-1">
              <div className="flex items-center space-x-2">
                <Label className="text-white font-medium">Quantum-Safe Cryptography</Label>
                <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30 text-xs">
                  NIST Approved
                </Badge>
              </div>
              <p className="text-sm text-gray-400">
                Enable post-quantum cryptographic algorithms (FALCON-1024, Kyber)
              </p>
            </div>
            <Switch
              checked={securitySettings.quantumSafe}
              onCheckedChange={(checked) => handleSettingChange('quantumSafe', checked)}
            />
          </div>

          {/* Post-Quantum Algorithms */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 ml-4">
            <div className="flex items-center justify-between p-3 border border-green-500/20 rounded-lg bg-green-500/5">
              <div className="space-y-1">
                <div className="flex items-center space-x-2">
                  <Key className="h-4 w-4 text-green-400" />
                  <Label className="text-white">FALCON-1024 Signing</Label>
                </div>
                <p className="text-xs text-gray-400">
                  Post-quantum digital signatures
                </p>
              </div>
              <Switch
                checked={securitySettings.falconSigning}
                onCheckedChange={(checked) => handleSettingChange('falconSigning', checked)}
                disabled={!securitySettings.quantumSafe}
              />
            </div>

            <div className="flex items-center justify-between p-3 border border-blue-500/20 rounded-lg bg-blue-500/5">
              <div className="space-y-1">
                <div className="flex items-center space-x-2">
                  <Lock className="h-4 w-4 text-blue-400" />
                  <Label className="text-white">Kyber Key Exchange</Label>
                </div>
                <p className="text-xs text-gray-400">
                  Quantum-resistant key encapsulation
                </p>
              </div>
              <Switch
                checked={securitySettings.kyberKeyExchange}
                onCheckedChange={(checked) => handleSettingChange('kyberKeyExchange', checked)}
                disabled={!securitySettings.quantumSafe}
              />
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Protocol Configuration */}
        <div className="space-y-4">
          <h4 className="text-white font-medium">Protocol Configuration</h4>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="tlsVersion" className="text-white">TLS Version</Label>
              <Select 
                value={securitySettings.tlsVersion}
                onValueChange={(value: '1.2' | '1.3') => handleSettingChange('tlsVersion', value)}
              >
                <SelectTrigger className="bg-black/20 border-green-500/30 text-white">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-black border-green-500/30">
                  {tlsVersions.map((version) => (
                    <SelectItem key={version.value} value={version.value} className="text-white">
                      <div className="flex items-center space-x-2">
                        <span>{version.label}</span>
                        {version.recommended && (
                          <Badge className="bg-green-500/20 text-green-400 border-green-500/30 text-xs">
                            Recommended
                          </Badge>
                        )}
                        {version.deprecated && (
                          <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30 text-xs">
                            Legacy
                          </Badge>
                        )}
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-gray-400">
                Transport Layer Security protocol version
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="certificateValidation" className="text-white">Certificate Validation</Label>
              <Select 
                value={securitySettings.certificateValidation}
                onValueChange={(value: 'strict' | 'moderate' | 'permissive') => 
                  handleSettingChange('certificateValidation', value)
                }
              >
                <SelectTrigger className="bg-black/20 border-green-500/30 text-white">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-black border-green-500/30">
                  {validationLevels.map((level) => (
                    <SelectItem key={level.value} value={level.value} className="text-white">
                      <div className="space-y-1">
                        <div className="flex items-center space-x-2">
                          <span>{level.label}</span>
                          {level.recommended && (
                            <Badge className="bg-green-500/20 text-green-400 border-green-500/30 text-xs">
                              Recommended
                            </Badge>
                          )}
                          {level.warning && (
                            <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30 text-xs">
                              Caution
                            </Badge>
                          )}
                        </div>
                        <p className="text-xs text-gray-400">{level.description}</p>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Additional Security Features */}
        <div className="space-y-4">
          <h4 className="text-white font-medium">Additional Security Features</h4>
          
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-white">OCSP Stapling</Label>
                <p className="text-sm text-gray-400">
                  Online Certificate Status Protocol for real-time certificate validation
                </p>
              </div>
              <Switch
                checked={securitySettings.ocspStapling}
                onCheckedChange={(checked) => handleSettingChange('ocspStapling', checked)}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label className="text-white">HTTP Strict Transport Security</Label>
                <p className="text-sm text-gray-400">
                  Force HTTPS connections and prevent downgrade attacks
                </p>
              </div>
              <Switch
                checked={securitySettings.hsts}
                onCheckedChange={(checked) => handleSettingChange('hsts', checked)}
              />
            </div>
          </div>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Cipher Suites */}
        <div className="space-y-4">
          <h4 className="text-white font-medium">Active Cipher Suites</h4>
          <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
            {securitySettings.cipherSuites.map((suite) => (
              <Badge
                key={suite}
                variant="outline"
                className="justify-center p-2 text-xs bg-gray-500/10 border-gray-500/30 text-gray-300"
              >
                {suite}
              </Badge>
            ))}
          </div>
          <p className="text-xs text-gray-400">
            Cryptographic protocols used for secure communication
          </p>
        </div>

        <Separator className="bg-green-500/20" />

        {/* Security Status Summary */}
        <div className="p-4 border border-green-500/20 rounded-lg bg-green-500/5">
          <h4 className="text-white font-medium mb-2">Security Status Summary</h4>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <span className="text-gray-400">Quantum Resistant:</span>
              <div className={cn(
                "font-medium",
                securitySettings.quantumSafe ? "text-green-400" : "text-red-400"
              )}>
                {securitySettings.quantumSafe ? "Yes" : "No"}
              </div>
            </div>
            <div>
              <span className="text-gray-400">TLS Version:</span>
              <div className="text-white font-medium">{securitySettings.tlsVersion}</div>
            </div>
            <div>
              <span className="text-gray-400">Validation:</span>
              <div className="text-white font-medium capitalize">{securitySettings.certificateValidation}</div>
            </div>
            <div>
              <span className="text-gray-400">OCSP:</span>
              <div className={cn(
                "font-medium",
                securitySettings.ocspStapling ? "text-green-400" : "text-gray-400"
              )}>
                {securitySettings.ocspStapling ? "Enabled" : "Disabled"}
              </div>
            </div>
          </div>
        </div>

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
              disabled={loading}
              className="border-blue-500/30 text-blue-400 hover:bg-blue-500/20"
            >
              <TestTube2 className="h-4 w-4 mr-2" />
              Test Security
            </Button>
            
            <Button
              onClick={handleSave}
              disabled={loading || !isDirty}
              className="bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-400 hover:to-emerald-500 text-black"
            >
              {loading ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Save className="h-4 w-4 mr-2" />
              )}
              {loading ? 'Saving...' : 'Save Security Settings'}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}