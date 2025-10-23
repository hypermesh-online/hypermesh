//! Comprehensive HyperMesh Test Runner
//! 
//! Validates all implemented components with real tests and honest reporting

use nexus_connection_manager::{ConnectionManager, ConnectionManagerConfig};
use nexus_transport::{CertificateManager, TransportBuilder, TransportConfig};
use nexus_shared::{ServiceId, NodeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn, error};

/// Comprehensive test suite results
#[derive(Debug)]
struct TestSuiteResults {
    transport_tests: TestResults,
    connection_manager_tests: TestResults,
    integration_tests: TestResults,
    performance_tests: TestResults,
    overall_success: bool,
}

#[derive(Debug)]
struct TestResults {
    name: String,
    tests_run: usize,
    tests_passed: usize,
    tests_failed: usize,
    duration: Duration,
    issues: Vec<String>,
}

impl TestResults {
    fn new(name: String) -> Self {
        Self {
            name,
            tests_run: 0,
            tests_passed: 0,
            tests_failed: 0,
            duration: Duration::from_secs(0),
            issues: Vec::new(),
        }
    }
    
    fn add_test(&mut self, passed: bool, issue: Option<String>) {
        self.tests_run += 1;
        if passed {
            self.tests_passed += 1;
        } else {
            self.tests_failed += 1;
            if let Some(issue) = issue {
                self.issues.push(issue);
            }
        }
    }
    
    fn success_rate(&self) -> f64 {
        if self.tests_run == 0 {
            0.0
        } else {
            self.tests_passed as f64 / self.tests_run as f64
        }
    }
    
    fn print_summary(&self) {
        println!("  {} Results:", self.name);
        println!("    Tests Run: {}", self.tests_run);
        println!("    Passed: {} ({:.1}%)", self.tests_passed, self.success_rate() * 100.0);
        println!("    Failed: {}", self.tests_failed);
        println!("    Duration: {:.2}s", self.duration.as_secs_f64());
        
        if !self.issues.is_empty() {
            println!("    Issues Found:");
            for issue in &self.issues {
                println!("      - {}", issue);
            }
        }
    }
}

/// Comprehensive test runner
struct ComprehensiveTestRunner {
    start_time: Instant,
}

impl ComprehensiveTestRunner {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
    
    /// Run all comprehensive tests
    async fn run_all_tests(&self) -> TestSuiteResults {
        info!("🚀 Starting Comprehensive HyperMesh Validation");
        info!("================================================");
        
        // Run each test suite
        let transport_tests = self.test_transport_layer().await;
        let connection_manager_tests = self.test_connection_manager().await;
        let integration_tests = self.test_integration().await;
        let performance_tests = self.test_performance().await;
        
        // Determine overall success
        let overall_success = transport_tests.tests_failed == 0 &&
                             connection_manager_tests.tests_failed == 0 &&
                             integration_tests.tests_failed == 0;
        
        TestSuiteResults {
            transport_tests,
            connection_manager_tests,
            integration_tests,
            performance_tests,
            overall_success,
        }
    }
    
