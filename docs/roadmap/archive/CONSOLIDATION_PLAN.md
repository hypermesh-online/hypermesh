# Code Consolidation Plan

## Phase 1: Certificate Management Consolidation

### Current State (4 implementations):
1. **stoq/src/transport/certificates.rs** - Full implementation (643 lines)
   - TrustChain client integration
   - Certificate rotation
   - Consensus proof validation
   - Full functionality

2. **trustchain/src/ca/certificate_manager.rs** - Placeholder (21 lines)
   - Empty stub implementation

3. **trustchain/src/ca/certificate_authority.rs** - Partial (CertificateRotationManager)
   - Different purpose (CA operations)

4. **hypermesh/core/transport/src/certificate.rs** - Separate implementation (312 lines)
   - Self-signed cert generation
   - rustls integration
   - Basic rotation

### Consolidation Strategy:
- **PRIMARY**: Use stoq/src/transport/certificates.rs as authoritative implementation
- **ACTION**: Redirect trustchain stub to use stoq implementation
- **ACTION**: Keep hypermesh separate (different transport layer needs)
- **ACTION**: Keep CA's CertificateRotationManager (different purpose)

## Phase 2: Handler Consolidation

### Current State:
1. **stoq/src/server.rs**:
   - EchoMessageHandler (lines 238-250)
   - JsonMessageHandler (lines 253-273)

2. **stoq/examples/integrated_echo_server.rs**:
   - IntegratedEchoHandler (lines 21-43)
   - JsonHandler (lines 46-60)

### Consolidation Strategy:
- **PRIMARY**: Keep server.rs implementations as examples
- **ACTION**: Remove duplicate handlers from examples
- **ACTION**: Import from server module instead

## Phase 3: Hardware Service Consolidation

### Current State:
- Hardware service has been properly implemented with real sysinfo API
- No duplicates found - already clean

## Phase 4: Transport Layer Consolidation

### Current State:
- STOQ transport is separate and complete
- HyperMesh transport is separate and complete
- No duplication - different purposes

## Phase 5: API Endpoint Review

### Areas to Check:
- HTTP gateway endpoints
- STOQ protocol handlers
- Certificate API routes
- Hardware monitoring endpoints

## Implementation Order:
1. Certificate Manager consolidation (highest impact)
2. Handler deduplication (code cleanup)
3. Import path updates
4. Test verification
5. Documentation updates