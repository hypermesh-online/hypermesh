// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QuantumSecuritySettings, type SecuritySettings } from '../QuantumSecuritySettings';

/**
 * Feature Parity Tests for QuantumSecuritySettings
 * Validates React component matches Svelte settings.svelte (lines 24-32, 219-287)
 */

describe('QuantumSecuritySettings - Feature Parity', () => {
  const mockOnSave = vi.fn();
  const mockOnTest = vi.fn();
  const mockOnReset = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Default Values Match Svelte Implementation', () => {
    it('should render with correct default security settings', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // TLS version - matches settings.svelte line 28 (default '1.3')
      expect(screen.getByDisplayValue('1.3')).toBeInTheDocument();
    });

    it('should display correct certificate validation default', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Certificate validation - matches settings.svelte line 29 (default 'strict')
      expect(screen.getByDisplayValue('strict')).toBeInTheDocument();
    });

    it('should display security level indicator', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Should show maximum security with all quantum features enabled
      expect(screen.getByText('Maximum Security')).toBeInTheDocument();
    });
  });

  describe('Quantum-Safe Cryptography Master Toggle', () => {
    it('should disable dependent features when quantum-safe is disabled', async () => {
      const user = userEvent.setup();
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      expect(screen.getByText('Post-quantum digital signatures')).toBeInTheDocument();
      expect(screen.getByText('Quantum-resistant key encapsulation')).toBeInTheDocument();
    });
  });

  describe('Protocol Configuration', () => {
    it('should handle TLS version selection', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // TLS version select should be present with correct default
      expect(screen.getByDisplayValue('1.3')).toBeInTheDocument();
      
      // Should show recommended badge for TLS 1.3
      expect(screen.getByText('Recommended')).toBeInTheDocument();
    });

    it('should handle certificate validation level selection', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Certificate validation should be present with correct default
      expect(screen.getByDisplayValue('strict')).toBeInTheDocument();
    });
  });

  describe('Additional Security Features', () => {
    it('should handle OCSP stapling toggle', async () => {
      const user = userEvent.setup();
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      expect(screen.getByText(/online certificate status protocol/i)).toBeInTheDocument();
      expect(screen.getByText(/force https connections/i)).toBeInTheDocument();
    });
  });

  describe('Cipher Suites Display', () => {
    it('should display active cipher suites', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Default cipher suites should be displayed
      expect(screen.getByText('FALCON-1024')).toBeInTheDocument();
      expect(screen.getByText('Kyber-768')).toBeInTheDocument();
      expect(screen.getByText('AES-256-GCM')).toBeInTheDocument();
    });

    it('should show cipher suites description', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      expect(screen.getByText(/cryptographic protocols used for secure communication/i)).toBeInTheDocument();
    });
  });

  describe('Security Status Summary', () => {
    it('should display comprehensive security status', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
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
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Initially no unsaved changes
      expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();
      
      // Make a change
      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });
      await user.click(ocspSwitch);
      
      // Should show unsaved changes
      expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
    });

    it('should handle save action correctly', async () => {
      const user = userEvent.setup();
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Make a change
      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });
      await user.click(ocspSwitch);
      
      // Save settings
      const saveButton = screen.getByRole('button', { name: /save security settings/i });
      await user.click(saveButton);
      
      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({
            ocspStapling: false
          })
        );
      });
      
      // Dirty state should be cleared
      expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();
    });

    it('should handle test security action', async () => {
      const user = userEvent.setup();
      render(<QuantumSecuritySettings onTest={mockOnTest} />);
      
      const testButton = screen.getByRole('button', { name: /test security/i });
      await user.click(testButton);
      
      expect(mockOnTest).toHaveBeenCalledWith(
        expect.objectContaining({
          quantumSafe: true,
          falconSigning: true,
          kyberKeyExchange: true
        })
      );
    });

    it('should handle reset to defaults', async () => {
      const user = userEvent.setup();
      render(<QuantumSecuritySettings onReset={mockOnReset} />);
      
      // Make changes
      const ocspSwitch = screen.getByRole('switch', { name: /ocsp stapling/i });
      await user.click(ocspSwitch);
      
      // Reset
      const resetButton = screen.getByRole('button', { name: /reset to defaults/i });
      await user.click(resetButton);
      
      await waitFor(() => {
        expect(ocspSwitch).toBeChecked();
        expect(mockOnReset).toHaveBeenCalled();
      });
    });
  });

  describe('Loading States', () => {
    it('should handle loading state correctly', () => {
      render(<QuantumSecuritySettings loading={true} onSave={mockOnSave} />);
      
      const saveButton = screen.getByRole('button', { name: /saving.../i });
      expect(saveButton).toBeDisabled();
      
      const testButton = screen.getByRole('button', { name: /test security/i });
      expect(testButton).toBeDisabled();
      
      const resetButton = screen.getByRole('button', { name: /reset to defaults/i });
      expect(resetButton).toBeDisabled();
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
        cipherSuites: ['FALCON-1024', 'Kyber-768', 'AES-256-GCM']
      };
      
      render(<QuantumSecuritySettings settings={settings} onSave={mockOnSave} />);
      
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
        cipherSuites: ['Kyber-768', 'AES-256-GCM']
      };
      
      render(<QuantumSecuritySettings settings={settings} onSave={mockOnSave} />);
      
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
        cipherSuites: ['AES-256-GCM']
      };
      
      render(<QuantumSecuritySettings settings={settings} onSave={mockOnSave} />);
      
      expect(screen.getByText('Standard Security')).toBeInTheDocument();
    });
  });

  describe('Accessibility Features', () => {
    it('should have proper ARIA labels and descriptions', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      // Check for NIST approval badge
      expect(screen.getByText('NIST Approved')).toBeInTheDocument();
      
      // Check for algorithm descriptions
      expect(screen.getByText(/enable post-quantum cryptographic algorithms/i)).toBeInTheDocument();
    });

    it('should have proper switch accessibility', () => {
      render(<QuantumSecuritySettings onSave={mockOnSave} />);
      
      const switches = screen.getAllByRole('switch');
      
      switches.forEach(switchElement => {
        expect(switchElement).toHaveAttribute('aria-checked');
      });
    });
  });
});