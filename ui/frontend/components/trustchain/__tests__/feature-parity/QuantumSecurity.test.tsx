// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  QuantumSecuritySettings,
  type SecuritySettings,
  type SecurityAuditResult,
  type TestCertResult,
} from '../../../modules/trustchain/QuantumSecuritySettings';

/**
 * Feature Parity Tests for QuantumSecuritySettings
 * Validates React component matches Svelte settings.svelte (lines 24-32, 219-287)
 */

const defaultSecuritySettings: SecuritySettings = {
  quantumSafe: true,
  falconSigning: true,
  kyberKeyExchange: true,
  tlsVersion: '1.3',
  certificateValidation: 'strict',
  ocspStapling: true,
  hsts: true,
};

const defaultAuditResult: SecurityAuditResult = {
  overallScore: 95,
  vulnerabilities: [],
  compliance: {
    quantumResistant: true,
    pciCompliant: true,
    fipsApproved: true,
  },
  recommendations: [],
};

const defaultTestCertResult: TestCertResult = {
  success: true,
  certificateDetails: {
    algorithm: 'FALCON-1024',
    keySize: 1024,
    validFrom: new Date(),
    validTo: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000),
    fingerprint: 'abc123',
  },
  verificationTests: {
    signatureValid: true,
    chainValid: true,
    quantumSafe: true,
    ocspValid: true,
  },
};

function renderQuantumSecurity(overrides: Partial<React.ComponentProps<typeof QuantumSecuritySettings>> = {}) {
  const defaultProps: React.ComponentProps<typeof QuantumSecuritySettings> = {
    securitySettings: defaultSecuritySettings,
    onSettingsChange: vi.fn(),
    onSecurityAudit: vi.fn().mockResolvedValue(defaultAuditResult),
    onGenerateTestCert: vi.fn().mockResolvedValue(defaultTestCertResult),
    onApply: vi.fn().mockResolvedValue(undefined),
  };

  return render(<QuantumSecuritySettings {...defaultProps} {...overrides} />);
}

