# TODO Resolution Plan - Sprint 2.2

## Overview
**Total TODOs**: 26 comments
**Critical (Must Fix)**: 8 items
**Enhancement (Defer/Document)**: 18 items

---

## Day 5: Critical TODOs (Must Implement)

### Priority 1: DNS STOQ Listener (2 hours)
**Location**: `src/dns/mod.rs:292`
**Current**:
```rust
// TODO: Implement proper STOQ DNS service listener
```

**Impact**: Blocks DNS-over-STOQ functionality

**Implementation**:
```rust
/// Start STOQ-based DNS service listener
pub async fn start_stoq_listener(
    addr: SocketAddr,
    config: DnsConfig,
) -> Result<()> {
    let stoq_client = StoqClient::new(Default::default()).await
        .context("Failed to create STOQ client for DNS")?;

    info!("Starting DNS-over-STOQ listener on {}", addr);

    stoq_client.listen(addr, move |request, peer| {
        let config = config.clone();
        async move {
            handle_dns_request(request, peer, &config).await
        }
    }).await
    .context("STOQ DNS listener failed")?;

    Ok(())
}
```

**Verification**:
```bash
cargo test --lib dns::tests::test_stoq_listener
```

---

### Priority 2: PEM Certificate Parsing (1.5 hours)
**Location**: `src/api/stoq_api.rs:135`
**Current**:
```rust
// TODO: Implement proper PEM parsing
```

**Impact**: API cannot parse uploaded certificates

**Implementation**:
```rust
use x509_parser::prelude::*;
use pem::parse;

async fn parse_certificate_pem(pem_data: &str) -> Result<CertificateInfo> {
    // Parse PEM format
    let pem = parse(pem_data)
        .map_err(|e| anyhow!("Invalid PEM format: {}", e))?;

    // Parse X.509 certificate
    let (_, cert) = X509Certificate::from_der(&pem.contents)
        .map_err(|e| anyhow!("Invalid X.509 certificate: {}", e))?;

    // Extract certificate details
    let details = CertificateDetails {
        subject: cert.subject().to_string(),
        issuer: cert.issuer().to_string(),
        serial: cert.serial.to_string(),
        not_before: cert.validity().not_before.to_datetime(),
        not_after: cert.validity().not_after.to_datetime(),
        fingerprint: compute_fingerprint(&pem.contents)?,
    };

    Ok(CertificateInfo {
        pem: pem_data.to_string(),
        details: Some(details),
    })
}
```

**Dependencies**: Add to Cargo.toml:
```toml
x509-parser = "0.15"
pem = "3.0"
```

**Verification**:
```bash
cargo test --lib api::tests::test_certificate_parsing
```

---

### Priority 3: CSR Subject Extraction (1.5 hours)
**Location**: `src/api/stoq_api.rs:188,193`
**Current**:
```rust
// TODO: Parse CSR to extract subject info
common_name: "placeholder.trustchain.local".to_string(), // TODO: Extract from CSR
```

**Impact**: Certificate issuance uses wrong subject names

**Implementation**:
```rust
use openssl::x509::X509Req;
use openssl::nid::Nid;

async fn parse_csr(csr_pem: &str) -> Result<CertificateRequest> {
    let csr = X509Req::from_pem(csr_pem.as_bytes())
        .context("Failed to parse CSR PEM")?;

    // Extract common name
    let common_name = csr.subject_name()
        .entries_by_nid(Nid::COMMONNAME)
        .next()
        .ok_or_else(|| anyhow!("CSR missing Common Name"))?
        .data()
        .as_utf8()
        .context("Invalid UTF-8 in CN")?
        .to_string();

    // Extract SANs if present
    let sans = extract_sans_from_csr(&csr)?;

    Ok(CertificateRequest {
        common_name,
        subject_alt_names: sans,
        organization: extract_org(&csr)?,
        key_algorithm: detect_key_algorithm(&csr)?,
    })
}

fn extract_sans_from_csr(csr: &X509Req) -> Result<Vec<String>> {
    // Parse extensions for SAN
    // OpenSSL API for CSR extensions
    Ok(vec![]) // Simplified
}
```

**Dependencies**: Already have openssl

**Verification**:
```bash
cargo test --lib api::tests::test_csr_parsing
```

