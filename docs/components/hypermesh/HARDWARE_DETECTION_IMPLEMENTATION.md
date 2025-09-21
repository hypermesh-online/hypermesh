# Hardware Detection Implementation

## Overview
Implemented a comprehensive hardware detection service that replaces all hardcoded resource values in the UI with real system information. The service provides real-time CPU, memory, storage, and network detection through HTTP API endpoints.

## Implementation Components

### 1. Backend Hardware Detection Service (`src/hardware.rs`)
- **Real System Detection**:
  - CPU: Detects physical/logical cores, model, frequency, vendor, architecture, real-time usage
  - Memory: Total/available/used RAM, swap memory, usage percentage
  - Storage: All mounted drives with capacity, usage, filesystem type, SSD detection
  - Network: All interfaces with speed, MAC address, traffic statistics
  - System: OS info, hostname, uptime, process count

- **Resource Management**:
  - `ResourceAllocation`: Tracks current allocation status for all resources
  - `SharingCapabilities`: Determines safe sharing limits based on hardware
  - `SharingMode`: Configures Private/Federated/Public resource sharing modes

### 2. API Endpoints
All endpoints are available under `/api/v1/system/`:

- **GET `/api/v1/system/hardware`** - Complete hardware capabilities
- **GET `/api/v1/system/network`** - Network interface details
- **GET `/api/v1/system/allocation`** - Current resource allocation status
- **GET `/api/v1/system/capabilities`** - Sharing capabilities and limits
- **POST `/api/v1/system/refresh`** - Force refresh hardware detection

### 3. Transport Layer Integration
- `src/transport/hardware_handler.rs`: HTTP route handler for hardware API
- Integrated with STOQ transport layer for certificate-authenticated access
- Automatic registration of endpoints on server startup

### 4. Frontend Integration

#### API Client (`ui/frontend/lib/api/hardware.ts`)
- TypeScript interfaces for all hardware data types
- `HardwareApi` class with methods for all endpoints
- Helper functions for formatting bytes and bandwidth

#### React Hooks (`ui/frontend/lib/hooks/useHardware.ts`)
- `useHardware()`: Main hook for hardware capabilities with auto-refresh
- `useResourceMonitor()`: Real-time resource monitoring
- `useSharingCapabilities()`: Sharing configuration data
- Formatted data for UI display

#### UI Components Updated
- **DashboardHome**: Shows real CPU cores, RAM, storage from system
- **HyperMeshModule**: Uses actual hardware data for resource management
- Dynamic color coding based on actual usage (green/yellow/red)
- Real-time usage percentages instead of static values

## Key Features

### Real-Time Detection
- Automatic refresh every 5 seconds for dashboard
- 2-second interval for resource monitoring
- Cached results to prevent excessive system calls

### Smart Defaults
- Graceful fallback when hardware detection unavailable
- Recommended sharing: 50% of total resources
- Maximum sharing: 100% for public mode

### Performance Optimizations
- Caching with 5-second TTL
- Batched API requests in frontend
- Efficient system information gathering

## Removed Hardcoded Values

### Before (Hardcoded):
```javascript
const totalResources = { cpu: 8, ram: 32, storage: 1000 };
const sharedResources = {
  cpu: Math.floor(totalResources.cpu * 0.5),
  ram: Math.floor(totalResources.ram * 0.5),
  storage: Math.floor(totalResources.storage * 0.5)
};
```

### After (Real Detection):
```javascript
const totalResources = capabilities ? {
  cpu: capabilities.cpu.logical_cores,
  ram: Math.round(capabilities.memory.total_bytes / (1024 * 1024 * 1024)),
  storage: Math.round(
    capabilities.storage.reduce((sum, disk) => sum + disk.total_bytes, 0) / (1024 * 1024 * 1024)
  )
} : { cpu: 8, ram: 32, storage: 1000 }; // Fallback only
```

## Testing

Use the provided test script:
```bash
./test_hardware_api.sh
```

This will test all hardware detection endpoints and display the real system information being returned.

## Dependencies

Added to `Cargo.toml`:
- `sysinfo = "0.30"` - Cross-platform system information library

## Integration Points

1. **Server Initialization** (`src/main.rs`):
   - Hardware service created before dashboard handler
   - Registered with transport layer for HTTP access

2. **Dashboard Handler** (`src/dashboard.rs`):
   - Added `HardwareDetection` message types
   - Handler methods for STOQ protocol access

3. **Monitoring System** (`src/monitoring.rs`):
   - Can now track real resource utilization
   - Performance metrics based on actual hardware

## Future Enhancements

1. **Platform-Specific Detection**:
   - GPU detection and monitoring
   - Temperature sensors
   - Power consumption metrics

2. **Advanced Metrics**:
   - Network latency testing
   - Disk I/O performance
   - Memory bandwidth measurement

3. **Resource Prediction**:
   - ML-based usage prediction
   - Optimal allocation recommendations
   - Anomaly detection

## Summary

The hardware detection implementation successfully replaces all hardcoded resource values with real system information. The UI now displays:
- Actual CPU cores and usage
- Real memory capacity and utilization
- Detected storage devices and space
- Network interfaces and bandwidth
- Dynamic resource allocation based on actual hardware

This provides users with accurate information about their system capabilities and enables informed decisions about resource sharing in the HyperMesh network.