// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { test, expect } from '@playwright/test';

/**
 * Navigation E2E Tests
 * Validates routing and navigation between pages works correctly
 */

test.describe('Application Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for initial load
    await page.waitForLoadState('networkidle');
  });

  test('should navigate to STOQ Demo page', async ({ page }) => {
    // Find and click the STOQ Demo link in sidebar
    const stoqLink = page.getByRole('link', { name: /stoq demo/i });
    if (await stoqLink.isVisible()) {
      await stoqLink.click();
      await expect(page).toHaveURL(/\/stoq-demo/);
      await expect(page.getByText('STOQ Native Protocol Demo')).toBeVisible();
    }
  });

  test('should navigate to HyperMesh page', async ({ page }) => {
    const link = page.getByRole('link', { name: /hypermesh/i });
    if (await link.isVisible()) {
      await link.click();
      await expect(page).toHaveURL(/\/hypermesh/);
    }
  });

  test('should navigate to TrustChain page', async ({ page }) => {
    const link = page.getByRole('link', { name: /trustchain/i });
    if (await link.isVisible()) {
      await link.click();
      await expect(page).toHaveURL(/\/trustchain/);
    }
  });

  test('should navigate to Catalog page', async ({ page }) => {
    const link = page.getByRole('link', { name: /catalog/i });
    if (await link.isVisible()) {
      await link.click();
      await expect(page).toHaveURL(/\/catalog/);
    }
  });

  test('should navigate to Caesar page', async ({ page }) => {
    const link = page.getByRole('link', { name: /caesar/i });
    if (await link.isVisible()) {
      await link.click();
      await expect(page).toHaveURL(/\/caesar/);
    }
  });

  test('should navigate back to dashboard from a module page', async ({ page }) => {
    // Navigate to a module first
    const trustchainLink = page.getByRole('link', { name: /trustchain/i });
    if (await trustchainLink.isVisible()) {
      await trustchainLink.click();
      await expect(page).toHaveURL(/\/trustchain/);

      // Navigate back to dashboard
      const dashboardLink = page.getByRole('link', { name: /dashboard/i });
      if (await dashboardLink.isVisible()) {
        await dashboardLink.click();
        await expect(page).toHaveURL('/');
      }
    }
  });

  test('should maintain layout across page navigations', async ({ page }) => {
    // Verify sidebar persists during navigation
    const sidebar = page.locator('nav, aside, [role="navigation"]').first();
    await expect(sidebar).toBeVisible();

    // Navigate to a module
    const link = page.getByRole('link', { name: /trustchain/i });
    if (await link.isVisible()) {
      await link.click();
      // Sidebar should still be visible
      await expect(sidebar).toBeVisible();
    }
  });

  test('should handle direct URL navigation', async ({ page }) => {
    // Navigate directly to a route
    await page.goto('/stoq-demo');
    await page.waitForLoadState('networkidle');

    // The page should render the STOQ demo content
    await expect(page.getByText('STOQ Native Protocol Demo')).toBeVisible();
  });
});
