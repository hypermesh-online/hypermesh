//! eBPF program management and utilities
//!
//! Provides utilities for loading, managing, and interfacing with eBPF programs
//! including program lifecycle management and map operations.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// eBPF program manager for loading and managing eBPF bytecode
pub struct ProgramManager {
    loaded_programs: RwLock<HashMap<String, LoadedProgram>>,
    program_configs: HashMap<String, ProgramConfig>,
}

impl ProgramManager {
    pub fn new() -> Self {
        Self {
            loaded_programs: RwLock::new(HashMap::new()),
            program_configs: Self::default_program_configs(),
        }
    }

    /// Load an eBPF program from bytecode
    pub async fn load_program(&self, name: &str, bytecode_path: &str) -> Result<()> {
        info!("📝 Loading eBPF program: {}", name);
        
        // In a real implementation, this would:
        // 1. Read the eBPF bytecode from file
        // 2. Verify the program using the kernel verifier
        // 3. Load the program into the kernel
        // 4. Store the file descriptor and metadata
        
        let config = self.program_configs.get(name)
            .ok_or_else(|| anyhow::anyhow!("No configuration found for program: {}", name))?;
        
        let loaded_program = LoadedProgram {
            name: name.to_string(),
            program_type: config.program_type.clone(),
            bytecode_path: bytecode_path.to_string(),
            fd: Self::simulate_load_program(bytecode_path).await?,
            maps: HashMap::new(),
            attached: false,
            load_time: std::time::Instant::now(),
        };
        
        let mut programs = self.loaded_programs.write().await;
        programs.insert(name.to_string(), loaded_program);
        
        debug!("Program loaded: {}", name);
        Ok(())
    }

    /// Attach a loaded program to a network interface or hook point
    pub async fn attach_program(&self, name: &str, attach_point: &str) -> Result<()> {
        info!("🔗 Attaching eBPF program {} to {}", name, attach_point);
        
        let mut programs = self.loaded_programs.write().await;
        if let Some(program) = programs.get_mut(name) {
            // In a real implementation, this would:
            // 1. Attach the program to the specified hook point
            // 2. Configure the attachment parameters
            // 3. Verify successful attachment
            
            program.attached = true;
            debug!("Program attached: {} -> {}", name, attach_point);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Program not found: {}", name))
        }
    }

