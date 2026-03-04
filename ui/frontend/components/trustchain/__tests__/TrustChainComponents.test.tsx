// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';

import {
  NodeConfigurationSettings,
  QuantumSecuritySettings,
  StateProofMetricsPanel,
  CertificateDetailsPanel,
  EcosystemMetricsDashboard
} from '../index';

// Mock components and hooks
vi.mock('@/components/ui/card', () => ({
  Card: ({ children, className }: any) => <div className={className}>{children}</div>,
  CardContent: ({ children }: any) => <div>{children}</div>,
  CardDescription: ({ children }: any) => <div>{children}</div>,
  CardHeader: ({ children }: any) => <div>{children}</div>,
  CardTitle: ({ children }: any) => <div>{children}</div>
}));

vi.mock('@/components/ui/button', () => ({
  Button: ({ children, onClick, disabled, className }: any) => (
    <button onClick={onClick} disabled={disabled} className={className}>
      {children}
    </button>
  )
}));

vi.mock('@/components/ui/input', () => ({
  Input: ({ value, onChange, className, ...props }: any) => (
    <input 
      value={value} 
      onChange={onChange} 
      className={className} 
      {...props}
    />
  )
}));

vi.mock('@/components/ui/switch', () => ({
  Switch: ({ checked, onCheckedChange }: any) => (
    <input 
      type="checkbox" 
      checked={checked} 
      onChange={(e) => onCheckedChange(e.target.checked)}
    />
  )
}));

vi.mock('@/components/ui/progress', () => ({
  Progress: ({ value, className }: any) => (
    <div className={className} data-testid="progress" data-value={value}>
      Progress: {value}%
    </div>
  )
}));

vi.mock('@/components/ui/badge', () => ({
  Badge: ({ children, className }: any) => (
    <span className={className}>{children}</span>
  )
}));

vi.mock('@/components/ui/tabs', () => ({
  Tabs: ({ children, value, onValueChange }: any) => (
    <div data-testid="tabs" data-value={value}>
      {children}
    </div>
  ),
  TabsList: ({ children }: any) => <div>{children}</div>,
  TabsTrigger: ({ children, value, onClick }: any) => (
    <button onClick={() => onClick?.(value)}>{children}</button>
  ),
  TabsContent: ({ children, value }: any) => <div data-value={value}>{children}</div>
}));

vi.mock('@/components/ui/select', () => ({
  Select: ({ children, value, onValueChange }: any) => (
    <div data-testid="select" data-value={value}>
      {children}
    </div>
  ),
  SelectContent: ({ children }: any) => <div>{children}</div>,
  SelectItem: ({ children, value }: any) => (
    <option value={value}>{children}</option>
  ),
  SelectTrigger: ({ children }: any) => <div>{children}</div>,
  SelectValue: () => <div>Select Value</div>
}));

vi.mock('@/components/ui/slider', () => ({
  Slider: ({ value, onValueChange, max, min }: any) => (
    <input 
      type="range"
      value={value[0]}
      onChange={(e) => onValueChange([parseInt(e.target.value)])}
      max={max}
      min={min}
      data-testid="slider"
    />
  )
}));

vi.mock('lucide-react', () => ({
  Network: () => <div>Network Icon</div>,
  Shield: () => <div>Shield Icon</div>,
  Save: () => <div>Save Icon</div>,
  RefreshCw: () => <div>Refresh Icon</div>,
  TestTube2: () => <div>Test Icon</div>,
  Key: () => <div>Key Icon</div>,
  Lock: () => <div>Lock Icon</div>,
  Activity: () => <div>Activity Icon</div>,
  CheckCircle: () => <div>Check Icon</div>,
  Database: () => <div>Database Icon</div>,
  Clock: () => <div>Clock Icon</div>,
  Zap: () => <div>Zap Icon</div>,
  HardDrive: () => <div>HardDrive Icon</div>,
  Coins: () => <div>Coins Icon</div>,
  TrendingUp: () => <div>TrendingUp Icon</div>,
  TrendingDown: () => <div>TrendingDown Icon</div>,
  Minus: () => <div>Minus Icon</div>,
  AlertTriangle: () => <div>Alert Icon</div>,
  XCircle: () => <div>XCircle Icon</div>,
  Globe: () => <div>Globe Icon</div>,
  Copy: () => <div>Copy Icon</div>,
  Eye: () => <div>Eye Icon</div>,
  EyeOff: () => <div>EyeOff Icon</div>,
  Download: () => <div>Download Icon</div>,
  Trash2: () => <div>Trash Icon</div>,
  ExternalLink: () => <div>External Icon</div>,
  Plus: () => <div>Plus Icon</div>,
  Separator: () => <div className="separator" />,
  Users: () => <div>Users Icon</div>
}));

vi.mock('@/lib/utils', () => ({
  cn: (...args: any[]) => args.filter(Boolean).join(' ')
}));