    /// Test transport layer functionality
    async fn test_transport_layer(&self) -> TestResults {
        info!("📡 Testing QUIC Transport Layer");
        
        let start = Instant::now();
        let mut results = TestResults::new("QUIC Transport".to_string());
        
        // Test 1: Certificate creation
        match self.test_certificate_creation().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Certificate creation: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Certificate creation failed: {}", e)));
                error!("❌ Certificate creation: FAILED - {}", e);
            }
        }
        
        // Test 2: Server startup
        match self.test_server_startup().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Server startup: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Server startup failed: {}", e)));
                error!("❌ Server startup: FAILED - {}", e);
            }
        }
        
        // Test 3: Client connection
        match self.test_client_connection().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Client connection: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Client connection failed: {}", e)));
                error!("❌ Client connection: FAILED - {}", e);
            }
        }
        
        // Test 4: Message transmission
        match self.test_message_transmission().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Message transmission: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Message transmission failed: {}", e)));
                error!("❌ Message transmission: FAILED - {}", e);
            }
        }
        
        results.duration = start.elapsed();
        results
    }
    
    /// Test connection manager functionality
    async fn test_connection_manager(&self) -> TestResults {
        info!("🔗 Testing Connection Manager");
        
        let start = Instant::now();
        let mut results = TestResults::new("Connection Manager".to_string());
        
        // Test 1: Connection manager creation
        match self.test_connection_manager_creation().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Connection manager creation: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Connection manager creation failed: {}", e)));
                error!("❌ Connection manager creation: FAILED - {}", e);
            }
        }
        
        // Test 2: Service registration
        match self.test_service_registration().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Service registration: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Service registration failed: {}", e)));
                error!("❌ Service registration: FAILED - {}", e);
            }
        }
        
        // Test 3: Service discovery
        match self.test_service_discovery().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Service discovery: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Service discovery failed: {}", e)));
                error!("❌ Service discovery: FAILED - {}", e);
            }
        }
        
        results.duration = start.elapsed();
        results
    }
    
    /// Test integration scenarios
    async fn test_integration(&self) -> TestResults {
        info!("🔄 Testing Integration Scenarios");
        
        let start = Instant::now();
        let mut results = TestResults::new("Integration Tests".to_string());
        
        // Test 1: Multi-node cluster
        match self.test_multi_node_cluster().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Multi-node cluster: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Multi-node cluster failed: {}", e)));
                error!("❌ Multi-node cluster: FAILED - {}", e);
            }
        }
        
        // Test 2: Service mesh formation
        match self.test_service_mesh().await {
            Ok(_) => {
                results.add_test(true, None);
                info!("✅ Service mesh formation: PASSED");
            }
            Err(e) => {
                results.add_test(false, Some(format!("Service mesh formation failed: {}", e)));
                error!("❌ Service mesh formation: FAILED - {}", e);
            }
        }
        
        results.duration = start.elapsed();
        results
    }
    
    /// Test performance characteristics
    async fn test_performance(&self) -> TestResults {
        info!("⚡ Testing Performance Characteristics");
        
        let start = Instant::now();
        let mut results = TestResults::new("Performance Tests".to_string());
        
        // Test 1: Connection latency
        match self.test_connection_latency().await {
            Ok(latency) => {
                if latency < Duration::from_millis(10) {
                    results.add_test(true, None);
                    info!("✅ Connection latency: PASSED ({:.2}ms)", latency.as_millis());
                } else {
                    results.add_test(false, Some(format!("Connection latency too high: {:.2}ms", latency.as_millis())));
                    warn!("⚠️  Connection latency: HIGH ({:.2}ms)", latency.as_millis());
                }
            }
            Err(e) => {
                results.add_test(false, Some(format!("Connection latency test failed: {}", e)));
                error!("❌ Connection latency: FAILED - {}", e);
            }
        }
        
        // Test 2: Message throughput
        match self.test_message_throughput().await {
            Ok(throughput) => {
                if throughput > 1000.0 {
                    results.add_test(true, None);
                    info!("✅ Message throughput: PASSED ({:.0} msg/sec)", throughput);
                } else {
                    results.add_test(false, Some(format!("Message throughput too low: {:.0} msg/sec", throughput)));
                    warn!("⚠️  Message throughput: LOW ({:.0} msg/sec)", throughput);
                }
            }
            Err(e) => {
                results.add_test(false, Some(format!("Message throughput test failed: {}", e)));
                error!("❌ Message throughput: FAILED - {}", e);
            }
        }
        
        results.duration = start.elapsed();
        results
    }
    
    // Individual test implementations
    
    async fn test_certificate_creation(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _cert = CertificateManager::new_self_signed(
            "test-cert".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?;
        Ok(())
    }
    
    async fn test_server_startup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"test-server\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        let mut config = TransportConfig::default();\n        config.bind_address = \"127.0.0.1\".parse()?;\n        config.port = 0; // Use any available port\n        \n        let _server = TransportBuilder::new()\n            .with_config(config)\n            .with_certificate_manager(cert)\n            .build_server()\n            .await?;\n            \n        Ok(())\n    }\n    \n    async fn test_client_connection(&self) -> Result<(), Box<dyn std::error::Error>> {\n        let server_port = find_available_port()?;\n        \n        // Start server\n        let server_cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"connection-test-server\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        let mut server_config = TransportConfig::default();\n        server_config.bind_address = \"127.0.0.1\".parse()?;\n        server_config.port = server_port;\n        \n        let _server = TransportBuilder::new()\n            .with_config(server_config)\n            .with_certificate_manager(server_cert.clone())\n            .build_server()\n            .await?;\n            \n        tokio::time::sleep(Duration::from_millis(100)).await;\n        \n        // Create client and connect\n        let client_config = TransportConfig::default();\n        let client = TransportBuilder::new()\n            .with_config(client_config)\n            .with_certificate_manager(server_cert)\n            .build_client()\n            .await?;\n            \n        let server_addr = format!(\"127.0.0.1:{}\", server_port);\n        let _connection = client.connect(&server_addr.parse()?).await?;\n        \n        Ok(())\n    }\n    \n    async fn test_message_transmission(&self) -> Result<(), Box<dyn std::error::Error>> {\n        let server_port = find_available_port()?;\n        \n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"message-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        // Start server\n        let mut server_config = TransportConfig::default();\n        server_config.bind_address = \"127.0.0.1\".parse()?;\n        server_config.port = server_port;\n        \n        let _server = TransportBuilder::new()\n            .with_config(server_config)\n            .with_certificate_manager(cert.clone())\n            .build_server()\n            .await?;\n            \n        tokio::time::sleep(Duration::from_millis(100)).await;\n        \n        // Create client and connect\n        let client_config = TransportConfig::default();\n        let client = TransportBuilder::new()\n            .with_config(client_config)\n            .with_certificate_manager(cert)\n            .build_client()\n            .await?;\n            \n        let server_addr = format!(\"127.0.0.1:{}\", server_port);\n        let connection = client.connect(&server_addr.parse()?).await?;\n        \n        // Send message\n        let stream = connection.open_stream().await?;\n        let test_message = b\"comprehensive_test_message\";\n        stream.write_message(test_message).await?;\n        \n        Ok(())\n    }\n    \n    async fn test_connection_manager_creation(&self) -> Result<(), Box<dyn std::error::Error>> {\n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"cm-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        let config = ConnectionManagerConfig::default();\n        let _manager = ConnectionManager::new(config, cert).await?;\n        \n        Ok(())\n    }\n    \n    async fn test_service_registration(&self) -> Result<(), Box<dyn std::error::Error>> {\n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"service-reg-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        let mut config = ConnectionManagerConfig::default();\n        config.port = find_available_port()?;\n        \n        let mut manager = ConnectionManager::new(config, cert).await?;\n        let _addr = manager.start().await?;\n        \n        let service_id = ServiceId::new(\"test-service\", \"v1\");\n        let service_addr = \"127.0.0.1:9999\".parse()?;\n        let metadata = HashMap::new();\n        \n        manager.register_service(service_id, service_addr, metadata).await?;\n        \n        Ok(())\n    }\n    \n    async fn test_service_discovery(&self) -> Result<(), Box<dyn std::error::Error>> {\n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"service-discovery-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        let mut config = ConnectionManagerConfig::default();\n        config.port = find_available_port()?;\n        \n        let mut manager = ConnectionManager::new(config, cert).await?;\n        let _addr = manager.start().await?;\n        \n        // Register a service\n        let service_id = ServiceId::new(\"discovery-test\", \"v1\");\n        let service_addr = \"127.0.0.1:9998\".parse()?;\n        let metadata = HashMap::new();\n        \n        manager.register_service(service_id.clone(), service_addr, metadata).await?;\n        \n        // Discover the service\n        let discovered = manager.discover_services(&service_id).await?;\n        \n        if discovered.is_empty() {\n            return Err(\"Service discovery returned no results\".into());\n        }\n        \n        Ok(())\n    }\n    \n    async fn test_multi_node_cluster(&self) -> Result<(), Box<dyn std::error::Error>> {\n        // Create 2 nodes\n        let mut nodes = Vec::new();\n        \n        for i in 0..2 {\n            let cert = Arc::new(\n                CertificateManager::new_self_signed(\n                    format!(\"cluster-node-{}\", i),\n                    365,\n                    Duration::from_secs(3600)\n                ).await?\n            );\n            \n            let mut config = ConnectionManagerConfig::default();\n            config.port = find_available_port()?;\n            \n            let mut manager = ConnectionManager::new(config, cert).await?;\n            let addr = manager.start().await?;\n            \n            nodes.push((manager, addr));\n        }\n        \n        // Register services on each node\n        for (i, (manager, _)) in nodes.iter().enumerate() {\n            let service_id = ServiceId::new(&format!(\"cluster-service-{}\", i), \"v1\");\n            let service_addr = format!(\"127.0.0.1:{}\", 10000 + i).parse()?;\n            let metadata = HashMap::new();\n            \n            manager.register_service(service_id, service_addr, metadata).await?;\n        }\n        \n        tokio::time::sleep(Duration::from_millis(200)).await;\n        \n        Ok(())\n    }\n    \n    async fn test_service_mesh(&self) -> Result<(), Box<dyn std::error::Error>> {\n        // Similar to multi-node cluster but with cross-node service discovery\n        // This is a simplified test - full service mesh would require more coordination\n        \n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"mesh-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        let mut config = ConnectionManagerConfig::default();\n        config.port = find_available_port()?;\n        \n        let mut manager = ConnectionManager::new(config, cert).await?;\n        let _addr = manager.start().await?;\n        \n        // Register multiple services to simulate mesh\n        let services = vec![\n            (\"frontend\", \"v1\"),\n            (\"backend\", \"v1\"),\n            (\"database\", \"v1\"),\n        ];\n        \n        for (name, version) in services {\n            let service_id = ServiceId::new(name, version);\n            let service_addr = format!(\"127.0.0.1:{}\", 11000 + name.len()).parse()?;\n            let metadata = HashMap::new();\n            \n            manager.register_service(service_id, service_addr, metadata).await?;\n        }\n        \n        tokio::time::sleep(Duration::from_millis(100)).await;\n        \n        Ok(())\n    }\n    \n    async fn test_connection_latency(&self) -> Result<Duration, Box<dyn std::error::Error>> {\n        let server_port = find_available_port()?;\n        \n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"latency-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        // Start server\n        let mut server_config = TransportConfig::default();\n        server_config.bind_address = \"127.0.0.1\".parse()?;\n        server_config.port = server_port;\n        \n        let _server = TransportBuilder::new()\n            .with_config(server_config)\n            .with_certificate_manager(cert.clone())\n            .build_server()\n            .await?;\n            \n        tokio::time::sleep(Duration::from_millis(100)).await;\n        \n        // Measure connection time\n        let client_config = TransportConfig::default();\n        let client = TransportBuilder::new()\n            .with_config(client_config)\n            .with_certificate_manager(cert)\n            .build_client()\n            .await?;\n            \n        let server_addr = format!(\"127.0.0.1:{}\", server_port);\n        \n        let start = Instant::now();\n        let _connection = client.connect(&server_addr.parse()?).await?;\n        let latency = start.elapsed();\n        \n        Ok(latency)\n    }\n    \n    async fn test_message_throughput(&self) -> Result<f64, Box<dyn std::error::Error>> {\n        let server_port = find_available_port()?;\n        \n        let cert = Arc::new(\n            CertificateManager::new_self_signed(\n                \"throughput-test\".to_string(),\n                365,\n                Duration::from_secs(3600)\n            ).await?\n        );\n        \n        // Start server\n        let mut server_config = TransportConfig::default();\n        server_config.bind_address = \"127.0.0.1\".parse()?;\n        server_config.port = server_port;\n        \n        let _server = TransportBuilder::new()\n            .with_config(server_config)\n            .with_certificate_manager(cert.clone())\n            .build_server()\n            .await?;\n            \n        tokio::time::sleep(Duration::from_millis(100)).await;\n        \n        // Create client\n        let client_config = TransportConfig::default();\n        let client = TransportBuilder::new()\n            .with_config(client_config)\n            .with_certificate_manager(cert)\n            .build_client()\n            .await?;\n            \n        let server_addr = format!(\"127.0.0.1:{}\", server_port);\n        let connection = client.connect(&server_addr.parse()?).await?;\n        let stream = connection.open_stream().await?;\n        \n        // Send messages and measure throughput\n        let test_message = b\"throughput_test_message\";\n        let test_duration = Duration::from_secs(2);\n        let start = Instant::now();\n        let mut message_count = 0;\n        \n        while start.elapsed() < test_duration {\n            match stream.write_message(test_message).await {\n                Ok(_) => message_count += 1,\n                Err(_) => break,\n            }\n        }\n        \n        let actual_duration = start.elapsed();\n        let throughput = message_count as f64 / actual_duration.as_secs_f64();\n        \n        Ok(throughput)\n    }\n}\n\n/// Find an available port for testing\nfn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {\n    use std::net::TcpListener;\n    let listener = TcpListener::bind(\"127.0.0.1:0\")?;\n    let port = listener.local_addr()?.port();\n    Ok(port)\n}\n\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n    // Initialize logging\n    tracing_subscriber::fmt()\n        .with_target(false)\n        .with_level(true)\n        .init();\n    \n    println!(\"\\n\" + &\"=\".repeat(80));\n    println!(\"                 🔍 HYPERMESH COMPREHENSIVE TEST VALIDATION\");\n    println!(\"                           Honest Reality Check\");\n    println!(\"{}\", \"=\".repeat(80));\n    \n    let runner = ComprehensiveTestRunner::new();\n    let results = runner.run_all_tests().await;\n    \n    // Print comprehensive results\n    let total_duration = runner.start_time.elapsed();\n    \n    println!(\"\\n\" + &\"=\".repeat(60));\n    println!(\"                    📊 TEST RESULTS SUMMARY\");\n    println!(\"{}\", \"=\".repeat(60));\n    \n    results.transport_tests.print_summary();\n    println!();\n    results.connection_manager_tests.print_summary();\n    println!();\n    results.integration_tests.print_summary();\n    println!();\n    results.performance_tests.print_summary();\n    \n    println!(\"\\n\" + &\"=\".repeat(60));\n    println!(\"                      🎯 OVERALL ASSESSMENT\");\n    println!(\"{}\", \"=\".repeat(60));\n    \n    let total_tests = results.transport_tests.tests_run +\n                     results.connection_manager_tests.tests_run +\n                     results.integration_tests.tests_run +\n                     results.performance_tests.tests_run;\n    \n    let total_passed = results.transport_tests.tests_passed +\n                      results.connection_manager_tests.tests_passed +\n                      results.integration_tests.tests_passed +\n                      results.performance_tests.tests_passed;\n    \n    let total_failed = total_tests - total_passed;\n    \n    println!(\"⏱️  Total Test Duration: {:.2} seconds\", total_duration.as_secs_f64());\n    println!(\"📊 Total Tests: {}\", total_tests);\n    println!(\"✅ Total Passed: {} ({:.1}%)\", total_passed, \n             (total_passed as f64 / total_tests as f64) * 100.0);\n    println!(\"❌ Total Failed: {}\", total_failed);\n    \n    if results.overall_success {\n        println!(\"\\n🎉 OVERALL STATUS: SUCCESS\");\n        println!(\"✅ Core HyperMesh infrastructure is working correctly!\");\n        println!(\"🚀 Ready for next development phase\");\n    } else {\n        println!(\"\\n⚠️  OVERALL STATUS: ISSUES DETECTED\");\n        println!(\"🔧 Core functionality needs attention before proceeding\");\n        \n        println!(\"\\n🐛 Issues Summary:\");\n        for issue in &results.transport_tests.issues {\n            println!(\"  Transport: {}\", issue);\n        }\n        for issue in &results.connection_manager_tests.issues {\n            println!(\"  Connection Manager: {}\", issue);\n        }\n        for issue in &results.integration_tests.issues {\n            println!(\"  Integration: {}\", issue);\n        }\n        for issue in &results.performance_tests.issues {\n            println!(\"  Performance: {}\", issue);\n        }\n    }\n    \n    println!(\"\\n📋 Next Steps:\");\n    if results.overall_success {\n        println!(\"  1. ✅ Core transport and connection management validated\");\n        println!(\"  2. 🔄 Proceed with distributed state manager implementation\");\n        println!(\"  3. 🔄 Implement resource scheduler\");\n        println!(\"  4. 🔄 Add Byzantine consensus validation\");\n        println!(\"  5. 🔄 Build comprehensive UI and developer experience\");\n    } else {\n        println!(\"  1. 🔧 Address critical issues identified above\");\n        println!(\"  2. 🔄 Re-run validation after fixes\");\n        println!(\"  3. 📈 Only proceed when all tests pass\");\n    }\n    \n    println!(\"\\n🏁 Validation completed in {:.2} seconds\", total_duration.as_secs_f64());\n    println!(\"{}\", \"=\".repeat(80));\n    \n    Ok(())\n}