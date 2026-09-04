// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { test, expect } from '@playwright/test';

/**
 * Dashboard Home Page E2E Tests
 * Validates the main dashboard loads correctly and displays ecosystem overview
 */

test.describe('Dashboard Home Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load the dashboard home page', async ({ page }) => {
    // Verify the page title
    await expect(page).toHaveTitle(/HyperMesh Ecosystem Dashboard/);

    // Verify the root element renders
    const root = page.locator('#root');
    await expect(root).toBeVisible();
  });

  test('should display the sidebar navigation', async ({ page }) => {
    // The sidebar should contain navigation links to core modules
    const sidebar = page.locator('nav, aside, [role="navigation"]').first();
    await expect(sidebar).toBeVisible();

    // Check for key navigation items
    await expect(page.getByText('Dashboard')).toBeVisible();
  });

  test('should display API connection status indicator', async ({ page }) => {
    // The app shows an API status indicator during initialization
    // Either "Initializing HyperMesh API..." or "API Connected" or "API Warning"
    const statusIndicator = page.locator('[class*="fixed"][class*="top-4"]');
    await expect(statusIndicator).toBeVisible({ timeout: 10000 });
  });

  test('should render the main content area', async ({ page }) => {
    // The layout has a main content area
    const mainContent = page.locator('main');
    await expect(mainContent).toBeVisible();
  });

  test('should have a dark theme by default', async ({ page }) => {
    // The app uses a dark theme with black background
    const appContainer = page.locator('.min-h-screen');
    await expect(appContainer).toHaveClass(/bg-black/);
  });
});