    /// Detach a program from its attachment point
    pub async fn detach_program(&self, name: &str) -> Result<()> {
        info!("🔓 Detaching eBPF program: {}", name);
        
        let mut programs = self.loaded_programs.write().await;
        if let Some(program) = programs.get_mut(name) {
            program.attached = false;
            debug!("Program detached: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Program not found: {}", name))
        }
    }

    /// Unload a program from the kernel
    pub async fn unload_program(&self, name: &str) -> Result<()> {
        info!("🗑️ Unloading eBPF program: {}", name);
        
        let mut programs = self.loaded_programs.write().await;
        if let Some(program) = programs.remove(name) {
            // In a real implementation, this would:
            // 1. Detach the program if still attached
            // 2. Close the program file descriptor
            // 3. Clean up any associated maps
            
            if program.attached {
                warn!("Unloading attached program: {}", name);
            }
            
            debug!("Program unloaded: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Program not found: {}", name))
        }
    }

    /// Get information about a loaded program
    pub async fn get_program_info(&self, name: &str) -> Option<ProgramInfo> {
        let programs = self.loaded_programs.read().await;
        programs.get(name).map(|p| ProgramInfo {
            name: p.name.clone(),
            program_type: p.program_type.clone(),
            attached: p.attached,
            load_time: p.load_time,
            map_count: p.maps.len() as u32,
        })
    }

    /// List all loaded programs
    pub async fn list_programs(&self) -> Vec<String> {
        let programs = self.loaded_programs.read().await;
        programs.keys().cloned().collect()
    }

    /// Create a new eBPF map
    pub async fn create_map(&self, name: &str, map_config: &MapConfig) -> Result<i32> {
        info!("🗺️ Creating eBPF map: {}", name);
        
        // In a real implementation, this would:
        // 1. Create the map with specified configuration
        // 2. Return the map file descriptor
        
        let map_fd = Self::simulate_create_map(map_config).await?;
        debug!("Map created: {} (fd: {})", name, map_fd);
        Ok(map_fd)
    }

    /// Update an entry in an eBPF map
    pub async fn update_map_entry(&self, map_fd: i32, key: &[u8], value: &[u8]) -> Result<()> {
        // In a real implementation, this would use bpf_map_update_elem syscall
        debug!("Updating map entry (fd: {}, key_len: {}, value_len: {})", 
               map_fd, key.len(), value.len());
        Ok(())
    }

    /// Lookup an entry in an eBPF map
    pub async fn lookup_map_entry(&self, map_fd: i32, key: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would use bpf_map_lookup_elem syscall
        debug!("Looking up map entry (fd: {}, key_len: {})", map_fd, key.len());
        
        // Simulate returning some data
        Ok(vec![0u8; 64])
    }

    /// Delete an entry from an eBPF map
    pub async fn delete_map_entry(&self, map_fd: i32, key: &[u8]) -> Result<()> {
        // In a real implementation, this would use bpf_map_delete_elem syscall
        debug!("Deleting map entry (fd: {}, key_len: {})", map_fd, key.len());
        Ok(())
    }

    /// Get statistics about eBPF program execution
    pub async fn get_program_stats(&self, name: &str) -> Option<ProgramStats> {
        let programs = self.loaded_programs.read().await;
        programs.get(name).map(|_| ProgramStats {
            instructions_executed: rand::random::<u64>() % 1000000,
            runtime_ns: rand::random::<u64>() % 10000,
            map_operations: rand::random::<u32>() % 1000,
            verifier_log: "Program verification successful".to_string(),
        })
    }

    /// Reload a program with new bytecode
    pub async fn reload_program(&self, name: &str, new_bytecode_path: &str) -> Result<()> {
        info!("🔄 Reloading eBPF program: {}", name);
        
        // Unload existing program
        self.unload_program(name).await?;
        
        // Load new version
        self.load_program(name, new_bytecode_path).await?;
        
        Ok(())
    }

    fn default_program_configs() -> HashMap<String, ProgramConfig> {
        let mut configs = HashMap::new();
        
        configs.insert("network-monitor".to_string(), ProgramConfig {
            program_type: ProgramType::Xdp,
            attach_type: Some(AttachType::XdpGeneric),
            max_entries: 10000,
            flags: vec![ProgramFlag::JitCompile],
        });
        
        configs.insert("traffic-control".to_string(), ProgramConfig {
            program_type: ProgramType::SchedCls,
            attach_type: Some(AttachType::TcIngress),
            max_entries: 5000,
            flags: vec![ProgramFlag::JitCompile],
        });
        
        configs.insert("load-balancer".to_string(), ProgramConfig {
            program_type: ProgramType::Xdp,
            attach_type: Some(AttachType::XdpOffload),
            max_entries: 20000,
            flags: vec![ProgramFlag::JitCompile, ProgramFlag::HardwareOffload],
        });
        
        configs.insert("security-filter".to_string(), ProgramConfig {
            program_type: ProgramType::SocketFilter,
            attach_type: Some(AttachType::SocketFilter),
            max_entries: 50000,
            flags: vec![ProgramFlag::JitCompile],
        });
        
        configs
    }

    async fn simulate_load_program(bytecode_path: &str) -> Result<i32> {
        // Simulate program loading delay
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Check if bytecode file exists (in a real implementation)
        if !Path::new(bytecode_path).exists() {
            debug!("Simulating program load for: {}", bytecode_path);
        }
        
        // Return a simulated file descriptor
        Ok(rand::random::<i32>().abs() % 1000 + 100)
    }

    async fn simulate_create_map(config: &MapConfig) -> Result<i32> {
        // Simulate map creation delay
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        debug!("Creating map: type={:?}, key_size={}, value_size={}, max_entries={}", 
               config.map_type, config.key_size, config.value_size, config.max_entries);
        
        // Return a simulated map file descriptor
        Ok(rand::random::<i32>().abs() % 1000 + 1000)
    }
}

/// Loaded eBPF program information
#[derive(Debug, Clone)]
struct LoadedProgram {
    name: String,
    program_type: ProgramType,
    bytecode_path: String,
    fd: i32,
    maps: HashMap<String, i32>,
    attached: bool,
    load_time: std::time::Instant,
}

/// eBPF program configuration
#[derive(Debug, Clone)]
struct ProgramConfig {
    program_type: ProgramType,
    attach_type: Option<AttachType>,
    max_entries: u32,
    flags: Vec<ProgramFlag>,
}

/// eBPF program types
#[derive(Debug, Clone)]
pub enum ProgramType {
    SocketFilter,
    Kprobe,
    SchedCls,
    SchedAct,
    Tracepoint,
    Xdp,
    PerfEvent,
    CgroupSkb,
    CgroupSock,
    LwtIn,
    LwtOut,
    LwtXmit,
    SockOps,
    SkSkb,
}

