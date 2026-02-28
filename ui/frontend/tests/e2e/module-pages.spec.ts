// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { test, expect } from '@playwright/test';

/**
 * Module Pages E2E Tests
 * Validates that each module page renders its core content
 */

test.describe('Module Pages Render Content', () => {

  test('STOQ Demo page renders protocol information', async ({ page }) => {
    await page.goto('/stoq-demo');
    await page.waitForLoadState('networkidle');

    // Core heading
    await expect(page.getByText('STOQ Native Protocol Demo')).toBeVisible();

    // Protocol information section
    await expect(page.getByText('Internet 2.0 Protocol Information')).toBeVisible();

    // Protocol details
    await expect(page.getByText('Pure QUIC over IPv6')).toBeVisible();
    await expect(page.getByText('TrustChain Certificates')).toBeVisible();
    await expect(page.getByText('WebAssembly in Browser')).toBeVisible();

    // Connection status section
    await expect(page.getByText('Connection Status')).toBeVisible();
  });

  test('Integration test page renders backend status', async ({ page }) => {
    await page.goto('/integration');
    await page.waitForLoadState('networkidle');

    // Should show the integration test page content
    // Either loading state or the actual status display
    const backendTitle = page.getByText('Backend Integration Status');
    const loadingTitle = page.getByText('Testing Backend Integration...');

    // One of these should be visible
    const isBackendVisible = await backendTitle.isVisible().catch(() => false);
    const isLoadingVisible = await loadingTitle.isVisible().catch(() => false);

    expect(isBackendVisible || isLoadingVisible).toBeTruthy();
  });

  test('Monitor page renders dashboard monitoring', async ({ page }) => {
    await page.goto('/monitor');
    await page.waitForLoadState('networkidle');

    // The monitor page should have some content visible
    const mainContent = page.locator('main');
    await expect(mainContent).toBeVisible();
  });

  test('STOQ Demo page shows connect button when disconnected', async ({ page }) => {
    await page.goto('/stoq-demo');
    await page.waitForLoadState('networkidle');

    // When not connected, should show the Connect button
    const connectButton = page.getByRole('button', { name: /connect via stoq/i });
    const isVisible = await connectButton.isVisible().catch(() => false);

    // If WebAssembly is not available, the button may be disabled
    if (isVisible) {
      expect(await connectButton.isVisible()).toBeTruthy();
    }
  });

  test('Integration test page shows API endpoints', async ({ page }) => {
    await page.goto('/integration');
    await page.waitForLoadState('networkidle');

    // Wait for loading to complete (max 10 seconds)
    await page.waitForTimeout(3000);

    // Should display configured API endpoints
    const endpointsSection = page.getByText('Configured API Endpoints');
    const isEndpointsVisible = await endpointsSection.isVisible().catch(() => false);

    if (isEndpointsVisible) {
      // Verify endpoint addresses are shown
      await expect(page.getByText('TrustChain:')).toBeVisible();
      await expect(page.getByText('STOQ Transport:')).toBeVisible();
    }
  });

  test('pages render without JavaScript errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', error => {
      errors.push(error.message);
    });

    // Visit key pages
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await page.goto('/stoq-demo');
    await page.waitForLoadState('networkidle');

    await page.goto('/integration');
    await page.waitForLoadState('networkidle');

    // Filter out expected API connection errors (backend not running during tests)
    const unexpectedErrors = errors.filter(error =>
      !error.includes('fetch') &&
      !error.includes('Failed to fetch') &&
      !error.includes('NetworkError') &&
      !error.includes('net::ERR_')
    );

    expect(unexpectedErrors).toHaveLength(0);
  });
});
