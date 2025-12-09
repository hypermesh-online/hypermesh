# Web3 Ecosystem API Specifications v1.0

## Gateway Architecture

### Unified API Gateway
**Port**: 8443 (HTTP/3 over QUIC)
**Purpose**: Single entry point routing requests to backend services

**Architecture**:
```
Client --> Gateway (8443) --> Service Router
                              |
                              ├──> HyperMesh/BlockMatrix (8446)
                              ├──> TrustChain (50053)
                              ├──> STOQ (Dynamic)
                              └──> Caesar (Dynamic)
```

### Gateway Configuration

```json
{
  "gateway": {
    "port": 8443,
    "protocol": "http3",
    "transport": "quic",
    "routes": {
      "/api/v1/hypermesh/*": "http://[::1]:8446",
      "/api/v1/trustchain/*": "http://[::1]:50053",
      "/api/v1/stoq/*": "http://[::1]:8446",
      "/api/v1/caesar/*": "http://[::1]:8446"
    },
    "cors": {
      "allowed_origins": ["*"],
      "allowed_methods": ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
      "allowed_headers": ["Content-Type", "X-Client-Certificate", "X-Request-ID"],
      "max_age": 3600
    },
    "middleware": {
      "request_logging": true,
      "request_id_injection": true,
      "response_time_tracking": true,
      "error_handling": true
    }
  }
}
```

### Gateway Health Check

## Endpoint: GET /health

**Priority**: P0
**Service**: Gateway
**Purpose**: Verify gateway is operational and can route to backend services

**Request:**
- Method: GET
- Path: /health
- Headers:
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body: None

**Response:**
- Status Codes:
  - 200: Gateway operational
  - 503: Gateway degraded (some services unavailable)
