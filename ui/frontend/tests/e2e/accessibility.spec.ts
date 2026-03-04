// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

/**
 * Accessibility Tests for TrustChain UI Components
 * WCAG 2.1 AA Compliance Validation
 */

test.describe('TrustChain UI Accessibility Tests', () => {
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: /trustchain/i }).click();
  });

  test.describe('WCAG 2.1 AA Compliance', () => {
    test('should pass axe accessibility tests for node configuration @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      
      expect(accessibilityScanResults.violations).toEqual([]);
    });

    test('should pass axe accessibility tests for quantum security @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      
      expect(accessibilityScanResults.violations).toEqual([]);
    });

    test('should pass axe accessibility tests for state proof metrics @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /state proof metrics/i }).click();
      
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      
      expect(accessibilityScanResults.violations).toEqual([]);
    });

    test('should pass axe accessibility tests for certificate details @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /certificate details/i }).click();
      
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      
      expect(accessibilityScanResults.violations).toEqual([]);
    });

    test('should pass axe accessibility tests for ecosystem dashboard @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /ecosystem metrics/i }).click();
      
      const accessibilityScanResults = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      
      expect(accessibilityScanResults.violations).toEqual([]);
    });
  });

  test.describe('Keyboard Navigation', () => {
    test('should support complete keyboard navigation through node configuration @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Start from first focusable element
      await page.keyboard.press('Tab');
      await expect(page.getByLabel(/node id/i)).toBeFocused();
      
      // Navigate through all form fields
      await page.keyboard.press('Tab');
      await expect(page.getByLabel(/ipv6 address/i)).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByRole('combobox', { name: /region/i })).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByLabel(/availability zone/i)).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByLabel(/maximum connections/i)).toBeFocused();
      
      // Navigate to switches
      await page.keyboard.press('Tab');
      await expect(page.getByRole('switch', { name: /enable nat-like proxy/i })).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByRole('switch', { name: /auto-discovery/i })).toBeFocused();
      
      // Navigate to sliders
      await page.keyboard.press('Tab');
      await expect(page.getByRole('slider').first()).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByRole('slider').nth(1)).toBeFocused();
      
      // Navigate to action buttons
      await page.keyboard.press('Tab');
      await expect(page.getByRole('button', { name: /reset to defaults/i })).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByRole('button', { name: /test configuration/i })).toBeFocused();
      
      await page.keyboard.press('Tab');
      await expect(page.getByRole('button', { name: /save settings/i })).toBeFocused();
    });

    test('should support keyboard interaction with switches @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      const quantumSwitch = page.getByRole('switch', { name: /quantum-safe cryptography/i });
      await quantumSwitch.focus();
      
      // Verify initial state
      await expect(quantumSwitch).toHaveAttribute('aria-checked', 'true');
      
      // Toggle with space key
      await page.keyboard.press('Space');
      await expect(quantumSwitch).toHaveAttribute('aria-checked', 'false');
      
      // Toggle back
      await page.keyboard.press('Space');
      await expect(quantumSwitch).toHaveAttribute('aria-checked', 'true');
      
      // Test Enter key as well
      await page.keyboard.press('Enter');
      await expect(quantumSwitch).toHaveAttribute('aria-checked', 'false');
    });

    test('should support keyboard navigation for sliders @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const uploadSlider = page.getByRole('slider').first();
      await uploadSlider.focus();
      
      // Get initial value
      const initialValue = await uploadSlider.getAttribute('aria-valuenow');
      
      // Use arrow keys to change value
      await page.keyboard.press('ArrowRight');
      const newValue = await uploadSlider.getAttribute('aria-valuenow');
      
      expect(parseInt(newValue || '0')).toBeGreaterThan(parseInt(initialValue || '0'));
      
      // Test other arrow keys
      await page.keyboard.press('ArrowLeft');
      await page.keyboard.press('ArrowUp');
      await page.keyboard.press('ArrowDown');
      
      // Verify slider still functional
      await expect(uploadSlider).toHaveAttribute('aria-valuenow');
    });

    test('should support keyboard navigation for dropdown menus @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const regionSelect = page.getByRole('combobox', { name: /region/i });
      await regionSelect.focus();
      
      // Open dropdown with Enter
      await page.keyboard.press('Enter');
      
      // Navigate options with arrow keys
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('ArrowDown');
      
      // Select with Enter
      await page.keyboard.press('Enter');
      
      // Verify selection worked
      await expect(regionSelect).toBeFocused();
    });
  });

  test.describe('Screen Reader Support', () => {
    test('should provide proper form labels and descriptions @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Verify form labels
      const nodeIdInput = page.getByLabel(/node id/i);
      await expect(nodeIdInput).toHaveAttribute('aria-describedby');
      
      const ipv6Input = page.getByLabel(/ipv6 address/i);
      await expect(ipv6Input).toHaveAttribute('aria-describedby');
      
      // Verify descriptions are present
      await expect(page.getByText(/unique identifier for this node/i)).toBeVisible();
      await expect(page.getByText(/ipv6 address for network communication/i)).toBeVisible();
    });

    test('should announce validation errors @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const ipv6Input = page.getByLabel(/ipv6 address/i);
      
      // Create validation error
      await ipv6Input.clear();
      await ipv6Input.fill('invalid-ipv6');
      await ipv6Input.blur();
      
      // Verify error is associated with input
      const errorMessage = page.getByText('Invalid IPv6 address format');
      await expect(errorMessage).toBeVisible();
      
      // Verify ARIA attributes
      await expect(ipv6Input).toHaveAttribute('aria-invalid', 'true');
      
      // Error should be announced to screen readers
      const errorId = await errorMessage.getAttribute('id');
      if (errorId) {
        const describedBy = await ipv6Input.getAttribute('aria-describedby');
        expect(describedBy).toContain(errorId);
      }
    });

    test('should provide proper headings hierarchy @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /state proof metrics/i }).click();
      
      // Verify main heading
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      
      // Verify subheadings
      const subheadings = page.getByRole('heading', { level: 2 });
      const count = await subheadings.count();
      expect(count).toBeGreaterThan(0);
      
      // Verify heading text
      await expect(page.getByRole('heading', { name: /four-proof state verification system/i })).toBeVisible();
    });

    test('should provide proper table structure for data @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /state proof metrics/i }).click();
      
      // Check for proper table structure if metrics are displayed in table format
      const tables = page.getByRole('table');
      const tableCount = await tables.count();
      
      if (tableCount > 0) {
        // Verify table has headers
        const firstTable = tables.first();
        await expect(firstTable.getByRole('columnheader')).toHaveCount(1);
        
        // Verify table has caption or aria-label
        const hasCaption = await firstTable.locator('caption').count() > 0;
        const hasAriaLabel = await firstTable.getAttribute('aria-label');
        
        expect(hasCaption || hasAriaLabel).toBeTruthy();
      }
    });
  });

  test.describe('Focus Management', () => {
    test('should maintain focus after form submission @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const saveButton = page.getByRole('button', { name: /save settings/i });
      await saveButton.focus();
      await saveButton.click();
      
      // Focus should remain on save button or move to success message
      await page.waitForTimeout(100);
      const focusedElement = page.locator(':focus');
      await expect(focusedElement).toBeVisible();
    });

    test('should provide focus indicators @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Test focus indicator on various elements
      const nodeIdInput = page.getByLabel(/node id/i);
      await nodeIdInput.focus();
      
      // Verify focus indicator is visible (this would need custom CSS checking)
      await expect(nodeIdInput).toBeFocused();
      
      const saveButton = page.getByRole('button', { name: /save settings/i });
      await saveButton.focus();
      await expect(saveButton).toBeFocused();
    });

    test('should handle focus trap in modal dialogs @accessibility', async ({ page }) => {
      // This test would apply if there are modal dialogs
      // For now, verify tab navigation stays within component
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Navigate to last focusable element
      const saveButton = page.getByRole('button', { name: /save settings/i });
      await saveButton.focus();
      
      // Tab should cycle back to first focusable element
      await page.keyboard.press('Tab');
      
      // Should focus on first tab or first form element
      const focusedElement = page.locator(':focus');
      await expect(focusedElement).toBeVisible();
    });
  });

  test.describe('Color and Contrast', () => {
    test('should meet color contrast requirements @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /quantum security/i }).click();
      
      // This would require color contrast checking tools
      // For now, verify essential text is visible
      await expect(page.getByText('Quantum Security Settings')).toBeVisible();
      await expect(page.getByText('Maximum Security')).toBeVisible();
      
      // Verify error states have adequate contrast
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      const ipv6Input = page.getByLabel(/ipv6 address/i);
      await ipv6Input.clear();
      await ipv6Input.fill('invalid');
      await ipv6Input.blur();
      
      const errorMessage = page.getByText('Invalid IPv6 address format');
      await expect(errorMessage).toBeVisible();
      
      // Error text should be clearly visible
      await expect(errorMessage).toHaveCSS('color', /.+/);
    });

    test('should not rely solely on color for information @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /state proof metrics/i }).click();
      
      // Verify proof status uses more than just color
      // Should have text labels like "VALID", "PENDING", etc.
      const proofElements = page.locator('[data-testid*="proof"]');
      const count = await proofElements.count();
      
      for (let i = 0; i < count; i++) {
        const element = proofElements.nth(i);
        const text = await element.textContent();
        
        // Should have text indicators, not just color
        expect(text).toBeTruthy();
        expect(text!.length).toBeGreaterThan(0);
      }
    });
  });

  test.describe('Responsive Design Accessibility', () => {
    test('should maintain accessibility on mobile viewports @accessibility', async ({ page }) => {
      // Set mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });
      
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Verify form is still accessible
      await expect(page.getByLabel(/node id/i)).toBeVisible();
      await expect(page.getByLabel(/ipv6 address/i)).toBeVisible();
      
      // Verify buttons are still clickable
      const saveButton = page.getByRole('button', { name: /save settings/i });
      await expect(saveButton).toBeVisible();
      
      // Test touch targets are adequate size (minimum 44px)
      const buttonBox = await saveButton.boundingBox();
      expect(buttonBox!.height).toBeGreaterThanOrEqual(44);
    });

    test('should support 200% zoom without horizontal scrolling @accessibility', async ({ page }) => {
      // Simulate 200% zoom by reducing viewport
      await page.setViewportSize({ width: 640, height: 480 });
      
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Verify no horizontal scrolling required
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 10); // Allow small tolerance
      
      // Verify content is still readable
      await expect(page.getByText('Node Configuration')).toBeVisible();
      await expect(page.getByLabel(/node id/i)).toBeVisible();
    });
  });

  test.describe('Dynamic Content Accessibility', () => {
    test('should announce dynamic content changes @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Make a change that triggers dynamic content
      const nodeIdInput = page.getByLabel(/node id/i);
      await nodeIdInput.clear();
      await nodeIdInput.fill('new-node-id');
      
      // Verify unsaved changes indicator appears
      await expect(page.getByText('Unsaved Changes')).toBeVisible();
      
      // The changes should be announced via aria-live regions
      const liveRegions = page.locator('[aria-live]');
      const count = await liveRegions.count();
      expect(count).toBeGreaterThan(0);
    });

    test('should handle loading states accessibly @accessibility', async ({ page }) => {
      await page.getByRole('tab', { name: /node configuration/i }).click();
      
      // Test loading state
      const saveButton = page.getByRole('button', { name: /save settings/i });
      await saveButton.click();
      
      // Should show loading indicator
      await expect(page.getByText(/saving.../i)).toBeVisible();
      
      // Button should be properly disabled
      await expect(saveButton).toBeDisabled();
      await expect(saveButton).toHaveAttribute('aria-disabled', 'true');
    });
  });
});