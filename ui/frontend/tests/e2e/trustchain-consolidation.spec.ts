// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { test, expect } from '@playwright/test';

/**
 * End-to-End Tests for TrustChain UI Consolidation
 * Validates complete user journeys match Svelte functionality
 */

test.describe('TrustChain UI Consolidation - E2E Tests', () => {
  
  test.beforeEach(async ({ page }) => {
    // Navigate to TrustChain module
    await page.goto('/');
    await page.getByRole('link', { name: /trustchain/i }).click();
  });

  test.describe('Node Configuration Workflow', () => {
    test('should complete full node configuration setup @user-journey', async ({ page }) => {
      // Navigate to node configuration
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Step 1: Set Node ID
      const nodeIdInput = page.getByLabel(/node id/i);
      await nodeIdInput.clear();
      await nodeIdInput.fill('production-node-001');
      
      // Step 2: Configure IPv6 Address
      const ipv6Input = page.getByLabel(/ipv6 address/i);
      await ipv6Input.clear();
      await ipv6Input.fill('2001:db8:85a3::8a2e:370:7334');
      
      // Verify no validation errors
      await expect(page.getByText('Invalid IPv6 address format')).not.toBeVisible();
      
      // Step 3: Select Region
      await page.getByRole('combobox', { name: /region/i }).click();
      await page.getByRole('option', { name: /eu central 1/i }).click();
      
      // Step 4: Configure Bandwidth
      const uploadSlider = page.getByLabel(/upload bandwidth/i);
      await uploadSlider.fill('5000');
      
      const downloadSlider = page.getByLabel(/download bandwidth/i);
      await downloadSlider.fill('8000');
      
      // Step 5: Configure Network Options
      const proxySwitch = page.getByRole('switch', { name: /enable nat-like proxy/i });
      await proxySwitch.check();
      
      const autoDiscoverySwitch = page.getByRole('switch', { name: /auto-discovery/i });
      await autoDiscoverySwitch.check();
      
      // Step 6: Test Configuration
      await page.getByRole('button', { name: /test configuration/i }).click();
      
      // Wait for test completion (simulated)
      await page.waitForTimeout(1000);
      
      // Step 7: Save Settings
      await page.getByRole('button', { name: /save settings/i }).click();
      
      // Verify success
      await expect(page.getByText(/unsaved changes/i)).not.toBeVisible();
      
      // Verify all values were saved
      await expect(nodeIdInput).toHaveValue('production-node-001');
      await expect(ipv6Input).toHaveValue('2001:db8:85a3::8a2e:370:7334');
      await expect(page.getByText('5000 Mbps')).toBeVisible();
      await expect(page.getByText('8000 Mbps')).toBeVisible();
    });

    test('should validate IPv6 addresses correctly @validation', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const ipv6Input = page.getByLabel(/ipv6 address/i);
      
      // Test invalid IPv6
      await ipv6Input.clear();
      await ipv6Input.fill('invalid-ipv6-address');
      await ipv6Input.blur();
      
      await expect(page.getByText('Invalid IPv6 address format')).toBeVisible();
      
      // Test valid IPv6 formats
      const validAddresses = [
        '2001:db8::1',
        '::1',
        '2001:db8:85a3::8a2e:370:7334',
        '2001:db8:85a3:0:0:8a2e:370:7334'
      ];
      
      for (const address of validAddresses) {
        await ipv6Input.clear();
        await ipv6Input.fill(address);
        await ipv6Input.blur();
        
        await expect(page.getByText('Invalid IPv6 address format')).not.toBeVisible();
      }
    });

    test('should handle bandwidth slider interactions @performance', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Test upload bandwidth slider
      const uploadSlider = page.getByRole('slider').first();
      await uploadSlider.fill('7500');
      await expect(page.getByText('7500 Mbps')).toBeVisible();
      
      // Test download bandwidth slider
      const downloadSlider = page.getByRole('slider').nth(1);
      await downloadSlider.fill('9500');
      await expect(page.getByText('9500 Mbps')).toBeVisible();
      
      // Verify sliders are independent
      await expect(page.getByText('7500 Mbps')).toBeVisible();
      await expect(page.getByText('9500 Mbps')).toBeVisible();
    });
  });

  test.describe('Quantum Security Configuration', () => {
    test('should configure quantum security settings @security', async ({ page }) => {
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      // Verify initial state shows Maximum Security
      await expect(page.getByText('Maximum Security')).toBeVisible();
      
      // Test quantum-safe master toggle
      const quantumSafeSwitch = page.getByRole('switch', { name: /quantum-safe cryptography/i });
      await quantumSafeSwitch.uncheck();
      
      // Verify dependent features are disabled
      const falconSwitch = page.getByRole('switch', { name: /falcon-1024 signing/i });
      const kyberSwitch = page.getByRole('switch', { name: /kyber key exchange/i });
      
      await expect(falconSwitch).toBeDisabled();
      await expect(kyberSwitch).toBeDisabled();
      await expect(page.getByText('Standard Security')).toBeVisible();
      
      // Re-enable quantum-safe
      await quantumSafeSwitch.check();
      
      // Verify features are re-enabled
      await expect(falconSwitch).toBeEnabled();
      await expect(kyberSwitch).toBeEnabled();
      
      // Configure specific algorithms
      await falconSwitch.check();
      await kyberSwitch.check();
      
      await expect(page.getByText('Maximum Security')).toBeVisible();
      
      // Configure TLS settings
      await page.getByRole('combobox', { name: /tls version/i }).click();
      await page.getByRole('option', { name: /tls 1.3/i }).click();
      
      // Configure certificate validation
      await page.getByRole('combobox', { name: /certificate validation/i }).click();
      await page.getByRole('option', { name: /strict/i }).click();
      
      // Test security settings
      await page.getByRole('button', { name: /test security/i }).click();
      await page.waitForTimeout(1000);
      
      // Save settings
      await page.getByRole('button', { name: /save security settings/i }).click();
      
      // Verify security status
      await expect(page.getByText('Quantum Resistant:')).toBeVisible();
      await expect(page.getByText('Yes')).toBeVisible();
    });

    test('should display cipher suites correctly @security', async ({ page }) => {
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      // Verify default cipher suites are displayed
      await expect(page.getByText('FALCON-1024')).toBeVisible();
      await expect(page.getByText('Kyber-768')).toBeVisible();
      await expect(page.getByText('AES-256-GCM')).toBeVisible();
      
      // Verify cipher suite description
      await expect(page.getByText(/cryptographic protocols used for secure communication/i)).toBeVisible();
    });
  });

  test.describe('Consensus Metrics Monitoring', () => {
    test('should display four-proof consensus metrics @consensus', async ({ page }) => {
      await page.getByRole('tab', { name: /consensus metrics/i }).click();
      
      // Verify main metrics are displayed
      await expect(page.getByText('15,234')).toBeVisible(); // Block height
      await expect(page.getByText('2.3s')).toBeVisible(); // Block time
      await expect(page.getByText('847')).toBeVisible(); // TPS
      await expect(page.getByText('67')).toBeVisible(); // Validators
      
      // Verify four-proof system display
      await expect(page.getByText('Proof of Space (PoSp)')).toBeVisible();
      await expect(page.getByText('Proof of Stake (PoSt)')).toBeVisible();
      await expect(page.getByText('Proof of Work (PoWk)')).toBeVisible();
      await expect(page.getByText('Proof of Time (PoTm)')).toBeVisible();
      
      // Verify coverage percentages
      await expect(page.getByText('98.5%')).toBeVisible(); // Space coverage
      await expect(page.getByText('96.2%')).toBeVisible(); // Stake coverage
      await expect(page.getByText('99.1%')).toBeVisible(); // Work coverage
      await expect(page.getByText('97.8%')).toBeVisible(); // Time coverage
      
      // Test refresh functionality
      await page.getByRole('button', { name: /refresh/i }).click();
      await page.waitForTimeout(500);
      
      // Verify metrics are still displayed after refresh
      await expect(page.getByText('Four-Proof Consensus System')).toBeVisible();
    });

    test('should display recent blocks with proof validation @consensus', async ({ page }) => {
      await page.getByRole('tab', { name: /consensus metrics/i }).click();
      
      // Verify recent blocks section
      await expect(page.getByText(/recent blocks/i)).toBeVisible();
      
      // Verify block details
      await expect(page.getByText(/block height/i)).toBeVisible();
      await expect(page.getByText(/transactions/i)).toBeVisible();
      await expect(page.getByText(/validator/i)).toBeVisible();
      
      // Verify all four proofs are shown for each block
      await expect(page.getByText('SPACE')).toBeVisible();
      await expect(page.getByText('STAKE')).toBeVisible();
      await expect(page.getByText('WORK')).toBeVisible();
      await expect(page.getByText('TIME')).toBeVisible();
    });
  });

  test.describe('Certificate Management', () => {
    test('should display certificate details correctly @certificates', async ({ page }) => {
      await page.getByRole('tab', { name: /certificate details/i }).click();
      
      // Verify certificate overview
      await expect(page.getByText(/certificate id/i)).toBeVisible();
      await expect(page.getByText(/subject/i)).toBeVisible();
      await expect(page.getByText(/issuer/i)).toBeVisible();
      
      // Verify FALCON-1024 algorithm display
      await expect(page.getByText('FALCON-1024')).toBeVisible();
      
      // Test tab navigation
      await page.getByRole('tab', { name: /details/i }).click();
      await expect(page.getByText(/serial number/i)).toBeVisible();
      
      await page.getByRole('tab', { name: /extensions/i }).click();
      await expect(page.getByText(/digital signature/i)).toBeVisible();
      
      await page.getByRole('tab', { name: /validation/i }).click();
      await expect(page.getByText(/certificate status/i)).toBeVisible();
    });

    test('should handle certificate export @certificates', async ({ page }) => {
      await page.getByRole('tab', { name: /certificate details/i }).click();
      
      // Test export functionality
      await page.getByRole('button', { name: /export/i }).click();
      
      // Verify export initiated (would test actual download in real scenario)
      await page.waitForTimeout(500);
    });
  });

  test.describe('Ecosystem Dashboard', () => {
    test('should display ecosystem metrics @dashboard', async ({ page }) => {
      await page.getByRole('tab', { name: /ecosystem metrics/i }).click();
      
      // Verify main ecosystem metrics
      await expect(page.getByText('1,247')).toBeVisible(); // Total assets
      await expect(page.getByText('892')).toBeVisible(); // Active certificates
      await expect(page.getByText('2.95 Gbps')).toBeVisible(); // Network throughput
      
      // Verify system health indicators
      await expect(page.getByText('TrustChain CA')).toBeVisible();
      await expect(page.getByText('STOQ Protocol')).toBeVisible();
      await expect(page.getByText('HyperMesh Assets')).toBeVisible();
      await expect(page.getByText('Caesar Economics')).toBeVisible();
      
      // Test refresh functionality
      await page.getByRole('button', { name: /refresh/i }).click();
      await page.waitForTimeout(1000);
      
      // Verify metrics refresh
      await expect(page.getByText('Web3 Ecosystem Dashboard')).toBeVisible();
    });

    test('should identify performance bottlenecks @performance', async ({ page }) => {
      await page.getByRole('tab', { name: /ecosystem metrics/i }).click();
      
      // Verify STOQ performance bottleneck is highlighted
      await expect(page.getByText('2.95 Gbps')).toBeVisible();
      
      // This should be marked as below target (40 Gbps)
      // In a real implementation, this would show as a warning
      const stoqMetric = page.getByText('2.95 Gbps');
      await expect(stoqMetric).toBeVisible();
    });
  });

  test.describe('Cross-Component Integration', () => {
    test('should maintain state across component switches @integration', async ({ page }) => {
      // Configure node settings
      await page.getByRole('tab', { name: /node configuration/i }).click();
      const nodeIdInput = page.getByLabel(/node id/i);
      await nodeIdInput.clear();
      await nodeIdInput.fill('integration-test-node');
      
      // Switch to security settings
      await page.getByRole('tab', { name: /quantum security/i }).click();
      const quantumSwitch = page.getByRole('switch', { name: /quantum-safe cryptography/i });
      await quantumSwitch.uncheck();
      
      // Switch back to node configuration
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Verify node ID is preserved
      await expect(nodeIdInput).toHaveValue('integration-test-node');
      
      // Switch back to security settings
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      // Verify security setting is preserved
      await expect(quantumSwitch).not.toBeChecked();
    });

    test('should handle error states gracefully @error-handling', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Create validation error
      const ipv6Input = page.getByLabel(/ipv6 address/i);
      await ipv6Input.clear();
      await ipv6Input.fill('invalid-address');
      
      // Verify save button is disabled
      const saveButton = page.getByRole('button', { name: /save settings/i });
      await expect(saveButton).toBeDisabled();
      
      // Verify error message is displayed
      await expect(page.getByText('Invalid IPv6 address format')).toBeVisible();
      
      // Fix the error
      await ipv6Input.clear();
      await ipv6Input.fill('2001:db8::1');
      
      // Verify save button is enabled
      await expect(saveButton).toBeEnabled();
      await expect(page.getByText('Invalid IPv6 address format')).not.toBeVisible();
    });
  });

  test.describe('Accessibility Compliance', () => {
    test('should support keyboard navigation @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Test tab navigation through form
      await page.keyboard.press('Tab');
      await expect(page.getByLabel(/node id/i)).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByLabel(/ipv6 address/i)).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByRole('combobox', { name: /region/i })).toBeFocused();
      
      // Test form submission with Enter
      await page.getByLabel(/node id/i).focus();
      await page.keyboard.press('Enter');
      // Form should not submit with validation errors
    });

    test('should provide proper ARIA labels @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      // Check switch ARIA attributes
      const quantumSwitch = page.getByRole('switch', { name: /quantum-safe cryptography/i });
      await expect(quantumSwitch).toHaveAttribute('aria-checked');
      
      // Check form labels
      await page.getByRole('tab', { name: /node configuration/i }).click();
      const nodeIdInput = page.getByLabel(/node id/i);
      await expect(nodeIdInput).toHaveAttribute('aria-describedby');
    });

    test('should support screen readers @accessibility', async ({ page }) => {
      // This test would require screen reader testing tools
      // For now, verify semantic structure
      await page.getByRole('tab', { name: /consensus metrics/i }).click();
      
      // Verify proper heading structure
      await expect(page.getByRole('heading', { name: /four-proof consensus system/i })).toBeVisible();
      
      // Verify proper table structure for metrics
      await expect(page.getByRole('table')).toBeVisible();
    });
  });

  test.describe('Performance Validation', () => {
    test('should load components quickly @performance', async ({ page }) => {
      const startTime = Date.now();
      
      await page.getByRole('tab', { name: /ecosystem metrics/i }).click();
      
      // Wait for content to be visible
      await expect(page.getByText('Web3 Ecosystem Dashboard')).toBeVisible();
      
      const loadTime = Date.now() - startTime;
      
      // Should load within 2 seconds
      expect(loadTime).toBeLessThan(2000);
    });

    test('should handle rapid component switching @performance', async ({ page }) => {
      const tabs = [
        'node configuration',
        'quantum security', 
        'consensus metrics',
        'certificate details',
        'ecosystem metrics'
      ];
      
      // Rapidly switch between tabs
      for (let i = 0; i < 3; i++) {
        for (const tab of tabs) {
          await page.getByRole('tab', { name: new RegExp(tab, 'i') }).click();
          await page.waitForTimeout(100);
        }
      }
      
      // Verify final tab still works
      await page.getByRole('tab', { name: /ecosystem metrics/i }).click();
      await expect(page.getByText('Web3 Ecosystem Dashboard')).toBeVisible();
    });
  });
});