// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { Shield, RefreshCw, FileText } from 'lucide-react';
import { cn } from '@/lib/utils';
import { SecurityModeSelector } from './SecurityModeSelector';
import { AlgorithmConfiguration } from './AlgorithmConfiguration';
import { SecurityAuditResults } from './SecurityAuditResults';
import { useQuantumSecurity } from './hooks/useQuantumSecurity';

interface SecuritySettings {
  quantumSafe: boolean;
  falconSigning: boolean;
  kyberKeyExchange: boolean;
  tlsVersion: '1.2' | '1.3';
  certificateValidation: 'strict' | 'moderate' | 'permissive';
  ocspStapling: boolean;
  hsts: boolean;
}

interface SecurityAuditResult {
  overallScore: number;
  vulnerabilities: Array<{
    severity: 'high' | 'medium' | 'low';
    category: string;
    description: string;
    recommendation: string;
  }>;
  compliance: {
    quantumResistant: boolean;
    pciCompliant: boolean;
    fipsApproved: boolean;
  };
  recommendations: string[];
}

interface TestCertResult {
  success: boolean;
  certificateDetails: {
    algorithm: string;
    keySize: number;
    validFrom: Date;
    validTo: Date;
    fingerprint: string;
  };
  verificationTests: {
    signatureValid: boolean;
    chainValid: boolean;
    quantumSafe: boolean;
    ocspValid: boolean;
  };
}

interface QuantumSecuritySettingsProps {
  securitySettings: SecuritySettings;
  onSettingsChange: (settings: SecuritySettings) => void;
  onSecurityAudit: () => Promise<SecurityAuditResult>;
  onGenerateTestCert: () => Promise<TestCertResult>;
  onApply: () => Promise<void>;
  isLoading?: boolean;
  auditResults?: SecurityAuditResult;
  testCertResults?: TestCertResult;
  className?: string;
}


export function QuantumSecuritySettings({
  securitySettings,
  onSettingsChange,
  onSecurityAudit,
  onGenerateTestCert,
  onApply,
  isLoading = false,
  auditResults,
  testCertResults,
  className
}: QuantumSecuritySettingsProps) {
  const {
    settings,
    updateSettings,
    expandedSections,
    toggleSection,
    auditing,
    generatingCert,
    applying,
    handleSecurityAudit,
    handleGenerateTestCert,
    handleApply,
    securityScore
  } = useQuantumSecurity({
    initialSettings: securitySettings,
    onSettingsChange,
    onSecurityAudit,
    onGenerateTestCert,
    onApply
  });

  return (
    <Card className={cn("w-full max-w-4xl mx-auto", className)}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <Shield className="h-6 w-6 text-quantum-600" />
            <div>
              <CardTitle>Quantum Security Configuration</CardTitle>
              <CardDescription>
                Configure post-quantum cryptography and security protocols
              </CardDescription>
            </div>
          </div>
          <div className="flex items-center space-x-3">
            <div className="text-right">
              <div className="text-2xl font-bold text-quantum-600">{securityScore}%</div>
              <div className="text-sm text-muted-foreground">Security Score</div>
            </div>
            <Badge
              variant={settings.quantumSafe ? "default" : "destructive"}
              className={settings.quantumSafe ? "bg-quantum-600" : ""}
            >
              {settings.quantumSafe ? "Quantum-Safe" : "Legacy Mode"}
            </Badge>
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-6">
        {/* Security Mode Selection */}
        <SecurityModeSelector 
          settings={settings}
          onSettingsChange={updateSettings}
        />

        {/* Algorithm Configuration */}
        <AlgorithmConfiguration
          settings={settings}
          onSettingsChange={updateSettings}
          expandedSections={expandedSections}
          onToggleSection={toggleSection}
        />

        {/* Security Settings */}
        <div className="space-y-4">
          <h3 className="text-lg font-semibold">Security Configuration</h3>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="tlsVersion">TLS Version</Label>
                <Select
                  value={settings.tlsVersion}
                  onValueChange={(value: '1.2' | '1.3') => updateSettings({ tlsVersion: value })}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="1.3">
                      TLS 1.3 (Recommended)
                    </SelectItem>
                    <SelectItem value="1.2">
                      TLS 1.2 (Legacy)
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="certificateValidation">Certificate Validation</Label>
                <Select
                  value={settings.certificateValidation}
                  onValueChange={(value: 'strict' | 'moderate' | 'permissive') => 
                    updateSettings({ certificateValidation: value })}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="strict">
                      Strict (Recommended)
                    </SelectItem>
                    <SelectItem value="moderate">
                      Moderate
                    </SelectItem>
                    <SelectItem value="permissive">
                      Permissive (Not Recommended)
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label htmlFor="ocspStapling">OCSP Stapling</Label>
                  <p className="text-sm text-muted-foreground">
                    Real-time certificate revocation checking
                  </p>
                </div>
                <Switch
                  id="ocspStapling"
                  checked={settings.ocspStapling}
                  onCheckedChange={(checked) => updateSettings({ ocspStapling: checked })}
                />
              </div>

              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label htmlFor="hsts">HSTS (HTTP Strict Transport Security)</Label>
                  <p className="text-sm text-muted-foreground">
                    Force HTTPS connections for enhanced security
                  </p>
                </div>
                <Switch
                  id="hsts"
                  checked={settings.hsts}
                  onCheckedChange={(checked) => updateSettings({ hsts: checked })}
                />
              </div>
            </div>
          </div>
        </div>

        {/* Security Audit Results */}
        <SecurityAuditResults 
          auditResults={auditResults}
          testCertResults={testCertResults}
        />

        {/* Actions */}
        <div className="flex justify-between items-center pt-6 border-t">
          <Button
            variant="outline"
            onClick={handleSecurityAudit}
            disabled={auditing || isLoading}
          >
            {auditing ? (
              <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Shield className="h-4 w-4 mr-2" />
            )}
            {auditing ? 'Running Audit...' : 'Security Audit'}
          </Button>
          
          <div className="flex space-x-3">
            <Button
              variant="outline"
              onClick={handleGenerateTestCert}
              disabled={generatingCert || isLoading || !settings.quantumSafe}
            >
              {generatingCert ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <FileText className="h-4 w-4 mr-2" />
              )}
              {generatingCert ? 'Generating...' : 'Generate Test Certificate'}
            </Button>
            
            <Button
              onClick={handleApply}
              disabled={applying || isLoading}
            >
              {applying ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Shield className="h-4 w-4 mr-2" />
              )}
              {applying ? 'Applying...' : 'Apply Configuration'}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}