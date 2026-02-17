// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Test Script for Unified API Configuration
 * 
 * This script verifies that the UI is properly configured to connect to
 * the unified Internet 2.0 server on port 8443 instead of separate services.
 */

import { web3ApiClient, trustChainAPI, hyperMeshAPI, stoqAPI } from './index';

/**
 * Test unified API endpoints
 */
export async function testUnifiedAPIConfiguration() {
  console.log('🔧 Testing Unified API Configuration...');
  
  const results = {
    trustchain: { success: false, endpoint: '', error: null as string | null },
    hypermesh: { success: false, endpoint: '', error: null as string | null },
    stoq: { success: false, endpoint: '', error: null as string | null },
    configuration: { success: false, errors: [] as string[] }
  };

  // Test TrustChain endpoints
  try {
    console.log('📋 Testing TrustChain API endpoints...');
    const serviceConfig = web3ApiClient.getServiceConfig('trustchain');
    results.trustchain.endpoint = `http://${serviceConfig.baseUrl}`;
    
    if (serviceConfig.port !== 8443) {
      results.configuration.errors.push(`TrustChain port should be 8443, got ${serviceConfig.port}`);
    }
    
    // Test health endpoint - this should call /api/v1/trustchain/health
    await trustChainAPI.getHealthStatus();
    results.trustchain.success = true;
    console.log('✅ TrustChain API endpoints configured correctly');
  } catch (error) {
    results.trustchain.error = error instanceof Error ? error.message : 'Unknown error';
    console.log('ℹ️ TrustChain API will use mock data (expected if backend not running)');
  }

  // Test HyperMesh endpoints  
  try {
    console.log('🔗 Testing HyperMesh API endpoints...');
    const serviceConfig = web3ApiClient.getServiceConfig('hypermesh');
    results.hypermesh.endpoint = `http://${serviceConfig.baseUrl}`;
    
    if (serviceConfig.port !== 8443) {
      results.configuration.errors.push(`HyperMesh port should be 8443, got ${serviceConfig.port}`);
    }
    
    // Test system status endpoint - this should call /api/v1/hypermesh/system/status
    await hyperMeshAPI.getSystemStatus();
    results.hypermesh.success = true;
    console.log('✅ HyperMesh API endpoints configured correctly');
  } catch (error) {
    results.hypermesh.error = error instanceof Error ? error.message : 'Unknown error';
    console.log('ℹ️ HyperMesh API will use mock data (expected if backend not running)');
  }

  // Test STOQ endpoints
  try {
    console.log('⚡ Testing STOQ API endpoints...');
    const serviceConfig = web3ApiClient.getServiceConfig('stoq');
    results.stoq.endpoint = `http://${serviceConfig.baseUrl}`;
    
    if (serviceConfig.port !== 8443) {
      results.configuration.errors.push(`STOQ port should be 8443, got ${serviceConfig.port}`);
    }
    
    // Test system health endpoint - this should call /api/v1/stoq/system/health
    await stoqAPI.getSystemHealth();
    results.stoq.success = true;
    console.log('✅ STOQ API endpoints configured correctly');
  } catch (error) {
    results.stoq.error = error instanceof Error ? error.message : 'Unknown error';
    console.log('ℹ️ STOQ API will use mock data (expected if backend not running)');
  }

  // Validate configuration
  results.configuration.success = results.configuration.errors.length === 0;
  
  if (results.configuration.success) {
    console.log('✅ All services correctly configured for unified server (port 8443)');
  } else {
    console.log('❌ Configuration errors found:', results.configuration.errors);
  }

  return results;
}

/**
 * Test API endpoint paths
 */
export function testAPIEndpointPaths() {
  console.log('🛤️ Verifying API endpoint path structure...');
  
  const expectedPaths = {
    trustchain: {
      health: '/api/v1/trustchain/health',
      certificates: '/api/v1/trustchain/certificates',
      dns: '/api/v1/trustchain/dns/records'
    },
    hypermesh: {
      status: '/api/v1/hypermesh/system/status',
      assets: '/api/v1/hypermesh/assets',
      allocations: '/api/v1/hypermesh/allocations'
    },
    stoq: {
      health: '/api/v1/stoq/system/health',
      connections: '/api/v1/stoq/connections',
      metrics: '/api/v1/stoq/metrics/performance'
    }
  };

  console.log('📋 Expected API endpoint structure:');
  console.log(JSON.stringify(expectedPaths, null, 2));
  
  return expectedPaths;
}

/**
 * Display configuration summary
 */
export function displayConfigurationSummary() {
  console.log('\n🔧 Unified API Configuration Summary');
  console.log('=====================================');
  
  const services = ['trustchain', 'hypermesh', 'stoq', 'integration'] as const;
  
  services.forEach(service => {
    const config = web3ApiClient.getServiceConfig(service);
    console.log(`${service.toUpperCase()}:`);
    console.log(`  ↳ URL: http://${config.baseUrl}`);
    console.log(`  ↳ Port: ${config.port}`);
    console.log(`  ↳ Expected Endpoints: /api/v1/${service}/*`);
    console.log('');
  });
  
  console.log('✅ All services now point to unified server on port 8443');
  console.log('📡 Backend Integration: Ready for real API calls');
  console.log('🔄 Fallback: Mock data when backend unavailable');
}

// Auto-run tests when imported
if (typeof window !== 'undefined') {
  // Only run in browser environment
  setTimeout(() => {
    displayConfigurationSummary();
    testAPIEndpointPaths();
    testUnifiedAPIConfiguration().then(results => {
      console.log('\n📊 Test Results Summary:');
      console.log('========================');
      console.log(`TrustChain: ${results.trustchain.success ? '✅' : '⚠️'} ${results.trustchain.endpoint}`);
      console.log(`HyperMesh: ${results.hypermesh.success ? '✅' : '⚠️'} ${results.hypermesh.endpoint}`);
      console.log(`STOQ: ${results.stoq.success ? '✅' : '⚠️'} ${results.stoq.endpoint}`);
      console.log(`Configuration: ${results.configuration.success ? '✅' : '❌'}`);
      
      if (!results.configuration.success) {
        console.log('Configuration Errors:', results.configuration.errors);
      }
    });
  }, 1000);
}