---

### Priority 4: CA-Signed Certificates (2 hours)
**Location**: `src/crypto/certificate.rs:279`, `src/ca/mod.rs:501`, `src/ca/certificate_authority.rs:272`
**Current**:
```rust
// rcgen 0.13: Create self-signed certificate (TODO: should be CA-signed)
```

**Impact**: Certificates not properly signed by CA

**Implementation**:
```rust
use rcgen::{Certificate, CertificateParams};

async fn sign_with_ca(
    params: CertificateParams,
    ca_cert: &Certificate,
) -> Result<String> {
    // Create certificate signed by CA
    let cert = Certificate::from_params(params)
        .context("Failed to create certificate params")?;

    // Sign with CA
    let cert_pem = cert.serialize_pem_with_signer(ca_cert)
        .context("Failed to sign certificate with CA")?;

    Ok(cert_pem)
}

// In CertificateAuthority
pub async fn issue_certificate(
    &self,
    request: CertificateRequest,
) -> Result<IssuedCertificate> {
    // Load CA certificate
    let ca_cert = self.load_ca_cert().await?;

    // Create params from request
    let params = self.create_cert_params(request)?;

    // Sign with CA
    let certificate_pem = sign_with_ca(params, &ca_cert).await?;

    // Build chain
    let chain_pem = self.build_certificate_chain(&certificate_pem).await?;

    Ok(IssuedCertificate {
        certificate_pem,
        chain_pem,
        fingerprint: compute_fingerprint(&certificate_pem)?,
    })
}
```

**Verification**:
```bash
cargo test --lib ca::tests::test_ca_signed_certificate
```

---

### Priority 5: DNS Record Parsing (1 hour)
**Location**: `src/dns/stoq_transport.rs:208,209,231`
**Current**:
```rust
domain: "example.com".to_string(), // TODO: Parse actual domain
query_type: 1, // A record - TODO: Parse actual type
// TODO: Serialize actual DNS records from STOQ response
```

**Impact**: DNS queries don't parse actual request data

**Implementation**:
```rust
use trust_dns_proto::rr::{DNSClass, RecordType};
use trust_dns_proto::op::Message;

fn parse_dns_query(request_data: &[u8]) -> Result<DnsQuery> {
    let message = Message::from_vec(request_data)
        .context("Failed to parse DNS message")?;

    let query = message.queries()
        .first()
        .ok_or_else(|| anyhow!("No query in DNS message"))?;

    Ok(DnsQuery {
        domain: query.name().to_utf8(),
        query_type: query.query_type() as u16,
        query_class: query.query_class() as u16,
        transaction_id: message.id(),
    })
}

fn serialize_dns_response(records: Vec<DnsRecord>) -> Result<Vec<u8>> {
    let mut message = Message::new();
    message.set_message_type(MessageType::Response);

    for record in records {
        let rdata = match record.record_type {
            RecordType::A => RData::A(record.ipv4_addr),
            RecordType::AAAA => RData::AAAA(record.ipv6_addr),
            RecordType::CNAME => RData::CNAME(Name::from_str(&record.cname)?),
            _ => return Err(anyhow!("Unsupported record type")),
        };

        let record = Record::from_rdata(
            Name::from_str(&record.name)?,
            record.ttl,
            rdata,
        );

        message.add_answer(record);
    }

    Ok(message.to_vec()?)
}
```

**Dependencies**: Already have trust-dns-proto

**Verification**:
```bash
cargo test --lib dns::tests::test_dns_message_parsing
```

---

### Priority 6: SAN Extraction from Certificate (1 hour)
**Location**: `src/crypto/certificate.rs:300`
**Current**:
```rust
san_entries: vec![], // TODO: Extract from certificate
```

**Impact**: Cannot display certificate SANs in API responses

**Implementation**:
```rust
use x509_parser::extensions::GeneralName;

fn extract_san_entries(cert: &X509Certificate) -> Result<Vec<String>> {
    let mut sans = Vec::new();

    if let Some(san_ext) = cert.get_extension_unique(&OID_X509_EXT_SUBJECT_ALT_NAME)? {
        if let ParsedExtension::SubjectAlternativeName(san) = san_ext.parsed_extension() {
            for name in &san.general_names {
                match name {
                    GeneralName::DNSName(dns) => {
                        sans.push(dns.to_string());
                    }
                    GeneralName::IPAddress(ip) => {
                        sans.push(format!("IP:{}", ip));
                    }
                    _ => {} // Skip other types
                }
            }
        }
    }

    Ok(sans)
}
```

