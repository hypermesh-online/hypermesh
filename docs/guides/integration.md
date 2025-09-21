# Web3 Ecosystem Integration Guide

## Overview
This guide covers integration patterns and best practices for connecting with the Web3 ecosystem components.

## Component Integration Matrix

| From | To | Protocol | Method |
|------|-----|----------|--------|
| Application | TrustChain | HTTPS/STOQ | REST API |
| HyperMesh | TrustChain | STOQ | Certificate validation |
| HyperMesh | Caesar | STOQ | Token rewards |
| STOQ | TrustChain | Direct | Certificate lifecycle |
| Caesar | HyperMesh | STOQ | Resource payments |
| NGauge | Caesar | STOQ | Engagement rewards |

## Integration Patterns

### 1. Service Discovery
All services use TrustChain DNS for discovery:
```javascript
// Resolve service endpoint
const endpoint = await trustchain.resolve('service.hypermesh');
// Returns: IPv6 address with STOQ endpoint
```

### 2. Authentication
Every connection requires TrustChain certificate:
```javascript
// Initialize with certificate
const client = new HyperMeshClient({
  certificate: await trustchain.getCertificate(),
  privateKey: privateKey
});
```

### 3. Byzantine Consensus
Multi-service operations require consensus:
```javascript
// Submit operation requiring consensus
const result = await hypermesh.submitWithConsensus({
  operation: 'allocate_resources',
  validators: ['node1', 'node2', 'node3'],
  threshold: 0.66 // 2/3 majority
});
```

## API Integration

### TrustChain API
Base URL: `https://trust.hypermesh.online/api/v1`

**Authentication**: Bearer token or client certificate

**Key Endpoints**:
- `POST /certificates/issue` - Issue certificate
- `GET /dns/resolve/{domain}` - Resolve domain
- `POST /ct/submit` - Submit to CT log

### HyperMesh API
Base URL: `stoq://hypermesh/api/v1`

**Authentication**: TrustChain certificate required

**Key Endpoints**:
- `POST /assets/register` - Register new asset
- `GET /assets/{id}` - Get asset details
- `POST /compute/allocate` - Allocate compute resources

### Caesar API
Base URL: `https://caesar.hypermesh.online/api/v1`

**Authentication**: Web3 wallet signature

**Key Endpoints**:
- `GET /wallet/balance` - Get CAES balance
- `POST /staking/deposit` - Stake tokens
- `POST /rewards/claim` - Claim mining rewards

### STOQ Protocol
Transport layer - not directly exposed via API

**Usage**:
```javascript
import { StoqClient } from '@hypermesh/stoq';

const client = new StoqClient({
  certificate: cert,
  endpoints: ['[2001:db8::1]:443']
});

await client.connect();
const stream = await client.openStream();
```

## Cross-Component Workflows

### Resource Allocation Flow
1. Client requests resources from HyperMesh
2. HyperMesh validates certificate with TrustChain
3. HyperMesh checks payment with Caesar
4. Resources allocated with consensus proof
5. Caesar distributes rewards to providers

### Certificate Lifecycle
1. Node requests certificate from TrustChain
2. TrustChain validates identity and issues cert
3. Certificate submitted to CT log
4. STOQ uses certificate for transport
5. Auto-renewal every 24 hours

### Byzantine Fault Recovery
1. Malicious behavior detected by monitor
2. Consensus excludes faulty node
3. TrustChain revokes certificate
4. Caesar slashes staked tokens
5. Network self-heals around failure

## SDK Libraries

### JavaScript/TypeScript
```bash
npm install @hypermesh/sdk
```

### Rust
```toml
[dependencies]
hypermesh-sdk = "1.0"
```

### Python
```bash
pip install hypermesh-sdk
```

## Testing Integration

### Local Development
```bash
# Start all services locally
docker-compose up -d

# Run integration tests
npm run test:integration
```

### Staging Environment
- Endpoint: `https://staging.hypermesh.online`
- Uses test certificates from TrustChain
- Test CAES tokens available from faucet

### Production
- Endpoint: `https://hypermesh.online`
- Requires valid TrustChain certificate
- Real CAES tokens required for operations

## Security Considerations

1. **Certificate Validation**: Always verify TrustChain certificates
2. **Byzantine Tolerance**: Assume 33% of nodes may be malicious
3. **Quantum Resistance**: Use post-quantum algorithms where available
4. **Rate Limiting**: Implement exponential backoff for retries
5. **Consensus Verification**: Validate consensus proofs independently

## Troubleshooting

### Common Issues

**Certificate Expired**
- Solution: Implement auto-renewal before 24-hour expiry

**Consensus Timeout**
- Solution: Increase validator count or adjust threshold

**STOQ Connection Failed**
- Solution: Verify IPv6 connectivity and firewall rules

**Insufficient CAES Balance**
- Solution: Top up wallet or claim pending rewards

## Support

- Documentation: https://docs.hypermesh.online
- Discord: https://discord.gg/hypermesh
- GitHub: https://github.com/hypermesh-online