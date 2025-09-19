#!/usr/bin/env node

/**
 * Backend Integration Test
 * Tests the three Python backend services to ensure they're working correctly
 */

const BASE_URLS = {
    trustchain: 'http://localhost:8444',
    stoq: 'http://localhost:8445', 
    hypermesh: 'http://localhost:8446'
};

async function testEndpoint(service, endpoint, description) {
    try {
        const url = `${BASE_URLS[service]}${endpoint}`;
        console.log(`\n🔍 Testing ${service.toUpperCase()}: ${description}`);
        console.log(`   URL: ${url}`);
        
        const response = await fetch(url);
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        const data = await response.json();
        console.log(`   ✅ SUCCESS - Status: ${response.status}`);
        
        // Show relevant data
        if (endpoint === '/health') {
            console.log(`   📊 Status: ${data.status}, Uptime: ${data.uptime_seconds}s`);
        } else if (endpoint.includes('/certificates')) {
            console.log(`   📜 Certificates: ${data.length} found`);
        } else if (endpoint.includes('/assets')) {
            console.log(`   🏗️  Assets: ${data.length} found`);
        } else if (endpoint.includes('/system/health')) {
            console.log(`   ⚡ Performance: ${data.performance?.global_throughput || 'N/A'} Mbps`);
        } else if (endpoint.includes('/system/status')) {
            console.log(`   📊 Assets: ${data.total_assets}, Allocations: ${data.active_allocations}`);
        }
        
        return { success: true, data };
    } catch (error) {
        console.log(`   ❌ FAILED - ${error.message}`);
        return { success: false, error: error.message };
    }
}

async function runIntegrationTests() {
    console.log('🚀 Starting Backend Integration Tests\n');
    console.log('Testing three Python backend services:');
    console.log('  • TrustChain CA (port 8444)');
    console.log('  • STOQ Transport (port 8445)');
    console.log('  • HyperMesh Assets (port 8446)');
    
    const tests = [
        // Health checks
        ['trustchain', '/health', 'Health Check'],
        ['stoq', '/health', 'Health Check'],
        ['hypermesh', '/health', 'Health Check'],
        
        // Core API endpoints
        ['trustchain', '/api/v1/certificates', 'Certificate Management'],
        ['trustchain', '/api/v1/status', 'System Status'],
        ['stoq', '/api/v1/system/health', 'System Health'],
        ['hypermesh', '/api/v1/assets', 'Asset Management'],
        ['hypermesh', '/api/v1/system/status', 'System Status'],
        
        // Additional functional tests
        ['trustchain', '/api/v1/trust/hierarchy', 'Trust Hierarchy'],
        ['stoq', '/api/v1/connections', 'QUIC Connections'],
        ['hypermesh', '/api/v1/allocations', 'Asset Allocations'],
    ];
    
    let passed = 0;
    let failed = 0;
    
    for (const [service, endpoint, description] of tests) {
        const result = await testEndpoint(service, endpoint, description);
        if (result.success) {
            passed++;
        } else {
            failed++;
        }
        
        // Small delay between tests
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    console.log('\n' + '='.repeat(60));
    console.log('📊 INTEGRATION TEST RESULTS');
    console.log('='.repeat(60));
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`📈 Success Rate: ${((passed / (passed + failed)) * 100).toFixed(1)}%`);
    
    if (failed === 0) {
        console.log('\n🎉 ALL TESTS PASSED! Backend services are fully operational.');
        console.log('\n🎯 Ready for frontend integration:');
        console.log('   1. Frontend should be able to connect to all three services');
        console.log('   2. All API endpoints are responding correctly');
        console.log('   3. Sample data is available for testing');
        console.log('\n💡 Next steps:');
        console.log('   • Start the frontend: cd ui && npm run dev');
        console.log('   • Open browser: http://localhost:5173 (or configured port)');
        console.log('   • The dashboard should show live data from all services');
    } else {
        console.log('\n⚠️  Some tests failed. Check service logs for details.');
        console.log('   • TrustChain logs: logs/trustchain-8444.log');
        console.log('   • STOQ logs: logs/stoq-8445.log');
        console.log('   • HyperMesh logs: logs/hypermesh-8446.log');
    }
    
    console.log('\n📋 Service URLs:');
    console.log('   • TrustChain API: http://localhost:8444/docs');
    console.log('   • STOQ API: http://localhost:8445/docs');
    console.log('   • HyperMesh API: http://localhost:8446/docs');
    
    return failed === 0;
}

// Run tests
runIntegrationTests()
    .then(success => {
        process.exit(success ? 0 : 1);
    })
    .catch(error => {
        console.error('❌ Test runner failed:', error);
        process.exit(1);
    });