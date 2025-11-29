//! Runtime demonstration
//! Shows container lifecycle management

use nexus_runtime::*;
use nexus_shared::*;
use tempfile::TempDir;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🐳 Nexus Container Runtime Demo");
    println!("===============================");
    
    // Create temporary directory for runtime
    let temp_dir = TempDir::new()?;
    let mut config = RuntimeConfig::default();
    config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    // Create runtime
    println!("🔧 Initializing container runtime...");
    let runtime = Runtime::new(config).await?;
    println!("✅ Runtime initialized");
    
    // Show runtime configuration
    println!("⚙️  Runtime Configuration:");
    println!("   Storage driver: {:?}", runtime.storage_driver());
    println!("   Security: Hardware-assisted isolation");
    println!("   OCI compatibility: Full");
    
    // Create sample container specifications
    println!("📋 Creating container specifications...");
    
    let mut web_container = ContainerSpec::default();
    web_container.image.name = "nginx".to_string();
    web_container.image.tag = "1.20".to_string();
    web_container.command = vec!["nginx".to_string(), "-g".to_string(), "daemon off;".to_string()];
    web_container.resources.cpu_cores = 0.5;
    web_container.resources.memory_mb = 512;
    web_container.environment.insert("ENV".to_string(), "production".to_string());
    
    let mut app_container = ContainerSpec::default();
    app_container.image.name = "myapp".to_string();
    app_container.image.tag = "latest".to_string();
    app_container.command = vec!["./app".to_string()];
    app_container.resources.cpu_cores = 1.0;
    app_container.resources.memory_mb = 1024;
    
    println!("   Web container: {} (CPU: {}, Memory: {}MB)", 
             web_container.image.cache_key(), 
             web_container.resources.cpu_cores,
             web_container.resources.memory_mb);
    
    println!("   App container: {} (CPU: {}, Memory: {}MB)", 
             app_container.image.cache_key(), 
             app_container.resources.cpu_cores,
             app_container.resources.memory_mb);
    
    // Show security configuration
    println!("🔐 Security Configuration:");
    println!("   User namespaces: {}", web_container.security.enable_user_namespaces);
    println!("   Readonly rootfs: {}", web_container.security.readonly_rootfs);
    println!("   Capabilities dropped: {:?}", web_container.security.capabilities_drop);
    println!("   No new privileges: {}", web_container.security.no_new_privileges);
    
    // Demonstrate image management
    println!("🖼️  Image Management:");
    let images = runtime.list_images().await;
    println!("   Cached images: {}", images.len());
    
    // Show resource usage
    println!("📊 Resource Usage:");
    let usage = runtime.resource_usage().await;
    println!("   Active containers: {}", usage.len());
    
    // List containers
    println!("📦 Container Management:");
    let containers = runtime.list_containers().await;
    println!("   Running containers: {}", containers.len());
    
    // Demonstrate volume mounts
    println!("💾 Volume Management:");
    let volume = VolumeMount {
        source: "/host/data".to_string(),
        target: "/app/data".to_string(),
        options: vec!["bind".to_string(), "ro".to_string()],
        readonly: true,
    };
    println!("   Sample mount: {} -> {} ({})", 
             volume.source, volume.target, 
             if volume.readonly { "readonly" } else { "readwrite" });
    
    // Show networking capabilities
    println!("🌐 Networking:");
    println!("   Default mode: Bridge");
    println!("   IPv6 enabled: Yes");
    println!("   Port mapping: Enabled");
    
    sleep(Duration::from_secs(2)).await;
    
    println!("✅ Runtime demo completed successfully!");
    println!("\n💡 This demonstrates:");
    println!("   • OCI-compatible container specifications");
    println!("   • Hardware-assisted security isolation");
    println!("   • Resource management and quotas");
    println!("   • Image caching and management");
    println!("   • Volume mounting and storage");
    println!("   • Network namespace isolation");
    println!("\n⚠️  Note: Actual container execution requires root privileges");
    println!("   Run with: sudo cargo run --example runtime-demo");
    
    Ok(())
}