- Body Schema:
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "timestamp": "2025-12-09T12:00:00Z",
    "uptime_seconds": 3600,
    "version": "1.0.0",
    "services": {
      "hypermesh": "healthy",
      "trustchain": "healthy",
      "stoq": "healthy",
      "caesar": "degraded"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- None (health check endpoint)

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "GATEWAY_UNHEALTHY",
    "message": "Gateway is not operational",
    "details": {
      "failed_services": ["hypermesh", "trustchain"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Check connectivity to all backend services
- Cache results for 5 seconds to prevent overload
- Include version information from build metadata

---

## Week 1 Endpoints (Priority 0 and 1)

### 1. HyperMesh System Status

## Endpoint: GET /api/v1/hypermesh/system/status

**Priority**: P0
**Service**: HyperMesh
**Purpose**: Get comprehensive system status including matrix topology, blockchain state, and resource availability

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/system/status
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - include_matrix: boolean (optional, default: true) - Include matrix coordinate information
  - include_blockchain: boolean (optional, default: true) - Include blockchain state
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "node_id": "node-001",
    "status": "operational",
    "matrix_position": {
      "x": 10.5,
      "y": 20.3,
      "z": 0.0,
      "octant": 1,
      "neighbors": 8
    },
    "blockchain": {
      "height": 15234,
      "hash": "0x3f4a5b6c...",
      "pending_transactions": 12,
      "consensus_state": "synchronized"
    },
    "resources": {
      "cpu": {
        "cores": 16,
        "usage_percent": 45.2,
        "allocated_percent": 60.0
      },
      "memory": {
        "total_gb": 64,
        "used_gb": 28.5,
        "allocated_gb": 40.0
      },
      "gpu": {
        "available": true,
        "count": 2,
        "allocated": 1
      },
      "storage": {
        "total_tb": 2.0,
        "used_tb": 0.8,
        "allocated_tb": 1.2
      }
    },
    "network": {
      "connections": 42,
      "bandwidth_mbps": 950,
      "latency_ms": 12
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- Query parameters must be valid booleans if provided

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "SYSTEM_STATUS_ERROR",
    "message": "Failed to retrieve system status",
    "details": {
      "subsystem": "blockchain",
      "reason": "Blockchain service unavailable"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Pull from HyperMeshSystem instance
- Matrix position from matrix::coordinates module
- Blockchain state from blockchain module
- Resource metrics from os_integration module
- Cache results for 10 seconds

---

### 2. List Assets

## Endpoint: GET /api/v1/hypermesh/assets

**Priority**: P0
**Service**: HyperMesh
**Purpose**: List all registered assets with their current state and allocations

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/assets
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - asset_type: string (optional) - Filter by type: cpu, gpu, memory, storage
  - status: string (optional) - Filter by status: available, allocated, maintenance
  - limit: integer (optional, default: 100, max: 1000)
  - offset: integer (optional, default: 0)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 245,
    "offset": 0,
    "limit": 100,
    "assets": [
      {
        "asset_id": "asset-cpu-001",
        "asset_type": "cpu",
        "status": "allocated",
        "blockchain_registered": true,
        "registration_block": 15100,
        "properties": {
          "cores": 8,
          "frequency_ghz": 3.6,
          "architecture": "x86_64"
        },
        "allocation": {
          "allocated_to": "user-xyz",
          "allocation_percent": 75,
          "privacy_level": "private_network",
          "consensus_proofs": ["PoSpace", "PoStake", "PoWork", "PoTime"]
        },
        "metrics": {
          "usage_percent": 62.5,
          "temperature_celsius": 72,
          "power_watts": 85
        }
      },
      {
        "asset_id": "asset-gpu-001",
        "asset_type": "gpu",
        "status": "available",
        "blockchain_registered": true,
        "registration_block": 15102,
        "properties": {
          "model": "RTX 4090",
          "memory_gb": 24,
          "cuda_cores": 16384
        },
        "allocation": null,
        "metrics": {
          "usage_percent": 0,
          "temperature_celsius": 45,
          "power_watts": 50
        }
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- asset_type must be one of: cpu, gpu, memory, storage
- status must be one of: available, allocated, maintenance
- limit must be between 1 and 1000
- offset must be >= 0

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_ASSET_TYPE",
    "message": "Invalid asset type specified",
    "details": {
      "provided": "network",
      "valid_types": ["cpu", "gpu", "memory", "storage"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query AssetManager from assets module
- Include blockchain registration status
- Filter and paginate in memory (initial implementation)
- Future: Add database backing for large deployments

---

### 3. STOQ System Health

## Endpoint: GET /api/v1/stoq/system/health

**Priority**: P0
**Service**: STOQ
**Purpose**: Get STOQ protocol health including transport statistics and connection pool status

**Request:**
- Method: GET
- Path: /api/v1/stoq/system/health
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "protocol_version": "1.0.0",
    "status": "healthy",
    "uptime_seconds": 86400,
    "transport": {
      "type": "quic",
      "ipv6_enabled": true,
      "port": 8446
    },
    "connections": {
      "active": 142,
      "idle": 58,
      "max_capacity": 10000,
      "total_established": 5234
    },
    "performance": {
      "avg_latency_ms": 8.5,
      "throughput_mbps": 2950,
      "packet_loss_percent": 0.02
    },
    "pools": {
      "tier1_hot": {
        "size": 20,
        "active": 18,
          "idle": 2
      },
      "tier2_warm": {
        "size": 50,
        "active": 35,
        "idle": 15
      },
      "tier3_elastic": {
        "size": 100,
        "active": 89,
        "idle": 11
      }
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- None (health check endpoint)

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "STOQ_UNHEALTHY",
    "message": "STOQ protocol is experiencing issues",
    "details": {
      "subsystem": "connection_pool",
      "error": "Pool exhausted"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query StoqTransport instance
- Aggregate pool statistics
- Calculate performance metrics over 1-minute window
- Cache for 5 seconds

---

### 4. TrustChain Certificate Request

## Endpoint: POST /api/v1/trustchain/auth/certificate

**Priority**: P0
**Service**: TrustChain
**Purpose**: Request a new certificate for authentication in the ecosystem

**Request:**
- Method: POST
- Path: /api/v1/trustchain/auth/certificate
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body Schema:
```json
{
  "subject": {
    "common_name": "node-001.hypermesh.local",
    "organization": "HyperMesh Network",
    "country": "US"
  },
  "public_key": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0B...\n-----END PUBLIC KEY-----",
  "certificate_type": "node",
  "validity_days": 365,
  "extensions": {
    "san": ["node-001.local", "10.0.0.1"],
    "key_usage": ["digital_signature", "key_encipherment"]
  }
}
```

**Response:**
- Status Codes:
  - 200: Certificate issued successfully
  - 400: Invalid request (bad public key, invalid subject)
  - 401: Unauthorized (missing or invalid authentication)
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "certificate": "-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJ...\n-----END CERTIFICATE-----",
    "certificate_id": "cert-550e8400-e29b-41d4",
    "serial_number": "1234567890",
    "issuer": "CN=TrustChain Root CA",
    "subject": "CN=node-001.hypermesh.local",
    "not_before": "2025-12-09T12:00:00Z",
    "not_after": "2026-12-09T12:00:00Z",
    "fingerprint": {
      "sha256": "3f:4a:5b:6c:7d:8e:9f:a0:b1:c2:d3:e4:f5:06:17:28"
    },
    "chain": [
      "-----BEGIN CERTIFICATE-----\nIntermediate CA...\n-----END CERTIFICATE-----",
      "-----BEGIN CERTIFICATE-----\nRoot CA...\n-----END CERTIFICATE-----"
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- public_key must be valid PEM-encoded public key
- certificate_type must be one of: node, service, user
- validity_days must be between 1 and 3650
- subject.common_name is required and must be valid DNS name
- extensions.key_usage must contain valid key usage flags

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_PUBLIC_KEY",
    "message": "The provided public key is not valid",
    "details": {
      "error": "Failed to parse PEM encoding"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Validate public key using rustls
- Generate certificate with rcgen
- Store in TrustChain certificate store
- Return full certificate chain for client validation
- Log all certificate issuance for audit

---

### 5. List Resource Allocations

## Endpoint: GET /api/v1/hypermesh/allocations

**Priority**: P1
**Service**: HyperMesh
**Purpose**: List all active resource allocations with privacy levels and consensus proofs

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/allocations
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - user_id: string (optional) - Filter by user
  - asset_type: string (optional) - Filter by asset type
  - privacy_level: string (optional) - Filter by privacy level
  - limit: integer (optional, default: 100, max: 1000)
  - offset: integer (optional, default: 0)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 87,
    "offset": 0,
    "limit": 100,
    "allocations": [
      {
        "allocation_id": "alloc-001",
        "user_id": "user-xyz",
        "asset_id": "asset-cpu-001",
        "asset_type": "cpu",
        "allocation_percent": 75,
        "privacy_level": "private_network",
        "consensus_proofs": {
          "PoSpace": {
            "verified": true,
            "location": "rack-42-server-3",
            "commitment": "0x1234..."
          },
          "PoStake": {
            "verified": true,
            "stake_amount": 1000,
            "locked_until": "2025-12-31T23:59:59Z"
          },
          "PoWork": {
            "verified": true,
            "difficulty": 65536,
            "hash": "0xabcd..."
          },
          "PoTime": {
            "verified": true,
            "timestamp": "2025-12-09T10:00:00Z",
            "sequence": 98765
          }
        },
        "created_at": "2025-12-09T10:00:00Z",
        "expires_at": "2025-12-10T10:00:00Z",
        "status": "active",
        "metrics": {
          "usage_percent": 62.5,
          "data_processed_gb": 145.2,
          "uptime_percent": 99.9
        }
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- privacy_level must be one of: private, private_network, p2p, public_network, full_public
- asset_type must be one of: cpu, gpu, memory, storage
- limit must be between 1 and 1000
- offset must be >= 0

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_PRIVACY_LEVEL",
    "message": "Invalid privacy level specified",
    "details": {
      "provided": "semi-public",
      "valid_levels": ["private", "private_network", "p2p", "public_network", "full_public"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query AllocationManager from assets module
- Include all four consensus proofs
- Calculate real-time usage metrics
- Filter by privacy level using privacy module

---

### 6. Node Health Status

## Endpoint: GET /api/v1/hypermesh/nodes/health

**Priority**: P1
**Service**: HyperMesh
**Purpose**: Get health status of all nodes in the matrix topology

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/nodes/health
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - octant: integer (optional, 1-8) - Filter by matrix octant
  - status: string (optional) - Filter by health status
  - limit: integer (optional, default: 100, max: 1000)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total_nodes": 42,
    "healthy_nodes": 40,
    "degraded_nodes": 2,
    "offline_nodes": 0,
    "nodes": [
      {
        "node_id": "node-001",
        "health": "healthy",
        "matrix_position": {
          "x": 10.5,
          "y": 20.3,
          "z": 0.0,
          "octant": 1
        },
        "metrics": {
          "cpu_usage": 45.2,
          "memory_usage": 62.8,
          "disk_usage": 35.0,
          "network_latency_ms": 12
        },
        "blockchain": {
          "height": 15234,
          "sync_status": "synchronized",
          "pending_txs": 3
        },
        "last_seen": "2025-12-09T11:59:50Z"
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- octant must be between 1 and 8 if provided
- status must be one of: healthy, degraded, offline
- limit must be between 1 and 1000

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_OCTANT",
    "message": "Invalid matrix octant specified",
    "details": {
      "provided": 9,
      "valid_range": "1-8"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query matrix module for node positions
- Aggregate health from multiple sources
- Consider node healthy if last_seen < 30 seconds
- Cache results for 10 seconds

---

### 7. STOQ Active Connections

## Endpoint: GET /api/v1/stoq/connections

**Priority**: P1
**Service**: STOQ
**Purpose**: List all active STOQ protocol connections with performance metrics

**Request:**
- Method: GET
- Path: /api/v1/stoq/connections
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - state: string (optional) - Filter by state: active, idle, closing
  - peer_id: string (optional) - Filter by peer
  - limit: integer (optional, default: 100, max: 1000)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 142,
    "connections": [
      {
        "connection_id": "conn-001",
        "peer_id": "peer-xyz",
        "peer_address": "[2001:db8::1]:8446",
        "state": "active",
        "protocol": "quic",
        "established_at": "2025-12-09T10:00:00Z",
        "duration_seconds": 7200,
        "streams": {
          "active": 5,
          "total_opened": 234
        },
        "metrics": {
          "bytes_sent": 104857600,
          "bytes_received": 52428800,
          "packets_sent": 10234,
          "packets_received": 5678,
          "retransmissions": 12,
          "rtt_ms": 8.5,
          "congestion_events": 2
        },
        "tls": {
          "version": "TLS 1.3",
          "cipher": "TLS_AES_256_GCM_SHA384",
          "peer_certificate": "sha256:3f:4a:5b:6c..."
        }
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- state must be one of: active, idle, closing
- limit must be between 1 and 1000

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_CONNECTION_STATE",
    "message": "Invalid connection state filter",
    "details": {
      "provided": "connected",
      "valid_states": ["active", "idle", "closing"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query Quinn endpoint for connections
- Calculate real-time metrics
- Include TLS information for security audit
- Mask sensitive certificate details

---

### 8. STOQ Performance Metrics

## Endpoint: GET /api/v1/stoq/metrics/performance

**Priority**: P1
**Service**: STOQ
**Purpose**: Get detailed STOQ protocol performance metrics and statistics

**Request:**
- Method: GET
- Path: /api/v1/stoq/metrics/performance
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - period: string (optional, default: "1h") - Time period: 1m, 5m, 15m, 1h, 24h
  - metrics: string (optional) - Comma-separated list of metrics to include
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "period": "1h",
    "start_time": "2025-12-09T11:00:00Z",
    "end_time": "2025-12-09T12:00:00Z",
    "throughput": {
      "avg_mbps": 2950,
      "peak_mbps": 3200,
      "min_mbps": 2800,
      "percentile_95_mbps": 3100
    },
    "latency": {
      "avg_ms": 8.5,
      "min_ms": 2.1,
      "max_ms": 45.2,
      "percentile_50_ms": 7.8,
      "percentile_95_ms": 15.2,
      "percentile_99_ms": 28.5
    },
    "connections": {
      "total_established": 5234,
      "total_failed": 12,
      "avg_duration_seconds": 3456,
      "concurrent_peak": 189
    },
    "packets": {
      "total_sent": 10234567,
      "total_received": 9876543,
      "loss_percent": 0.02,
      "retransmission_percent": 0.15
    },
    "errors": {
      "connection_timeouts": 3,
      "handshake_failures": 2,
      "protocol_errors": 0,
      "certificate_errors": 1
    },
    "adaptive_tiers": {
      "tier1_connections": 18,
      "tier2_connections": 35,
      "tier3_connections": 89,
      "tier_promotions": 45,
      "tier_demotions": 12
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- period must be one of: 1m, 5m, 15m, 1h, 24h
- metrics if provided must be valid metric names

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_TIME_PERIOD",
    "message": "Invalid time period specified",
    "details": {
      "provided": "1w",
      "valid_periods": ["1m", "5m", "15m", "1h", "24h"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Aggregate from metrics collector
- Calculate percentiles using histogram data
- Include adaptive tier statistics
- Cache results for 30 seconds per period

---

### 9. Byzantine Fault Detections

## Endpoint: GET /api/v1/hypermesh/byzantine/detections

**Priority**: P1
**Service**: HyperMesh
**Purpose**: List detected Byzantine faults and consensus violations

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/byzantine/detections
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - severity: string (optional) - Filter by severity: low, medium, high, critical
  - status: string (optional) - Filter by status: active, resolved, investigating
  - start_time: ISO8601 (optional) - Start of time range
  - end_time: ISO8601 (optional) - End of time range
  - limit: integer (optional, default: 100, max: 1000)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 3,
    "detections": [
      {
        "detection_id": "byz-001",
        "detected_at": "2025-12-09T11:45:00Z",
        "severity": "high",
        "status": "investigating",
        "fault_type": "double_spend_attempt",
        "node_id": "node-suspicious-001",
        "details": {
          "transaction_ids": ["tx-123", "tx-456"],
          "conflicting_blocks": [15233, 15234],
          "detection_method": "consensus_validation",
          "confidence": 0.95
        },
        "consensus_impact": {
          "affected_nodes": 5,
          "consensus_disrupted": false,
          "fork_detected": false
        },
        "actions_taken": [
          {
            "action": "node_isolated",
            "timestamp": "2025-12-09T11:45:30Z",
            "result": "success"
          },
          {
            "action": "alert_sent",
            "timestamp": "2025-12-09T11:45:31Z",
            "result": "success"
          }
        ],
        "resolution": null
      }
    ],
    "summary": {
      "active_high_severity": 1,
      "active_medium_severity": 1,
      "active_low_severity": 1,
      "resolved_last_24h": 7
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- severity must be one of: low, medium, high, critical
- status must be one of: active, resolved, investigating
- time range must be valid ISO8601 timestamps
- limit must be between 1 and 1000

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_SEVERITY",
    "message": "Invalid severity level specified",
    "details": {
      "provided": "extreme",
      "valid_levels": ["low", "medium", "high", "critical"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query Byzantine detector from consensus module
- Include automatic mitigation actions
- Store detection history for audit
- Alert on high/critical severity

---

## Week 2 Endpoints (Priority 2)

### 10. Create Asset

## Endpoint: POST /api/v1/hypermesh/assets

**Priority**: P2
**Service**: HyperMesh
**Purpose**: Register a new asset in the HyperMesh system and blockchain

**Request:**
- Method: POST
- Path: /api/v1/hypermesh/assets
- Headers:
  - Content-Type: application/json
  - X-Client-Certificate: [required for production]
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body Schema:
```json
{
  "asset_type": "cpu",
  "properties": {
    "cores": 16,
    "frequency_ghz": 3.6,
    "architecture": "x86_64",
    "vendor": "Intel",
    "model": "Core i9-12900K"
  },
  "location": {
    "datacenter": "dc-west-1",
    "rack": "42",
    "server": "srv-003"
  },
  "capabilities": {
    "avx512": true,
    "sgx": true,
    "virtualization": true
  },
  "initial_allocation": {
    "allocated_percent": 0,
    "privacy_level": "private",
    "auto_allocate": false
  }
}
```

**Response:**
- Status Codes:
  - 201: Asset created successfully
  - 400: Invalid asset data
  - 401: Unauthorized
  - 409: Asset already exists
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "asset_id": "asset-cpu-002",
    "blockchain_transaction": {
      "tx_id": "0xabcdef1234567890",
      "block_height": 15235,
      "confirmation_time": "2025-12-09T12:00:15Z"
    },
    "asset": {
      "asset_id": "asset-cpu-002",
      "asset_type": "cpu",
      "status": "available",
      "properties": {
        "cores": 16,
        "frequency_ghz": 3.6,
        "architecture": "x86_64"
      },
      "created_at": "2025-12-09T12:00:00Z"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- asset_type must be one of: cpu, gpu, memory, storage
- properties must match schema for asset_type
- privacy_level must be valid privacy tier
- For CPU: cores > 0, frequency_ghz > 0
- For GPU: memory_gb > 0, cuda_cores > 0
- For Memory: size_gb > 0, speed_mhz > 0
- For Storage: size_tb > 0, iops > 0

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "ASSET_ALREADY_EXISTS",
    "message": "An asset with these properties already exists",
    "details": {
      "existing_asset_id": "asset-cpu-001",
      "conflict_field": "hardware_id"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Generate unique asset_id
- Register in blockchain before confirming
- Use AssetAdapter pattern for type-specific handling
- Emit event for asset creation
- Validate hardware properties against detected capabilities

---

### 11. Create Resource Allocation

## Endpoint: POST /api/v1/hypermesh/allocations

**Priority**: P2
**Service**: HyperMesh
**Purpose**: Create a new resource allocation with consensus proof validation

**Request:**
- Method: POST
- Path: /api/v1/hypermesh/allocations
- Headers:
  - Content-Type: application/json
  - X-Client-Certificate: [required]
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body Schema:
```json
{
  "asset_id": "asset-gpu-001",
  "allocation_percent": 50,
  "duration_hours": 24,
  "privacy_level": "private_network",
  "consensus_requirements": {
    "require_all_proofs": true,
    "minimum_stake": 1000,
    "geographic_restriction": "US-WEST"
  },
  "purpose": {
    "workload_type": "machine_learning",
    "description": "Training neural network model",
    "estimated_compute_hours": 20
  }
}
```

**Response:**
- Status Codes:
  - 201: Allocation created
  - 400: Invalid allocation request
  - 401: Unauthorized
  - 402: Insufficient stake/payment
  - 409: Resource not available
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "allocation_id": "alloc-002",
    "asset_id": "asset-gpu-001",
    "user_id": "user-xyz",
    "allocation_percent": 50,
    "privacy_level": "private_network",
    "consensus_validation": {
      "PoSpace": {
        "verified": true,
        "proof": "0x1234...",
        "timestamp": "2025-12-09T12:00:01Z"
      },
      "PoStake": {
        "verified": true,
        "stake_locked": 1000,
        "lock_tx": "0xabcd...",
        "timestamp": "2025-12-09T12:00:02Z"
      },
      "PoWork": {
        "verified": true,
        "work_proof": "0xefgh...",
        "timestamp": "2025-12-09T12:00:03Z"
      },
      "PoTime": {
        "verified": true,
        "time_proof": "0xijkl...",
        "timestamp": "2025-12-09T12:00:04Z"
      }
    },
    "created_at": "2025-12-09T12:00:00Z",
    "expires_at": "2025-12-10T12:00:00Z",
    "status": "active",
    "access_credentials": {
      "endpoint": "[2001:db8::42]:9000",
      "token": "eyJhbGciOiJIUzI1NiIs..."
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- asset_id must exist and be available
- allocation_percent must be 1-100
- duration_hours must be 1-8760 (max 1 year)
- privacy_level must be valid tier
- User must have sufficient stake if required
- All consensus proofs must validate if require_all_proofs

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INSUFFICIENT_STAKE",
    "message": "User does not have sufficient stake for this allocation",
    "details": {
      "required_stake": 1000,
      "user_stake": 500,
      "deficit": 500
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Validate all four consensus proofs
- Lock stake in escrow contract
- Generate access credentials
- Set up NAT-like proxy if needed
- Schedule automatic expiration

---

### 12. Validate Consensus

## Endpoint: POST /api/v1/hypermesh/consensus/validate

**Priority**: P2
**Service**: HyperMesh
**Purpose**: Validate consensus proofs for a transaction or operation

**Request:**
- Method: POST
- Path: /api/v1/hypermesh/consensus/validate
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body Schema:
```json
{
  "operation_type": "asset_transfer",
  "operation_data": {
    "from": "user-abc",
    "to": "user-xyz",
    "asset_id": "asset-gpu-001",
    "amount_percent": 25
  },
  "proofs": {
    "PoSpace": {
      "commitment": "0x1234567890abcdef",
      "merkle_proof": ["0xaaa", "0xbbb", "0xccc"],
      "location_proof": "rack-42-server-3"
    },
    "PoStake": {
      "stake_amount": 5000,
      "stake_proof": "0xfedcba0987654321",
      "signature": "0xsignature..."
    },
    "PoWork": {
      "nonce": 123456789,
      "difficulty": 65536,
      "hash": "0x00000000abcdef..."
    },
    "PoTime": {
      "timestamp": "2025-12-09T12:00:00Z",
      "sequence_number": 98766,
      "previous_hash": "0xprevious..."
    }
  }
}
```

**Response:**
- Status Codes:
  - 200: Validation complete
  - 400: Invalid proof format
  - 422: Validation failed
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "valid": true,
    "validation_results": {
      "PoSpace": {
        "valid": true,
        "confidence": 1.0,
        "verified_at": "2025-12-09T12:00:01Z"
      },
      "PoStake": {
        "valid": true,
        "confidence": 1.0,
        "stake_verified": true,
        "verified_at": "2025-12-09T12:00:02Z"
      },
      "PoWork": {
        "valid": true,
        "confidence": 1.0,
        "difficulty_met": true,
        "verified_at": "2025-12-09T12:00:03Z"
      },
      "PoTime": {
        "valid": true,
        "confidence": 1.0,
        "sequence_valid": true,
        "verified_at": "2025-12-09T12:00:04Z"
      }
    },
    "consensus_achieved": true,
    "validation_hash": "0xvalidation123...",
    "can_proceed": true
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:05Z"
}
```

**Validation Rules:**
- All proof fields must be present
- PoWork hash must meet difficulty target
- PoTime sequence must be monotonic
- PoStake signature must be valid
- PoSpace commitment must be verifiable

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "CONSENSUS_VALIDATION_FAILED",
    "message": "One or more consensus proofs failed validation",
    "details": {
      "failed_proofs": ["PoWork"],
      "PoWork_error": "Hash does not meet difficulty requirement"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Validate each proof independently
- Calculate aggregate confidence score
- Cache validation results for 60 seconds
- Log all validation attempts for audit

---

### 13. List Proxy Nodes

## Endpoint: GET /api/v1/hypermesh/proxy/list

**Priority**: P2
**Service**: HyperMesh
**Purpose**: List available proxy nodes for NAT-like memory addressing

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/proxy/list
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - region: string (optional) - Geographic region filter
  - privacy_tier: string (optional) - Required privacy tier
  - min_bandwidth_mbps: integer (optional) - Minimum bandwidth
  - limit: integer (optional, default: 50, max: 200)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 23,
    "proxies": [
      {
        "proxy_id": "proxy-west-001",
        "node_id": "node-042",
        "region": "us-west",
        "address": "[2001:db8::100]:8000",
        "capabilities": {
          "nat_memory": true,
          "nat_storage": true,
          "encryption": ["AES-256-GCM", "Kyber-1024"],
          "bandwidth_mbps": 10000,
          "latency_ms": 5
        },
        "privacy_tiers": ["private_network", "p2p", "federated"],
        "trust_score": 0.98,
        "availability": {
          "uptime_percent": 99.95,
          "current_load": 0.45,
          "max_connections": 1000,
          "active_connections": 450
        },
        "cost": {
          "per_gb": 0.01,
          "per_hour": 0.10,
          "currency": "CAESAR"
        }
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- region must be valid region code if provided
- privacy_tier must be valid tier if provided
- min_bandwidth_mbps must be positive if provided
- limit must be between 1 and 200

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_REGION",
    "message": "Invalid region specified",
    "details": {
      "provided": "mars-1",
      "valid_regions": ["us-west", "us-east", "eu-west", "ap-south"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query proxy registry from assets/proxy module
- Filter by trust score and availability
- Sort by trust score and latency
- Include cost information for planning

---

### 14. Caesar Wallet Information

## Endpoint: GET /api/v1/caesar/wallet

**Priority**: P2
**Service**: Caesar
**Purpose**: Get wallet information including balance and staking status

**Request:**
- Method: GET
- Path: /api/v1/caesar/wallet
- Headers:
  - Content-Type: application/json
  - X-Client-Certificate: [required]
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - include_transactions: boolean (optional, default: false)
  - include_stakes: boolean (optional, default: true)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 401: Unauthorized
  - 404: Wallet not found
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
    "balance": {
      "available": "10000.50",
      "locked": "5000.00",
      "pending": "250.00",
      "total": "15250.50",
      "currency": "CAESAR"
    },
    "staking": {
      "total_staked": "5000.00",
      "stakes": [
        {
          "stake_id": "stake-001",
          "amount": "3000.00",
          "locked_until": "2025-12-31T23:59:59Z",
          "purpose": "node_operation",
          "rewards_earned": "150.00"
        },
        {
          "stake_id": "stake-002",
          "amount": "2000.00",
          "locked_until": "2025-12-15T23:59:59Z",
          "purpose": "resource_allocation",
          "rewards_earned": "75.00"
        }
      ]
    },
    "reputation": {
      "score": 0.95,
      "tier": "gold",
      "violations": 0,
      "successful_allocations": 234
    },
    "created_at": "2025-01-01T00:00:00Z",
    "last_activity": "2025-12-09T11:55:00Z"
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- Must have valid client certificate
- Certificate must match wallet owner

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "WALLET_NOT_FOUND",
    "message": "No wallet associated with this certificate",
    "details": {
      "certificate_id": "cert-xyz"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Derive wallet from client certificate
- Query Caesar economic system
- Include staking information
- Calculate reputation score

---

### 15. Caesar Transactions

## Endpoint: GET /api/v1/caesar/transactions

**Priority**: P2
**Service**: Caesar
**Purpose**: List transaction history for the authenticated wallet

**Request:**
- Method: GET
- Path: /api/v1/caesar/transactions
- Headers:
  - Content-Type: application/json
  - X-Client-Certificate: [required]
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - type: string (optional) - Filter by type: payment, stake, reward, fee
  - start_date: ISO8601 (optional)
  - end_date: ISO8601 (optional)
  - limit: integer (optional, default: 100, max: 1000)
  - offset: integer (optional, default: 0)
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 401: Unauthorized
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 1234,
    "offset": 0,
    "limit": 100,
    "transactions": [
      {
        "tx_id": "0xabc123...",
        "type": "payment",
        "status": "confirmed",
        "amount": "-100.00",
        "currency": "CAESAR",
        "from": "0x1234567890abcdef...",
        "to": "0xfedcba0987654321...",
        "description": "Resource allocation payment",
        "block_height": 15234,
        "confirmations": 6,
        "timestamp": "2025-12-09T10:30:00Z",
        "fee": "0.10",
        "metadata": {
          "allocation_id": "alloc-001",
          "resource_type": "gpu"
        }
      },
      {
        "tx_id": "0xdef456...",
        "type": "reward",
        "status": "confirmed",
        "amount": "+25.50",
        "currency": "CAESAR",
        "from": "system",
        "to": "0x1234567890abcdef...",
        "description": "Staking reward",
        "block_height": 15200,
        "confirmations": 40,
        "timestamp": "2025-12-09T08:00:00Z",
        "fee": "0.00",
        "metadata": {
          "stake_id": "stake-001",
          "reward_period": "daily"
        }
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- type must be one of: payment, stake, reward, fee
- Date range must be valid
- limit must be between 1 and 1000

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_DATE_RANGE",
    "message": "Invalid date range specified",
    "details": {
      "error": "end_date must be after start_date"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query transaction history from Caesar
- Filter by authenticated wallet
- Include confirmation count
- Support pagination for large histories

---

### 16. STOQ Connection Pools

## Endpoint: GET /api/v1/stoq/pools

**Priority**: P2
**Service**: STOQ
**Purpose**: Get detailed information about STOQ adaptive connection pools

**Request:**
- Method: GET
- Path: /api/v1/stoq/pools
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - tier: string (optional) - Filter by tier: tier1, tier2, tier3
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "tier1_hot": {
      "config": {
        "min_size": 10,
        "max_size": 50,
        "target_size": 20,
        "idle_timeout_ms": 5000
      },
      "status": {
        "current_size": 20,
        "active": 18,
        "idle": 2,
        "connecting": 0
      },
      "performance": {
        "avg_acquisition_ms": 0.5,
        "avg_rtt_ms": 2.1,
        "connections_created": 234,
        "connections_recycled": 1023
      }
    },
    "tier2_warm": {
      "config": {
        "min_size": 20,
        "max_size": 200,
        "target_size": 50,
        "idle_timeout_ms": 30000
      },
      "status": {
        "current_size": 50,
        "active": 35,
        "idle": 15,
        "connecting": 0
      },
      "performance": {
        "avg_acquisition_ms": 1.2,
        "avg_rtt_ms": 8.5,
        "connections_created": 567,
        "connections_recycled": 3456
      }
    },
    "tier3_elastic": {
      "config": {
        "min_size": 0,
        "max_size": 1000,
        "target_size": 100,
        "idle_timeout_ms": 60000
      },
      "status": {
        "current_size": 100,
        "active": 89,
        "idle": 11,
        "connecting": 0
      },
      "performance": {
        "avg_acquisition_ms": 5.5,
        "avg_rtt_ms": 15.2,
        "connections_created": 1234,
        "connections_recycled": 8901
      }
    },
    "adaptive_metrics": {
      "promotions_last_hour": 45,
      "demotions_last_hour": 12,
      "auto_scaling_events": 7,
      "optimization_score": 0.92
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- tier must be one of: tier1, tier2, tier3 if provided

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_POOL_TIER",
    "message": "Invalid pool tier specified",
    "details": {
      "provided": "tier4",
      "valid_tiers": ["tier1", "tier2", "tier3"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query adaptive pool manager
- Calculate real-time metrics
- Include configuration for transparency
- Show optimization score

---

### 17. Network Topology

## Endpoint: GET /api/v1/hypermesh/network/topology

**Priority**: P2
**Service**: HyperMesh
**Purpose**: Get the matrix network topology showing node positions and connections

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/network/topology
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - octant: integer (optional) - Filter by octant (1-8)
  - depth: integer (optional, default: 2, max: 5) - Neighbor depth
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total_nodes": 42,
    "octant_distribution": {
      "1": 6,
      "2": 5,
      "3": 6,
      "4": 5,
      "5": 5,
      "6": 5,
      "7": 5,
      "8": 5
    },
    "nodes": [
      {
        "node_id": "node-001",
        "position": {
          "x": 10.5,
          "y": 20.3,
          "z": 0.0,
          "octant": 1
        },
        "neighbors": [
          {
            "node_id": "node-002",
            "distance": 5.2,
            "latency_ms": 3.1,
            "connection_quality": 0.98
          },
          {
            "node_id": "node-003",
            "distance": 8.7,
            "latency_ms": 5.5,
            "connection_quality": 0.95
          }
        ],
        "role": "validator",
        "resources": {
          "cpu_available": true,
          "gpu_available": false,
          "storage_tb": 2.0
        }
      }
    ],
    "edges": [
      {
        "from": "node-001",
        "to": "node-002",
        "weight": 0.98,
        "type": "direct",
        "bandwidth_mbps": 1000
      }
    ],
    "clustering": {
      "algorithm": "golden_ratio",
      "clusters": 7,
      "avg_cluster_size": 6
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- octant must be 1-8 if provided
- depth must be 1-5 if provided

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_DEPTH",
    "message": "Invalid topology depth specified",
    "details": {
      "provided": 10,
      "max_depth": 5
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query matrix coordinate system
- Calculate neighbor relationships
- Include edge weights for visualization
- Apply golden ratio clustering

---

### 18. VM Execute

## Endpoint: POST /api/v1/hypermesh/vm/execute

**Priority**: P2
**Service**: HyperMesh/Catalog
**Purpose**: Execute code in a secure VM with resource constraints

**Request:**
- Method: POST
- Path: /api/v1/hypermesh/vm/execute
- Headers:
  - Content-Type: application/json
  - X-Client-Certificate: [required]
  - X-Request-ID: [UUID] (optional)
- Query Parameters: None
- Body Schema:
```json
{
  "language": "julia",
  "code": "function compute(x)\n  return x^2 + 2*x + 1\nend\ncompute(5)",
  "resources": {
    "cpu_cores": 2,
    "memory_mb": 1024,
    "timeout_seconds": 30,
    "gpu_required": false
  },
  "inputs": {
    "data": [1, 2, 3, 4, 5]
  },
  "output_format": "json"
}
```

**Response:**
- Status Codes:
  - 200: Execution successful
  - 400: Invalid code or parameters
  - 401: Unauthorized
  - 402: Insufficient resources
  - 408: Execution timeout
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "execution_id": "exec-001",
    "status": "completed",
    "result": {
      "output": 36,
      "type": "number"
    },
    "execution_time_ms": 125,
    "resources_used": {
      "cpu_time_ms": 100,
      "memory_peak_mb": 256,
      "gpu_time_ms": 0
    },
    "consensus_validation": {
      "validated": true,
      "validators": 3,
      "consensus_hash": "0xabc123..."
    },
    "cost": {
      "amount": "0.05",
      "currency": "CAESAR"
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- language must be supported (initially: julia)
- code must not exceed 100KB
- timeout_seconds must be 1-300
- memory_mb must be 128-8192
- cpu_cores must be 1-16

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "EXECUTION_TIMEOUT",
    "message": "Code execution exceeded timeout",
    "details": {
      "timeout_seconds": 30,
      "partial_output": null
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Use Catalog VM integration
- Sandbox execution environment
- Validate consensus for deterministic execution
- Track resource usage for billing
- Kill on timeout

---

### 19. Catalog Applications

## Endpoint: GET /api/v1/hypermesh/catalog/applications

**Priority**: P2
**Service**: HyperMesh/Catalog
**Purpose**: List available applications and VM images in the catalog

**Request:**
- Method: GET
- Path: /api/v1/hypermesh/catalog/applications
- Headers:
  - Content-Type: application/json
  - X-Request-ID: [UUID] (optional)
- Query Parameters:
  - category: string (optional) - Filter by category: ml, data, web, blockchain
  - language: string (optional) - Filter by language: julia, python, rust
  - limit: integer (optional, default: 50, max: 200)
  - search: string (optional) - Search in name/description
- Body: None

**Response:**
- Status Codes:
  - 200: Success
  - 400: Invalid query parameters
  - 500: Internal server error
- Body Schema:
```json
{
  "success": true,
  "data": {
    "total": 127,
    "applications": [
      {
        "app_id": "app-ml-transformer",
        "name": "Transformer Model Training",
        "version": "2.1.0",
        "category": "machine_learning",
        "language": "julia",
        "description": "Pre-configured transformer model training environment",
        "author": "hypermesh-community",
        "rating": 4.8,
        "downloads": 15234,
        "requirements": {
          "min_cpu_cores": 4,
          "min_memory_gb": 16,
          "gpu_required": true,
          "storage_gb": 50
        },
        "pricing": {
          "base_cost": "1.00",
          "per_hour": "0.50",
          "currency": "CAESAR"
        },
        "verification": {
          "signed": true,
          "signature": "0xsig123...",
          "verified_by": "trustchain"
        },
        "created_at": "2025-11-01T00:00:00Z",
        "updated_at": "2025-12-01T00:00:00Z"
      }
    ]
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Validation Rules:**
- category must be valid if provided
- language must be supported if provided
- limit must be between 1 and 200

**Error Responses:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_CATEGORY",
    "message": "Invalid application category",
    "details": {
      "provided": "games",
      "valid_categories": ["ml", "data", "web", "blockchain"]
    }
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-09T12:00:00Z"
}
```

**Implementation Notes:**
- Query Catalog registry
- Include verification status
- Sort by popularity/rating
- Cache for 5 minutes

---

## Authentication Specification

### Certificate-Based Authentication

**Primary Method**: X.509 Client Certificates issued by TrustChain

**Flow**:
1. Client requests certificate from `/api/v1/trustchain/auth/certificate`
2. Client includes certificate in `X-Client-Certificate` header
3. Gateway validates certificate against TrustChain
4. Gateway extracts identity and passes to backend services

**Certificate Format**:
```
-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJAKLNfehQVzC5MA0GCSqGSIb3DQEBCwUA...
[Base64 encoded certificate data]
-----END CERTIFICATE-----
```

### Development Mode Authentication

**Purpose**: Simplified authentication for development/testing

**Configuration**:
```json
{
  "auth": {
    "development_mode": true,
    "accept_self_signed": true,
    "default_user": "dev-user",
    "bypass_validation": true
  }
}
```

**Headers**:
- `X-Dev-Mode: true` - Bypass certificate validation
- `X-Dev-User: user-id` - Specify user for testing

### Token Format

**JWT Token Structure**:
```json
{
  "header": {
    "alg": "RS256",
    "typ": "JWT"
  },
  "payload": {
    "sub": "user-xyz",
    "iss": "trustchain",
    "aud": "hypermesh",
    "exp": 1702080000,
    "iat": 1701993600,
    "jti": "550e8400-e29b-41d4",
    "roles": ["user", "validator"],
    "permissions": ["read:assets", "write:allocations"]
  }
}
```

### Session Management

**Session Configuration**:
- Session timeout: 24 hours
- Refresh window: 1 hour before expiry
- Maximum sessions per user: 10
- Session store: Redis (future) / In-memory (current)

### Rate Limiting

**Default Limits**:
- Anonymous: 100 requests/minute
- Authenticated: 1000 requests/minute
- Premium: 10000 requests/minute

**Headers**:
- `X-RateLimit-Limit: 1000`
- `X-RateLimit-Remaining: 950`
- `X-RateLimit-Reset: 1702080000`

---

## Error Code Taxonomy

### 1xxx: Gateway Errors
- `1001: GATEWAY_UNREACHABLE` - Gateway service down
- `1002: GATEWAY_OVERLOADED` - Too many requests
- `1003: GATEWAY_TIMEOUT` - Request timeout at gateway
- `1004: INVALID_ROUTE` - No route to service
- `1005: SERVICE_UNAVAILABLE` - Backend service down

### 2xxx: Authentication/Authorization
- `2001: MISSING_CERTIFICATE` - No client certificate provided
- `2002: INVALID_CERTIFICATE` - Certificate validation failed
- `2003: EXPIRED_CERTIFICATE` - Certificate has expired
- `2004: UNAUTHORIZED` - No valid authentication
- `2005: FORBIDDEN` - Insufficient permissions
- `2006: INVALID_TOKEN` - JWT token invalid
- `2007: EXPIRED_TOKEN` - JWT token expired

### 3xxx: Validation Errors
- `3001: MISSING_REQUIRED_FIELD` - Required field not provided
- `3002: INVALID_FIELD_TYPE` - Wrong data type
- `3003: INVALID_FIELD_VALUE` - Value outside valid range
- `3004: INVALID_FORMAT` - Format validation failed
- `3005: FIELD_TOO_LONG` - Field exceeds maximum length
- `3006: INVALID_JSON` - Malformed JSON

### 4xxx: Business Logic
- `4001: RESOURCE_NOT_FOUND` - Requested resource doesn't exist
- `4002: RESOURCE_ALREADY_EXISTS` - Duplicate resource
- `4003: INSUFFICIENT_RESOURCES` - Not enough resources available
- `4004: INSUFFICIENT_FUNDS` - Not enough CAESAR tokens
- `4005: CONSENSUS_FAILED` - Consensus validation failed
- `4006: ALLOCATION_CONFLICT` - Resource already allocated
- `4007: INVALID_STATE` - Operation not allowed in current state

### 5xxx: Internal Errors
- `5001: INTERNAL_ERROR` - Generic internal error
- `5002: DATABASE_ERROR` - Database operation failed
- `5003: BLOCKCHAIN_ERROR` - Blockchain operation failed
- `5004: NETWORK_ERROR` - Network communication failed
- `5005: TIMEOUT` - Operation timeout
- `5006: PANIC` - System panic/crash

---

## Testing Specifications

### Unit Tests per Endpoint

**Template**:
```typescript
describe('GET /api/v1/hypermesh/system/status', () => {
  it('should return system status successfully', async () => {
    const response = await client.get('/api/v1/hypermesh/system/status');
    expect(response.status).toBe(200);
    expect(response.data.success).toBe(true);
    expect(response.data.data.node_id).toBeDefined();
  });

  it('should handle missing matrix data gracefully', async () => {
    const response = await client.get('/api/v1/hypermesh/system/status?include_matrix=true');
    expect(response.status).toBe(200);
    expect(response.data.data.matrix_position).toBeDefined();
  });

  it('should return 500 on internal error', async () => {
    // Mock internal error
    const response = await client.get('/api/v1/hypermesh/system/status');
    expect(response.status).toBe(500);
    expect(response.data.error.code).toBe('SYSTEM_STATUS_ERROR');
  });
});
```

### Performance Criteria

**All Endpoints**:
- P95 latency < 500ms
- P99 latency < 1000ms
- Throughput > 100 requests/second
- Error rate < 0.1%

**Critical Endpoints (P0)**:
- P95 latency < 100ms
- P99 latency < 200ms
- Throughput > 1000 requests/second

### Integration Tests

**Health Check Flow**:
```typescript
it('should verify all services are healthy', async () => {
  const gateway = await client.get('/health');
  expect(gateway.data.data.services.hypermesh).toBe('healthy');
  expect(gateway.data.data.services.trustchain).toBe('healthy');
  expect(gateway.data.data.services.stoq).toBe('healthy');
});
```

**Authentication Flow**:
```typescript
it('should complete certificate authentication flow', async () => {
  // 1. Request certificate
  const cert = await client.post('/api/v1/trustchain/auth/certificate', {...});

  // 2. Use certificate for authenticated request
  const assets = await client.get('/api/v1/hypermesh/assets', {
    headers: { 'X-Client-Certificate': cert.data.data.certificate }
  });

  expect(assets.status).toBe(200);
});
```

**Resource Allocation Flow**:
```typescript
it('should complete full allocation lifecycle', async () => {
  // 1. List available assets
  const assets = await client.get('/api/v1/hypermesh/assets');

  // 2. Create allocation
  const allocation = await client.post('/api/v1/hypermesh/allocations', {...});

  // 3. Verify allocation active
  const status = await client.get(`/api/v1/hypermesh/allocations/${allocation.data.data.allocation_id}`);

  expect(status.data.data.status).toBe('active');
});
```

---

## Implementation Priority

### Week 1 Implementation Order
1. Gateway setup with health check
2. HyperMesh system status (foundation)
3. TrustChain certificate endpoint (auth)
4. STOQ health (transport validation)
5. Asset listing (core functionality)
6. Remaining P1 endpoints

### Week 2 Implementation Order
1. Asset/Allocation creation (core operations)
2. Consensus validation (critical path)
3. Caesar wallet/transactions (economics)
4. VM execution (compute)
5. Network topology (visualization)
6. Remaining endpoints

---

## Notes

- All timestamps in ISO8601 format with timezone
- All responses include request_id for tracing
- Development mode bypasses certificate validation
- Production requires valid TrustChain certificates
- CORS enabled for browser-based clients
- IPv6 addresses in square brackets
- Binary data as base64 encoded strings
- Large responses support pagination
- Sensitive data masked in logs
- Metrics collected for all endpoints

---

**Document Version**: 1.0.0
**Last Updated**: 2025-12-09
**Total Endpoints**: 20 (10 Week 1, 10 Week 2)
**Total Lines**: ~1900