**Verification**:
```bash
cargo test --lib crypto::tests::test_san_extraction
```

---

### Priority 7: Client Address Extraction (30 minutes)
**Location**: `src/api/stoq_api.rs:271`
**Current**:
```rust
client_addr: std::net::Ipv6Addr::LOCALHOST, // TODO: Get actual client address
```

**Impact**: Audit logs show wrong client addresses

**Implementation**:
```rust
// In STOQ handler context
async fn handle_api_request(
    request: StoqRequest,
    peer_addr: SocketAddr,
) -> Result<StoqResponse> {
    let client_addr = match peer_addr {
        SocketAddr::V6(addr) => *addr.ip(),
        SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
    };

    // Use client_addr in audit log
    log_api_request(LogEntry {
        client_addr,
        timestamp: Utc::now(),
        operation: request.operation.clone(),
    }).await?;

    // Process request
    process_request(request).await
}
```

**Verification**:
```bash
cargo test --lib api::tests::test_client_address_logging
```

---

### Priority 8: Certificate Chain Building (1.5 hours)
**Location**: `src/ca/certificate_authority.rs:286`
**Current**:
```rust
let chain_pem = String::new(); // TODO: Build proper certificate chain
```

**Impact**: Clients cannot verify certificate trust chain

**Implementation**:
```rust
async fn build_certificate_chain(&self, cert_pem: &str) -> Result<String> {
    let mut chain = vec![cert_pem.to_string()];

    // Add intermediate CA if exists
    if let Some(intermediate_path) = &self.config.intermediate_cert_path {
        let intermediate_pem = tokio::fs::read_to_string(intermediate_path).await
            .context("Failed to read intermediate certificate")?;
        chain.push(intermediate_pem);
    }

    // Add root CA
    let root_pem = tokio::fs::read_to_string(&self.config.root_cert_path).await
        .context("Failed to read root certificate")?;
    chain.push(root_pem);

    // Concatenate chain (leaf -> intermediate -> root)
    Ok(chain.join("\n"))
}
```

**Verification**:
```bash
cargo test --lib ca::tests::test_certificate_chain_building
```

---

## Day 6: Enhancement TODOs (Document/Defer)

### Document-Only TODOs (2 hours)

#### 1. Consensus Generation Migration
**Location**: `src/consensus/mod.rs:77,89`
**Action**: Add deprecation notice, create migration guide

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use generate_from_network() for production. This method is for testing only. See MIGRATION.md"
)]
pub fn generate_dummy_proof() -> ConsensusProof {
    // Keep for backward compatibility
}
```

**Create**: `docs/CONSENSUS_MIGRATION.md` explaining the migration path

---

#### 2. Quality Gate Patterns
**Location**: `src/deployment/quality_gates.rs:225,460`
**Action**: These are test patterns, keep as documentation

```rust
// Quality gate: Ensure no TODO comments related to security
"TODO.*security",
// Quality gate: Ensure no mock responses in production
"mock.*response|Mock.*certificate|TODO.*Integrate"
```

**No changes needed** - these are intentional regex patterns for quality checks

---

#### 3. Merkle Tree Algorithm
**Location**: `src/ct/certificate_transparency.rs:30,266,471`
**Action**: Document why deferred, create tracking issue

```rust
// DEFERRED: Waiting for merkletree crate API stabilization
// See: https://github.com/hypermesh-online/trustchain/issues/XXX
// Current: Using simplified Merkle tree implementation
// TODO: Re-enable when merkletree API compatibility is resolved
```

**Create GitHub Issue**: "Upgrade to merkletree crate when API stabilizes"

---

#### 4. S3 Storage Integration
**Location**: `src/ct/certificate_transparency.rs:708,716`
**Action**: Document as future enhancement

```rust
/// Upload CT log to S3 (future enhancement)
///
/// Currently returns success without uploading.
/// When implemented, will use AWS SDK for S3.
///
/// Tracking: https://github.com/hypermesh-online/trustchain/issues/XXX
async fn upload_to_s3(&self, log_data: &[u8]) -> Result<()> {
    // TODO: Implement actual S3 upload with AWS SDK
    warn!("S3 upload not yet implemented, log data not backed up");
    Ok(())
}
```

**Create GitHub Issue**: "Implement S3 backup for CT logs"

---

#### 5. CA Metrics Collection
**Location**: `src/ca/security_integration.rs:353`
**Action**: Document as enhancement

```rust
// Enhancement: CA-specific metrics collection
// Currently using basic metrics from security module
// Future: Implement detailed CA operation metrics
// Tracking: https://github.com/hypermesh-online/trustchain/issues/XXX
```

---

### Cleanup TODOs (1 hour)

#### Convert to Tracking Issues
For each deferred TODO:
1. Create GitHub issue with details
2. Replace TODO with issue reference
3. Add issue link to ROADMAP.md

**Template**:
```rust
// BEFORE
// TODO: Implement S3 upload

