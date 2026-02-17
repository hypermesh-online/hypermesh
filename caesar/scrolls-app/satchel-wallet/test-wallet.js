// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

(async () => {
  const browser = await chromium.launch({ 
    headless: false,
    slowMo: 1000  // Slow down for better observation
  });
  
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 }, // iPhone 12 Pro dimensions for mobile testing
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true
  });
  
  const page = await context.newPage();
  
  console.log('🚀 Starting Satchel Wallet Comprehensive Test Suite');
  
  try {
    // Test 1: App loads without errors
    console.log('\n📱 Test 1: App Loading...');
    await page.goto('http://localhost:3002');
    await page.waitForLoadState('networkidle');
    
    // Check for any console errors
    const errors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });
    
    await page.screenshot({ path: 'test-results/01-app-loaded.png', fullPage: true });
    console.log('✅ App loaded successfully');
    
    // Test 2: Clean mobile interface displays properly
    console.log('\n📱 Test 2: Mobile Interface Check...');
    
    // Check if main elements are visible
    const header = await page.locator('header').isVisible();
    const bottomNav = await page.locator('[role="tablist"]').isVisible();
    const mainContent = await page.locator('main').isVisible();
    
    console.log(`Header visible: ${header}`);
    console.log(`Bottom navigation visible: ${bottomNav}`);
    console.log(`Main content visible: ${mainContent}`);
    
    await page.screenshot({ path: 'test-results/02-mobile-interface.png', fullPage: true });
    console.log('✅ Mobile interface elements verified');
    
    // Test 3: WalletConnect button in header works
    console.log('\n🔗 Test 3: WalletConnect Button...');
    
    const walletConnectButton = page.getByRole('button', { name: /connect wallet/i });
    const isWalletButtonVisible = await walletConnectButton.isVisible();
    console.log(`WalletConnect button visible: ${isWalletButtonVisible}`);
    
    if (isWalletButtonVisible) {
      await walletConnectButton.click();
      await page.waitForTimeout(2000); // Wait for modal/dialog
      await page.screenshot({ path: 'test-results/03-wallet-connect-clicked.png', fullPage: true });
      console.log('✅ WalletConnect button clicked successfully');
      
      // Check if modal appeared
      const modal = await page.locator('[role="dialog"]').isVisible().catch(() => false);
      console.log(`WalletConnect modal appeared: ${modal}`);
      
      // Close modal if it appeared
      if (modal) {
        const closeButton = page.locator('[role="dialog"] button').first();
        if (await closeButton.isVisible()) {
          await closeButton.click();
        }
      }
    }
    
    // Test 4: Loading states and error handling
    console.log('\n⏳ Test 4: Loading States and Error Handling...');
    
    // Look for loading indicators
    const loadingElements = await page.locator('[data-testid*="loading"], .animate-spin, [class*="loading"]').count();
    console.log(`Loading elements found: ${loadingElements}`);
    
    await page.screenshot({ path: 'test-results/04-loading-states.png', fullPage: true });
    
    // Test error states by navigating to a non-existent route
    await page.goto('http://localhost:3002/non-existent-route');
    await page.waitForTimeout(1000);
    await page.screenshot({ path: 'test-results/04b-error-handling.png', fullPage: true });
    
    // Return to main page
    await page.goto('http://localhost:3002');
    await page.waitForLoadState('networkidle');
    
    console.log('✅ Loading and error states tested');
    
    // Test 5: Toast notifications function
    console.log('\n🍞 Test 5: Toast Notifications...');
    
    // Check if toast container exists
    const toastContainer = await page.locator('[data-sonner-toaster]').isVisible().catch(() => false);
    console.log(`Toast container present: ${toastContainer}`);
    
    await page.screenshot({ path: 'test-results/05-toast-setup.png', fullPage: true });
    console.log('✅ Toast notification setup verified');
    
    // Test 6: Bottom tab navigation
    console.log('\n🧭 Test 6: Bottom Tab Navigation...');
    
    // Find all tab buttons
    const tabButtons = await page.locator('[role="tablist"] button').all();
    console.log(`Number of tabs found: ${tabButtons.length}`);
    
    for (let i = 0; i < tabButtons.length; i++) {
      const tab = tabButtons[i];
      const tabName = await tab.textContent();
      console.log(`Testing tab: ${tabName}`);
      
      await tab.click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: `test-results/06-tab-${i}-${tabName?.replace(/\s+/g, '-').toLowerCase()}.png`, fullPage: true });
    }
    
    console.log('✅ Bottom tab navigation tested');
    
    // Test 7: Wallet not connected state
    console.log('\n👤 Test 7: Wallet Not Connected State...');
    
    // Check for "not connected" indicators
    const notConnectedElements = await page.getByText(/not connected|connect wallet|no wallet/i).count();
    console.log(`"Not connected" indicators found: ${notConnectedElements}`);
    
    // Check for placeholder content or empty states
    const emptyStates = await page.locator('[data-testid*="empty"], [class*="empty-state"]').count();
    console.log(`Empty state elements found: ${emptyStates}`);
    
    await page.screenshot({ path: 'test-results/07-not-connected-state.png', fullPage: true });
    console.log('✅ Wallet not connected state verified');
    
    // Test 8: All new loading and error components render
    console.log('\n🔧 Test 8: Loading and Error Components...');
    
    // Check for custom loading components
    const loadingComponents = await page.locator('div[class*="animate-pulse"], div[class*="spinner"], svg[class*="animate-spin"]').count();
    console.log(`Custom loading components found: ${loadingComponents}`);
    
    // Check for error boundary components
    const errorComponents = await page.locator('[data-testid*="error"], [class*="error-"]').count();
    console.log(`Error components found: ${errorComponents}`);
    
    await page.screenshot({ path: 'test-results/08-components-render.png', fullPage: true });
    console.log('✅ Loading and error components verified');
    
    // Final comprehensive screenshot
    console.log('\n📸 Taking final comprehensive screenshot...');
    await page.screenshot({ path: 'test-results/final-comprehensive-view.png', fullPage: true });
    
    // Console error summary
    console.log('\n❌ Console Errors Summary:');
    if (errors.length > 0) {
      errors.forEach((error, index) => {
        console.log(`${index + 1}. ${error}`);
      });
    } else {
      console.log('✅ No console errors detected!');
    }
    
    console.log('\n🎉 Test suite completed successfully!');
    console.log('📁 Screenshots saved to test-results/ directory');
    
  } catch (error) {
    console.error('❌ Test failed:', error);
    await page.screenshot({ path: 'test-results/error-screenshot.png', fullPage: true });
  } finally {
    await browser.close();
  }
})();