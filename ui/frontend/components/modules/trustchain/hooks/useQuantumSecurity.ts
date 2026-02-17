// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useState, useEffect } from 'react';

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

interface UseQuantumSecurityProps {
  initialSettings: SecuritySettings;
  onSettingsChange: (settings: SecuritySettings) => void;
  onSecurityAudit: () => Promise<SecurityAuditResult>;
  onGenerateTestCert: () => Promise<TestCertResult>;
  onApply: () => Promise<void>;
}

export function useQuantumSecurity({
  initialSettings,
  onSettingsChange,
  onSecurityAudit,
  onGenerateTestCert,
  onApply
}: UseQuantumSecurityProps) {
  const [settings, setSettings] = useState<SecuritySettings>(initialSettings);
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({});
  const [auditing, setAuditing] = useState(false);
  const [generatingCert, setGeneratingCert] = useState(false);
  const [applying, setApplying] = useState(false);

  useEffect(() => {
    setSettings(initialSettings);
  }, [initialSettings]);

  const updateSettings = (updates: Partial<SecuritySettings>) => {
    const newSettings = { ...settings, ...updates };
    
    // Auto-disable dependent features when quantum-safe is disabled
    if ('quantumSafe' in updates && !updates.quantumSafe) {
      newSettings.falconSigning = false;
      newSettings.kyberKeyExchange = false;
    }
    
    setSettings(newSettings);
    onSettingsChange(newSettings);
  };

  const toggleSection = (section: string) => {
    setExpandedSections(prev => ({
      ...prev,
      [section]: !prev[section]
    }));
  };

  const handleSecurityAudit = async () => {
    setAuditing(true);
    try {
      return await onSecurityAudit();
    } finally {
      setAuditing(false);
    }
  };

  const handleGenerateTestCert = async () => {
    setGeneratingCert(true);
    try {
      return await onGenerateTestCert();
    } finally {
      setGeneratingCert(false);
    }
  };

  const handleApply = async () => {
    setApplying(true);
    try {
      await onApply();
    } finally {
      setApplying(false);
    }
  };

  const getSecurityScore = () => {
    let score = 0;
    if (settings.quantumSafe) score += 40;
    if (settings.falconSigning) score += 20;
    if (settings.kyberKeyExchange) score += 20;
    if (settings.tlsVersion === '1.3') score += 10;
    if (settings.certificateValidation === 'strict') score += 5;
    if (settings.ocspStapling) score += 3;
    if (settings.hsts) score += 2;
    return score;
  };

  return {
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
    securityScore: getSecurityScore()
  };
}