// AFTER
// Enhancement tracked in issue #142
// https://github.com/hypermesh-online/trustchain/issues/142
```

---

## Time Estimates

| Day | Task | Hours | Deliverable |
|-----|------|-------|-------------|
| **Day 5** | DNS STOQ Listener | 2.0 | Working listener |
| | PEM Parsing | 1.5 | Certificate parsing |
| | CSR Extraction | 1.5 | Subject parsing |
| | CA Signing | 2.0 | Proper signatures |
| | DNS Parsing | 1.0 | Message handling |
| | SAN Extraction | 1.0 | SAN parsing |
| | Client Address | 0.5 | Correct logging |
| | Chain Building | 1.5 | Full chain |
| **Day 6** | Documentation | 2.0 | Migration guides |
| | Issue Creation | 1.0 | GitHub issues |
| | Code Cleanup | 1.0 | Remove stale TODOs |
| | Verification | 2.0 | Test all changes |

---

## Verification Strategy

### After Each Priority Fix
```bash
# Build check
cargo build --lib

# Test specific functionality
cargo test --lib <module>::tests::test_<functionality>

# Check TODO count decreased
grep -r "TODO\|FIXME" src/ --include="*.rs" | grep -v "test" | wc -l
```

### End of Day 5
```bash
# Should have 8 fewer TODOs
./scripts/verify_todos.sh
# All critical functionality implemented
cargo test --lib -- --test-threads=1
```

### End of Day 6
```bash
# All TODOs either implemented or documented
grep -r "TODO" src/ --include="*.rs" | grep -v "test" | grep -v "issue #"
# Should return only documented/tracked TODOs
```

---

## Success Criteria

- [ ] 8 critical TODOs implemented with tests
- [ ] 18 enhancement TODOs documented with tracking issues
- [ ] All new functionality has unit tests
- [ ] No TODOs blocking production deployment
- [ ] Migration guides written for deferred items
- [ ] GitHub issues created for all future enhancements

---

## Code Quality Checklist

For each TODO resolution:
- [ ] Implementation includes proper error handling (no unwrap)
- [ ] Unit test covers functionality
- [ ] Documentation updated
- [ ] Related TODOs in same area addressed
- [ ] No new warnings introduced
- [ ] Integration test if cross-module

---

## Dependencies to Add

Add to `Cargo.toml`:
```toml
[dependencies]
x509-parser = "0.15"  # For PEM/certificate parsing
pem = "3.0"            # For PEM format handling
trust-dns-proto = "0.23" # Already present, verify version
openssl = "0.10"       # Already present for CSR parsing
```

Verify with:
```bash
cargo tree | grep -E "x509-parser|pem|trust-dns-proto|openssl"
```

---

## Rollback Plan

If a TODO fix breaks functionality:
1. Git revert the specific commit
2. Mark TODO as "attempted, needs more research"
3. Create detailed issue with what didn't work
4. Move to enhancement backlog

**Per-TODO Commits**:
Each TODO fix should be a separate commit for easy rollback:
```bash
git add src/api/stoq_api.rs
git commit -m "feat: Implement PEM certificate parsing (Priority 2)"
```