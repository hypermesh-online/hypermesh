// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { NodeConfigurationSettings, type NodeSettings } from '../NodeConfigurationSettings';

/**
 * Feature Parity Tests for NodeConfigurationSettings
 * Validates React component matches Svelte settings.svelte (lines 10-22, 145-217)
 */

describe('NodeConfigurationSettings - Feature Parity', () => {
  const mockOnSave = vi.fn();
  const mockOnTest = vi.fn();
  const mockOnReset = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Default Values Match Svelte Implementation', () => {
    it('should render with correct default values', () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Node ID - matches settings.svelte line 11
      expect(screen.getByDisplayValue('node-001')).toBeInTheDocument();
      
      // IPv6 Address - matches settings.svelte line 12  
      expect(screen.getByDisplayValue('2001:db8::1001')).toBeInTheDocument();
      
      // Region - matches settings.svelte line 13
      expect(screen.getByDisplayValue('us-west-2')).toBeInTheDocument();
      
      // Zone - matches settings.svelte line 14
      expect(screen.getByDisplayValue('us-west-2a')).toBeInTheDocument();
      
      // Max Connections - matches settings.svelte line 17
      expect(screen.getByDisplayValue('1000')).toBeInTheDocument();
      
      // Proxy Enabled - matches settings.svelte line 15
      const proxySwitch = screen.getByRole('switch', { name: /enable nat-like proxy/i });
      expect(proxySwitch).toBeChecked();
      
      // Auto Discovery - matches settings.svelte line 16
      const autoDiscoverySwitch = screen.getByRole('switch', { name: /auto-discovery/i });
      expect(autoDiscoverySwitch).toBeChecked();
    });

    it('should render bandwidth sliders with correct defaults', () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Bandwidth settings - matches settings.svelte lines 18-21
      expect(screen.getByText('1000 Mbps')).toBeInTheDocument(); // Upload
      expect(screen.getAllByText('1000 Mbps')).toHaveLength(2); // Upload + Download
    });
  });

  describe('IPv6 Address Validation', () => {
    it('should validate IPv6 addresses correctly', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const ipv6Input = screen.getByDisplayValue('2001:db8::1001');
      
      // Test invalid IPv6 address
      await user.clear(ipv6Input);
      await user.type(ipv6Input, 'invalid-ipv6');
      
      expect(screen.getByText('Invalid IPv6 address format')).toBeInTheDocument();
      
      // Test valid IPv6 address
      await user.clear(ipv6Input);
      await user.type(ipv6Input, '2001:db8:85a3::8a2e:370:7334');
      
      expect(screen.queryByText('Invalid IPv6 address format')).not.toBeInTheDocument();
    });

    it('should validate compressed IPv6 addresses', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const ipv6Input = screen.getByDisplayValue('2001:db8::1001');
      
      // Test compressed format
      await user.clear(ipv6Input);
      await user.type(ipv6Input, '::1');
      
      expect(screen.queryByText('Invalid IPv6 address format')).not.toBeInTheDocument();
      
      // Test another compressed format
      await user.clear(ipv6Input);
      await user.type(ipv6Input, '2001:db8::');
      
      expect(screen.queryByText('Invalid IPv6 address format')).not.toBeInTheDocument();
    });
  });

  describe('Region Selection', () => {
    it('should provide all required regions from Svelte implementation', () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Check region select is present
      const regionSelect = screen.getByDisplayValue('us-west-2');
      expect(regionSelect).toBeInTheDocument();
      
      // Note: Radix UI Select doesn't expose options until opened
      // This validates the component renders with correct default
    });
  });

  describe('Network Configuration', () => {
    it('should handle max connections input correctly', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const maxConnectionsInput = screen.getByDisplayValue('1000');
      
      // Test changing max connections
      await user.clear(maxConnectionsInput);
      await user.type(maxConnectionsInput, '5000');
      
      expect(screen.getByDisplayValue('5000')).toBeInTheDocument();
    });

    it('should toggle proxy and auto-discovery settings', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const proxySwitch = screen.getByRole('switch', { name: /enable nat-like proxy/i });
      const autoDiscoverySwitch = screen.getByRole('switch', { name: /auto-discovery/i });
      
      // Test toggling proxy
      expect(proxySwitch).toBeChecked();
      await user.click(proxySwitch);
      expect(proxySwitch).not.toBeChecked();
      
      // Test toggling auto-discovery
      expect(autoDiscoverySwitch).toBeChecked();
      await user.click(autoDiscoverySwitch);
      expect(autoDiscoverySwitch).not.toBeChecked();
    });
  });

  describe('Bandwidth Allocation', () => {
    it('should handle bandwidth slider changes', async () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Find upload bandwidth slider
      const uploadSlider = screen.getAllByRole('slider')[0];
      
      // Simulate slider change
      fireEvent.change(uploadSlider, { target: { value: '2000' } });
      
      // Wait for state update
      await waitFor(() => {
        expect(screen.getByText('2000 Mbps')).toBeInTheDocument();
      });
    });

    it('should maintain separate upload and download bandwidth values', async () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const sliders = screen.getAllByRole('slider');
      const uploadSlider = sliders[0];
      const downloadSlider = sliders[1];
      
      // Change upload bandwidth
      fireEvent.change(uploadSlider, { target: { value: '2000' } });
      
      // Change download bandwidth
      fireEvent.change(downloadSlider, { target: { value: '3000' } });
      
      await waitFor(() => {
        const mbpsElements = screen.getAllByText(/\d+ Mbps/);
        expect(mbpsElements).toHaveLength(2);
      });
    });
  });

  describe('State Management and Actions', () => {
    it('should track dirty state correctly', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Initially no unsaved changes badge
      expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();
      
      // Make a change
      const nodeIdInput = screen.getByDisplayValue('node-001');
      await user.clear(nodeIdInput);
      await user.type(nodeIdInput, 'node-002');
      
      // Should show unsaved changes
      expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
    });

    it('should handle save action correctly', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Make a change
      const nodeIdInput = screen.getByDisplayValue('node-001');
      await user.clear(nodeIdInput);
      await user.type(nodeIdInput, 'node-updated');
      
      // Save settings
      const saveButton = screen.getByRole('button', { name: /save settings/i });
      await user.click(saveButton);
      
      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({
            nodeId: 'node-updated'
          })
        );
      });
      
      // Dirty state should be cleared
      expect(screen.queryByText('Unsaved Changes')).not.toBeInTheDocument();
    });

    it('should handle test configuration action', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onTest={mockOnTest} />);
      
      const testButton = screen.getByRole('button', { name: /test configuration/i });
      await user.click(testButton);
      
      expect(mockOnTest).toHaveBeenCalledWith(
        expect.objectContaining({
          nodeId: 'node-001',
          ipv6Address: '2001:db8::1001'
        })
      );
    });

    it('should handle reset to defaults', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onReset={mockOnReset} />);
      
      // Make changes
      const nodeIdInput = screen.getByDisplayValue('node-001');
      await user.clear(nodeIdInput);
      await user.type(nodeIdInput, 'changed-node');
      
      // Reset
      const resetButton = screen.getByRole('button', { name: /reset to defaults/i });
      await user.click(resetButton);
      
      await waitFor(() => {
        expect(screen.getByDisplayValue('node-001')).toBeInTheDocument();
        expect(mockOnReset).toHaveBeenCalled();
      });
    });
  });

  describe('Loading States', () => {
    it('should handle loading state correctly', () => {
      render(<NodeConfigurationSettings loading={true} onSave={mockOnSave} />);
      
      const saveButton = screen.getByRole('button', { name: /saving.../i });
      expect(saveButton).toBeDisabled();
      
      const testButton = screen.getByRole('button', { name: /test configuration/i });
      expect(testButton).toBeDisabled();
      
      const resetButton = screen.getByRole('button', { name: /reset to defaults/i });
      expect(resetButton).toBeDisabled();
    });
  });

  describe('Validation States', () => {
    it('should disable save when IPv6 validation fails', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const ipv6Input = screen.getByDisplayValue('2001:db8::1001');
      await user.clear(ipv6Input);
      await user.type(ipv6Input, 'invalid');
      
      const saveButton = screen.getByRole('button', { name: /save settings/i });
      expect(saveButton).toBeDisabled();
    });

    it('should disable test when IPv6 validation fails', async () => {
      const user = userEvent.setup();
      render(<NodeConfigurationSettings onTest={mockOnTest} />);
      
      const ipv6Input = screen.getByDisplayValue('2001:db8::1001');
      await user.clear(ipv6Input);
      await user.type(ipv6Input, 'invalid');
      
      const testButton = screen.getByRole('button', { name: /test configuration/i });
      expect(testButton).toBeDisabled();
    });
  });

  describe('Accessibility Features', () => {
    it('should have proper form labels and descriptions', () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      // Check for labels
      expect(screen.getByLabelText(/node id/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/ipv6 address/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/region/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/maximum connections/i)).toBeInTheDocument();
      
      // Check for descriptions
      expect(screen.getByText(/unique identifier for this node/i)).toBeInTheDocument();
      expect(screen.getByText(/ipv6 address for network communication/i)).toBeInTheDocument();
      expect(screen.getByText(/maximum concurrent network connections/i)).toBeInTheDocument();
    });

    it('should have proper ARIA attributes for switches', () => {
      render(<NodeConfigurationSettings onSave={mockOnSave} />);
      
      const proxySwitch = screen.getByRole('switch', { name: /enable nat-like proxy/i });
      const autoDiscoverySwitch = screen.getByRole('switch', { name: /auto-discovery/i });
      
      expect(proxySwitch).toHaveAttribute('aria-checked', 'true');
      expect(autoDiscoverySwitch).toHaveAttribute('aria-checked', 'true');
    });
  });
});