describe('QuantumSecuritySettings - Feature Parity', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Default Values Match Svelte Implementation', () => {
    it('should render with correct default security settings', () => {
      renderQuantumSecurity();

      // Quantum-safe cryptography - matches settings.svelte line 25
      const quantumSafeSwitch = screen.getByRole('switch', { name: /quantum-safe cryptography/i });
      expect(quantumSafeSwitch).toBeChecked();

      // FALCON signing - matches settings.svelte line 26
      const falconSwitch = screen.getByRole('switch', { name: /falcon-1024 signing/i });
      expect(falconSwitch).toBeChecked();

      // Kyber key exchange - matches settings.svelte line 27
      const kyberSwitch = screen.getByRole('switch', { name: /kyber key exchange/i });
      expect(kyberSwitch).toBeChecked();

      // OCSP stapling - matches settings.svelte line 30
      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });
      expect(ocspSwitch).toBeChecked();

      // HSTS - matches settings.svelte line 31
      const hstsSwitch = screen.getByRole('switch', { name: /http strict transport security/i });
      expect(hstsSwitch).toBeChecked();
    });

    it('should display correct TLS version default', () => {
      renderQuantumSecurity();

      // TLS version - matches settings.svelte line 28 (default '1.3')
      expect(screen.getByDisplayValue('1.3')).toBeInTheDocument();
    });

    it('should display correct certificate validation default', () => {
      renderQuantumSecurity();

      // Certificate validation - matches settings.svelte line 29 (default 'strict')
      expect(screen.getByDisplayValue('strict')).toBeInTheDocument();
    });

    it('should display security level indicator', () => {
      renderQuantumSecurity();

      // Should show maximum security with all quantum features enabled
      expect(screen.getByText('Maximum Security')).toBeInTheDocument();
    });
  });

  describe('Quantum-Safe Cryptography Master Toggle', () => {
    it('should disable dependent features when quantum-safe is disabled', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      const quantumSafeSwitch = screen.getByRole('switch', { name: /quantum-safe cryptography/i });
      const falconSwitch = screen.getByRole('switch', { name: /falcon-1024 signing/i });
      const kyberSwitch = screen.getByRole('switch', { name: /kyber key exchange/i });

      // Initially enabled
      expect(quantumSafeSwitch).toBeChecked();
      expect(falconSwitch).toBeChecked();
      expect(kyberSwitch).toBeChecked();
      expect(falconSwitch).toBeEnabled();
      expect(kyberSwitch).toBeEnabled();

      // Disable quantum-safe
      await user.click(quantumSafeSwitch);

      await waitFor(() => {
        expect(quantumSafeSwitch).not.toBeChecked();
        expect(falconSwitch).not.toBeChecked();
        expect(kyberSwitch).not.toBeChecked();
        expect(falconSwitch).toBeDisabled();
        expect(kyberSwitch).toBeDisabled();
      });
    });

    it('should update security level when quantum features change', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      // Initially Maximum security
      expect(screen.getByText('Maximum Security')).toBeInTheDocument();

      // Disable quantum-safe
      const quantumSafeSwitch = screen.getByRole('switch', { name: /quantum-safe cryptography/i });
      await user.click(quantumSafeSwitch);

      await waitFor(() => {
        expect(screen.getByText('Standard Security')).toBeInTheDocument();
      });
    });
  });

  describe('Post-Quantum Algorithm Configuration', () => {
    it('should handle FALCON-1024 signing toggle', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      const falconSwitch = screen.getByRole('switch', { name: /falcon-1024 signing/i });

      // Initially enabled
      expect(falconSwitch).toBeChecked();

      // Disable FALCON
      await user.click(falconSwitch);

      await waitFor(() => {
        expect(falconSwitch).not.toBeChecked();
        // Should still show High security (quantum-safe still enabled)
        expect(screen.getByText('High Security')).toBeInTheDocument();
      });
    });

    it('should handle Kyber key exchange toggle', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      const kyberSwitch = screen.getByRole('switch', { name: /kyber key exchange/i });

      // Initially enabled
      expect(kyberSwitch).toBeChecked();

      // Disable Kyber
      await user.click(kyberSwitch);

      await waitFor(() => {
        expect(kyberSwitch).not.toBeChecked();
        expect(screen.getByText('High Security')).toBeInTheDocument();
      });
    });

    it('should show correct algorithm descriptions', () => {
      renderQuantumSecurity();

      expect(screen.getByText('Post-quantum digital signatures')).toBeInTheDocument();
      expect(screen.getByText('Quantum-resistant key encapsulation')).toBeInTheDocument();
    });
  });

  describe('Protocol Configuration', () => {
    it('should handle TLS version selection', () => {
      renderQuantumSecurity();

      // TLS version select should be present with correct default
      expect(screen.getByDisplayValue('1.3')).toBeInTheDocument();

      // Should show recommended badge for TLS 1.3
      expect(screen.getByText('Recommended')).toBeInTheDocument();
    });

    it('should handle certificate validation level selection', () => {
      renderQuantumSecurity();

      // Certificate validation should be present with correct default
      expect(screen.getByDisplayValue('strict')).toBeInTheDocument();
    });
  });

  describe('Additional Security Features', () => {
    it('should handle OCSP stapling toggle', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });

      // Initially enabled
      expect(ocspSwitch).toBeChecked();

      // Toggle OCSP
      await user.click(ocspSwitch);

      await waitFor(() => {
        expect(ocspSwitch).not.toBeChecked();
      });
    });

    it('should handle HSTS toggle', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      const hstsSwitch = screen.getByRole('switch', { name: /http strict transport security/i });

      // Initially enabled
      expect(hstsSwitch).toBeChecked();

      // Toggle HSTS
      await user.click(hstsSwitch);

      await waitFor(() => {
        expect(hstsSwitch).not.toBeChecked();
      });
    });

    it('should display feature descriptions', () => {
      renderQuantumSecurity();

      expect(screen.getByText(/online certificate status protocol/i)).toBeInTheDocument();
      expect(screen.getByText(/force https connections/i)).toBeInTheDocument();
    });
  });

  describe('Cipher Suites Display', () => {
    it('should display active cipher suites', () => {
      renderQuantumSecurity();

      // Default cipher suites should be displayed
      expect(screen.getByText('FALCON-1024')).toBeInTheDocument();
      expect(screen.getByText('Kyber-768')).toBeInTheDocument();
      expect(screen.getByText('AES-256-GCM')).toBeInTheDocument();
    });

    it('should show cipher suites description', () => {
      renderQuantumSecurity();

      expect(screen.getByText(/cryptographic protocols used for secure communication/i)).toBeInTheDocument();
    });
  });

  describe('Security Status Summary', () => {
    it('should display comprehensive security status', () => {
      renderQuantumSecurity();

      // Check status indicators
      expect(screen.getByText('Quantum Resistant:')).toBeInTheDocument();
      expect(screen.getByText('Yes')).toBeInTheDocument();
      expect(screen.getByText('TLS Version:')).toBeInTheDocument();
      expect(screen.getByText('1.3')).toBeInTheDocument();
      expect(screen.getByText('Validation:')).toBeInTheDocument();
      expect(screen.getByText('Strict')).toBeInTheDocument();
      expect(screen.getByText('OCSP:')).toBeInTheDocument();
      expect(screen.getByText('Enabled')).toBeInTheDocument();
    });

    it('should update status when quantum resistance is disabled', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      // Initially quantum resistant
      expect(screen.getByText('Yes')).toBeInTheDocument();

      // Disable quantum-safe
      const quantumSafeSwitch = screen.getByRole('switch', { name: /quantum-safe cryptography/i });
      await user.click(quantumSafeSwitch);

      await waitFor(() => {
        expect(screen.getByText('No')).toBeInTheDocument();
      });
    });
  });

  describe('State Management and Actions', () => {
    it('should track dirty state correctly', async () => {
      const user = userEvent.setup();
      renderQuantumSecurity();

      // Initially no unsaved changes
      expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();

      // Make a change
      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });
      await user.click(ocspSwitch);

      // Should show unsaved changes
      expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
    });

    it('should handle apply action correctly', async () => {
      const user = userEvent.setup();
      const mockOnApply = vi.fn().mockResolvedValue(undefined);
      renderQuantumSecurity({ onApply: mockOnApply });

      // Make a change
      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });
      await user.click(ocspSwitch);

      // Apply settings
      const applyButton = screen.getByRole('button', { name: /apply configuration/i });
      await user.click(applyButton);

      await waitFor(() => {
        expect(mockOnApply).toHaveBeenCalled();
      });

      // Dirty state should be cleared
      expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();
    });

    it('should handle security audit action', async () => {
      const user = userEvent.setup();
      const mockOnSecurityAudit = vi.fn().mockResolvedValue(defaultAuditResult);
      renderQuantumSecurity({ onSecurityAudit: mockOnSecurityAudit });

      const auditButton = screen.getByRole('button', { name: /security audit/i });
      await user.click(auditButton);

      expect(mockOnSecurityAudit).toHaveBeenCalled();
    });

    it('should handle generate test certificate action', async () => {
      const user = userEvent.setup();
      const mockOnGenerateTestCert = vi.fn().mockResolvedValue(defaultTestCertResult);
      renderQuantumSecurity({ onGenerateTestCert: mockOnGenerateTestCert });

      const generateButton = screen.getByRole('button', { name: /generate test certificate/i });
      await user.click(generateButton);

      await waitFor(() => {
        expect(mockOnGenerateTestCert).toHaveBeenCalled();
      });
    });
  });

  describe('Loading States', () => {
    it('should handle loading state correctly', () => {
      renderQuantumSecurity({ isLoading: true });

      const applyButton = screen.getByRole('button', { name: /apply configuration/i });
      expect(applyButton).toBeDisabled();

      const auditButton = screen.getByRole('button', { name: /security audit/i });
      expect(auditButton).toBeDisabled();
    });
  });

  describe('Security Level Calculation', () => {
    it('should calculate Maximum security correctly', () => {
      const settings: SecuritySettings = {
        quantumSafe: true,
        falconSigning: true,
        kyberKeyExchange: true,
        tlsVersion: '1.3',
        certificateValidation: 'strict',
        ocspStapling: true,
        hsts: true,
      };

      renderQuantumSecurity({ securitySettings: settings });

      expect(screen.getByText('Maximum Security')).toBeInTheDocument();
    });

    it('should calculate High security correctly', () => {
      const settings: SecuritySettings = {
        quantumSafe: true,
        falconSigning: false,
        kyberKeyExchange: true,
        tlsVersion: '1.3',
        certificateValidation: 'strict',
        ocspStapling: true,
        hsts: true,
      };

      renderQuantumSecurity({ securitySettings: settings });

      expect(screen.getByText('High Security')).toBeInTheDocument();
    });

    it('should calculate Standard security correctly', () => {
      const settings: SecuritySettings = {
        quantumSafe: false,
        falconSigning: false,
        kyberKeyExchange: false,
        tlsVersion: '1.3',
        certificateValidation: 'strict',
        ocspStapling: true,
        hsts: true,
      };

      renderQuantumSecurity({ securitySettings: settings });

      expect(screen.getByText('Standard Security')).toBeInTheDocument();
    });
  });

  describe('Accessibility Features', () => {
    it('should have proper ARIA labels and descriptions', () => {
      renderQuantumSecurity();

      // Check for NIST approval badge
      expect(screen.getByText('NIST Approved')).toBeInTheDocument();

      // Check for algorithm descriptions
      expect(screen.getByText(/enable post-quantum cryptographic algorithms/i)).toBeInTheDocument();
    });

    it('should have proper switch accessibility', () => {
      renderQuantumSecurity();

      const switches = screen.getAllByRole('switch');

      switches.forEach(switchElement => {
        expect(switchElement).toHaveAttribute('aria-checked');
      });
    });
  });
});