describe('TrustChain Components', () => {
  describe('NodeConfigurationSettings', () => {
    it('renders node configuration form', () => {
      const mockOnSave = vi.fn();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      expect(screen.getByText('Node Configuration')).toBeInTheDocument();
      expect(screen.getByDisplayValue('node-001')).toBeInTheDocument();
      expect(screen.getByDisplayValue('2001:db8::1001')).toBeInTheDocument();
    });

    it('handles settings changes', async () => {
      const mockOnSave = vi.fn();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const nodeIdInput = screen.getByDisplayValue('node-001');
      fireEvent.change(nodeIdInput, { target: { value: 'node-002' } });
      
      const saveButton = screen.getByText('Save Settings');
      fireEvent.click(saveButton);
      
      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({
            nodeId: 'node-002'
          })
        );
      });
    });

    it('validates IPv6 addresses', () => {
      render(<NodeConfigurationSettings />);
      
      const ipv6Input = screen.getByDisplayValue('2001:db8::1001');
      fireEvent.change(ipv6Input, { target: { value: 'invalid-ipv6' } });
      
      expect(screen.getByText('Invalid IPv6 address format')).toBeInTheDocument();
    });
  });

  describe('QuantumSecuritySettings', () => {
    it('renders quantum security options', () => {
      render(<QuantumSecuritySettings />);
      
      expect(screen.getByText('Quantum Security Settings')).toBeInTheDocument();
      expect(screen.getByText('Quantum-Safe Cryptography')).toBeInTheDocument();
      expect(screen.getByText('FALCON-1024 Signing')).toBeInTheDocument();
      expect(screen.getByText('Kyber Key Exchange')).toBeInTheDocument();
    });

    it('disables dependent features when quantum-safe is disabled', async () => {
      render(<QuantumSecuritySettings />);
      
      const quantumSafeSwitch = screen.getAllByRole('checkbox')[0];
      fireEvent.click(quantumSafeSwitch);
      
      // FALCON and Kyber switches should be disabled
      const switches = screen.getAllByRole('checkbox');
      expect(switches[1]).toBeDisabled(); // FALCON
      expect(switches[2]).toBeDisabled(); // Kyber
    });
  });

  describe('StateProofMetricsPanel', () => {
    it('renders state proof metrics', () => {
      render(<StateProofMetricsPanel />);

      expect(screen.getByText('Four-Proof State Verification System')).toBeInTheDocument();
      expect(screen.getByText('15,234')).toBeInTheDocument(); // Block height
      expect(screen.getByText('2.3s')).toBeInTheDocument(); // Block time
      expect(screen.getByText('847')).toBeInTheDocument(); // TPS
      expect(screen.getByText('67')).toBeInTheDocument(); // Validators
    });

    it('displays four proof types', () => {
      render(<StateProofMetricsPanel />);

      expect(screen.getByText('SPACE')).toBeInTheDocument();
      expect(screen.getByText('STAKE')).toBeInTheDocument();
      expect(screen.getByText('WORK')).toBeInTheDocument();
      expect(screen.getByText('TIME')).toBeInTheDocument();
    });

    it('handles refresh action', () => {
      const mockOnRefresh = vi.fn();
      render(<StateProofMetricsPanel onRefresh={mockOnRefresh} />);

      // Should call refresh automatically with autoRefresh
      expect(mockOnRefresh).toHaveBeenCalled();
    });
  });

  describe('CertificateDetailsPanel', () => {
    const mockCertificate = {
      id: 'cert-001',
      subject: 'CN=test.example.com',
      issuer: 'CN=Test CA',
      serialNumber: '123456',
      validFrom: '2024-01-01T00:00:00Z',
      validTo: '2025-12-31T23:59:59Z',
      fingerprint: 'A1B2C3D4E5F6789012345678901234567890123456789012345678901234567890',
      publicKey: 'MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...',
      status: 'active' as const,
      trustLevel: 'leaf' as const,
      keyAlgorithm: 'FALCON-1024',
      signatureAlgorithm: 'FALCON-1024-SHA256',
      extensions: [
        {
          oid: '2.5.29.15',
          critical: true,
          value: 'Digital Signature, Key Encipherment'
        }
      ]
    };

    it('renders certificate details', () => {
      render(<CertificateDetailsPanel certificate={mockCertificate} />);
      
      expect(screen.getByText('CN=test.example.com')).toBeInTheDocument();
      expect(screen.getByText('cert-001')).toBeInTheDocument();
      expect(screen.getByText('FALCON-1024')).toBeInTheDocument();
    });

    it('shows different tabs', () => {
      render(<CertificateDetailsPanel certificate={mockCertificate} />);
      
      expect(screen.getByText('Overview')).toBeInTheDocument();
      expect(screen.getByText('Details')).toBeInTheDocument();
      expect(screen.getByText('Extensions')).toBeInTheDocument();
      expect(screen.getByText('Validation')).toBeInTheDocument();
    });

    it('handles export action', () => {
      const mockOnExport = vi.fn();
      render(
        <CertificateDetailsPanel 
          certificate={mockCertificate} 
          onExport={mockOnExport}
        />
      );
      
      const exportButton = screen.getByText('Export');
      fireEvent.click(exportButton);
      
      expect(mockOnExport).toHaveBeenCalledWith('pem');
    });
  });

  describe('EcosystemMetricsDashboard', () => {
    it('renders ecosystem metrics', () => {
      render(<EcosystemMetricsDashboard />);
      
      expect(screen.getByText('Web3 Ecosystem Dashboard')).toBeInTheDocument();
      expect(screen.getByText('1,247')).toBeInTheDocument(); // Total assets
      expect(screen.getByText('892')).toBeInTheDocument(); // Active certificates
      expect(screen.getByText('2.95 Gbps')).toBeInTheDocument(); // Network throughput
    });

    it('shows system health overview', () => {
      render(<EcosystemMetricsDashboard />);
      
      expect(screen.getByText('System Health Overview')).toBeInTheDocument();
      expect(screen.getByText('TrustChain CA')).toBeInTheDocument();
      expect(screen.getByText('STOQ Protocol')).toBeInTheDocument();
    });

    it('handles refresh action', () => {
      const mockOnRefresh = vi.fn();
      render(<EcosystemMetricsDashboard onRefresh={mockOnRefresh} />);
      
      const refreshButton = screen.getByText('Refresh');
      fireEvent.click(refreshButton);
      
      expect(mockOnRefresh).toHaveBeenCalled();
    });
  });
});