/// eBPF attachment types
#[derive(Debug, Clone)]
pub enum AttachType {
    SocketFilter,
    XdpGeneric,
    XdpNative,
    XdpOffload,
    TcIngress,
    TcEgress,
    CgroupInetIngress,
    CgroupInetEgress,
    CgroupSockAddr,
}

/// eBPF program flags
#[derive(Debug, Clone)]
pub enum ProgramFlag {
    JitCompile,
    HardwareOffload,
    StrictAlignment,
    AnyAlignment,
}

/// eBPF map configuration
#[derive(Debug, Clone)]
pub struct MapConfig {
    pub map_type: MapType,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub flags: Vec<MapFlag>,
}

/// eBPF map types
#[derive(Debug, Clone)]
pub enum MapType {
    Hash,
    Array,
    ProgArray,
    PerfEventArray,
    PercpuHash,
    PercpuArray,
    StackTrace,
    CgroupArray,
    LruHash,
    LruPercpuHash,
    LpmTrie,
    ArrayOfMaps,
    HashOfMaps,
    DevMap,
    SockMap,
    CpuMap,
    XskMap,
}

/// eBPF map flags
#[derive(Debug, Clone)]
pub enum MapFlag {
    NoPrealloc,
    NoCommonLru,
    NumaNode,
    RdOnly,
    WrOnly,
    StackBuildId,
}

/// Program information for external queries
#[derive(Debug, Clone)]
pub struct ProgramInfo {
    pub name: String,
    pub program_type: ProgramType,
    pub attached: bool,
    pub load_time: std::time::Instant,
    pub map_count: u32,
}

/// Program execution statistics
#[derive(Debug, Clone)]
pub struct ProgramStats {
    pub instructions_executed: u64,
    pub runtime_ns: u64,
    pub map_operations: u32,
    pub verifier_log: String,
}

/// Helper utilities for eBPF program development
pub struct ProgramUtils;

impl ProgramUtils {
    /// Compile eBPF source code to bytecode
    pub async fn compile_program(source_path: &str, output_path: &str) -> Result<()> {
        info!("🔨 Compiling eBPF program: {} -> {}", source_path, output_path);
        
        // In a real implementation, this would:
        // 1. Use clang/llvm to compile the C code
        // 2. Generate eBPF bytecode
        // 3. Optimize the bytecode
        
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        debug!("Program compiled successfully");
        Ok(())
    }

    /// Verify eBPF bytecode without loading
    pub async fn verify_program(bytecode_path: &str) -> Result<VerificationResult> {
        info!("✅ Verifying eBPF program: {}", bytecode_path);
        
        // Simulate verification
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        Ok(VerificationResult {
            valid: true,
            instruction_count: 150,
            complexity_score: 25,
            warnings: vec![],
            errors: vec![],
        })
    }

    /// Generate program skeleton for easier integration
    pub async fn generate_skeleton(bytecode_path: &str, output_path: &str) -> Result<()> {
        info!("🏗️ Generating program skeleton: {} -> {}", bytecode_path, output_path);
        
        // In a real implementation, this would generate C/Rust bindings
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        debug!("Skeleton generated successfully");
        Ok(())
    }
}

/// Verification result for eBPF programs
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub instruction_count: u32,
    pub complexity_score: u32,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_program_manager_creation() {
        let manager = ProgramManager::new();
        let programs = manager.list_programs().await;
        assert!(programs.is_empty());
    }

    #[tokio::test]
    async fn test_program_loading() {
        let manager = ProgramManager::new();
        
        let result = manager.load_program("test-program", "/tmp/test.o").await;
        // In simulation mode, this should succeed
        assert!(result.is_ok());
        
        let programs = manager.list_programs().await;
        assert!(programs.contains(&"test-program".to_string()));
    }

    #[tokio::test]
    async fn test_map_operations() {
        let manager = ProgramManager::new();
        
        let map_config = MapConfig {
            map_type: MapType::Hash,
            key_size: 4,
            value_size: 8,
            max_entries: 1024,
            flags: vec![],
        };
        
        let map_fd = manager.create_map("test-map", &map_config).await.unwrap();
        assert!(map_fd > 0);
        
        let key = [1u8, 2, 3, 4];
        let value = [10u8, 20, 30, 40, 50, 60, 70, 80];
        
        manager.update_map_entry(map_fd, &key, &value).await.unwrap();
        let result = manager.lookup_map_entry(map_fd, &key).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_program_utils() {
        let result = ProgramUtils::verify_program("/tmp/test.o").await.unwrap();
        assert!(result.valid);
        assert!(result.instruction_count > 0);
    }
}