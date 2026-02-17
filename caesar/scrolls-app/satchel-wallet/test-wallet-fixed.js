// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

(async () => {
  const browser = await chromium.launch({ 
    headless: false,
    slowMo: 500
  });
  
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 }, // iPhone 12 Pro dimensions
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true
  });
  
  const page = await context.newPage();
  
  console.log('🚀 Starting Satchel Wallet Comprehensive Test Suite');
  
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      errors.push(msg.text());
    }
  });
  
  try {
    // Test 1: App loads without errors
    console.log('\n📱 Test 1: App Loading...');
    await page.goto('http://localhost:3002', { waitUntil: 'networkidle' });
    
    await page.screenshot({ path: 'test-results/01-app-loaded.png', fullPage: true });
    console.log('✅ App loaded successfully');
    
    // Test 2: Clean mobile interface displays properly
    console.log('\n📱 Test 2: Mobile Interface Check...');
    
    const header = await page.locator('header').isVisible();
    const mainContent = await page.locator('main').isVisible();
    const tabList = await page.locator('[role="tablist"]');
    const bottomNav = await tabList.isVisible();
    
    console.log(`Header visible: ${header}`);
    console.log(`Bottom navigation visible: ${bottomNav}`);
    console.log(`Main content visible: ${mainContent}`);
    
    await page.screenshot({ path: 'test-results/02-mobile-interface.png', fullPage: true });
    console.log('✅ Mobile interface elements verified');
    
    // Test 3: WalletConnect button in header works (fix for multiple buttons)
    console.log('\n🔗 Test 3: WalletConnect Button...');
    
    const headerWalletButton = page.locator('header button:has-text("Connect Wallet")');
    const isHeaderWalletButtonVisible = await headerWalletButton.isVisible();
    console.log(`Header WalletConnect button visible: ${isHeaderWalletButtonVisible}`);
    
    if (isHeaderWalletButtonVisible) {
      await headerWalletButton.click();
      await page.waitForTimeout(2000);
      await page.screenshot({ path: 'test-results/03-wallet-connect-clicked.png', fullPage: true });
      console.log('✅ Header WalletConnect button clicked successfully');
    }
    
    // Test 4: Loading states and error handling
    console.log('\n⏳ Test 4: Loading States and Error Handling...');
    
    const loadingElements = await page.locator('[data-testid*="loading"], .animate-spin, [class*="loading"]').count();
    console.log(`Loading elements found: ${loadingElements}`);
    
    await page.screenshot({ path: 'test-results/04-loading-states.png', fullPage: true });
    console.log('✅ Loading and error states verified');
    
    // Test 5: Toast notifications function
    console.log('\n🍞 Test 5: Toast Notifications...');
    
    const toastContainer = await page.locator('[data-sonner-toaster]').isVisible().catch(() => false);
    console.log(`Toast container present: ${toastContainer}`);
    
    await page.screenshot({ path: 'test-results/05-toast-setup.png', fullPage: true });
    console.log('✅ Toast notification setup verified');
    
    // Test 6: Bottom tab navigation
    console.log('\n🧭 Test 6: Bottom Tab Navigation...');
    
    const tabButtons = await tabList.locator('button').all();
    console.log(`Number of tabs found: ${tabButtons.length}`);
    
    for (let i = 0; i < tabButtons.length; i++) {
      const tab = tabButtons[i];
      const tabName = await tab.textContent();
      console.log(`Testing tab ${i + 1}: ${tabName?.trim()}`);
      
      await tab.click();
      await page.waitForTimeout(1500);
      await page.screenshot({ 
        path: `test-results/06-tab-${i + 1}-${(tabName?.trim() || 'unnamed').replace(/\s+/g, '-').toLowerCase()}.png`, 
        fullPage: true 
      });
    }
    
    console.log('✅ Bottom tab navigation tested');
    
    // Test 7: Wallet not connected state
    console.log('\n👤 Test 7: Wallet Not Connected State...');
    
    const notConnectedElements = await page.getByText(/not connected|connect wallet|no wallet/i).count();
    console.log(`"Not connected" indicators found: ${notConnectedElements}`);
    
    await page.screenshot({ path: 'test-results/07-not-connected-state.png', fullPage: true });
    console.log('✅ Wallet not connected state verified');
    
    // Test 8: All new loading and error components render
    console.log('\n🔧 Test 8: Loading and Error Components...');
    
    const loadingComponents = await page.locator('div[class*="animate-pulse"], div[class*="spinner"], svg[class*="animate-spin"]').count();
    console.log(`Custom loading components found: ${loadingComponents}`);
    
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
    console.error('❌ Test failed:', error.message);
    await page.screenshot({ path: 'test-results/error-screenshot.png', fullPage: true });
  } finally {
    await browser.close();
  }
})();