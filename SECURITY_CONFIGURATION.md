# Security Configuration Guide

## Overview

This document outlines security best practices for configuring and deploying the Web3 Ecosystem components (TrustChain, BlockMatrix, STOQ, Caesar).

## Environment Variable Configuration

### Using Environment Variables

All production deployments **MUST** use environment variables for sensitive configuration. Never hardcode secrets in source code.

### Configuration Loading Priority

1. **Environment variables** (highest priority)
2. **Configuration files** (.env for development)
3. **Default values** (localhost testing only)

### Example: Gateway Configuration

The gateway demonstrates proper environment variable usage:

```rust
// gateway/src/config.rs
impl GatewayConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(addr) = std::env::var("GATEWAY_LISTEN_ADDR") {
            config.listen_addr = addr.parse()?;
        }

        if let Ok(path) = std::env::var("CERT_PATH") {
            config.cert_path = PathBuf::from(path);
        }

        Ok(config)
    }
}
```

## Certificate Management

### Certificate Storage

**NEVER embed certificates or private keys in code**. Always use file system paths:

```bash
# CORRECT: Reference certificate files
TRUSTCHAIN_CERT_PATH=/var/lib/trustchain/certs/server.crt
TRUSTCHAIN_KEY_PATH=/var/lib/trustchain/certs/server.key

# WRONG: Embedding PEM data
TRUSTCHAIN_CERT="-----BEGIN CERTIFICATE-----..." # NEVER DO THIS
```

### File Permissions

Set strict permissions on certificate files:

```bash
# Private keys: read-only by owner
chmod 600 /var/lib/trustchain/certs/server.key
chown trustchain:trustchain /var/lib/trustchain/certs/server.key

# Certificates: read-only by owner and group
chmod 640 /var/lib/trustchain/certs/server.crt
chown trustchain:trustchain /var/lib/trustchain/certs/server.crt
```

### Certificate Rotation

TrustChain uses 24-hour certificate validity:

```bash
# Automated rotation via systemd timer
[Unit]
Description=TrustChain Certificate Rotation

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
```

## API Key Management

### Banking Provider Integration

Caesar integrates with banking APIs (Stripe, Plaid, OpenBanking). **NEVER commit API keys**.

#### Development Configuration

```bash
# .env (NOT committed to git)
STRIPE_API_KEY=sk_test_abc123xyz
PLAID_CLIENT_ID=dev_client_id
PLAID_SECRET=dev_secret
```

#### Production Configuration

Use a secret management system:

**HashiCorp Vault:**
```bash
# Store secrets
vault kv put secret/caesar/stripe api_key=sk_live_...

# Retrieve in application
export STRIPE_API_KEY=$(vault kv get -field=api_key secret/caesar/stripe)
```

**Kubernetes Secrets:**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: caesar-banking-secrets
type: Opaque
stringData:
  stripe-api-key: sk_live_...
  plaid-client-id: ...
  plaid-secret: ...
```

**AWS Secrets Manager:**
```bash
# Store secret
aws secretsmanager create-secret \
  --name caesar/stripe/api-key \
  --secret-string sk_live_...

# Retrieve in application startup
export STRIPE_API_KEY=$(aws secretsmanager get-secret-value \
  --secret-id caesar/stripe/api-key \
  --query SecretString \
  --output text)
```

## Configuration Files

### .env File (Development Only)

```bash
# Create .env from template
cp .env.template .env

# Edit with your development credentials
nano .env

# Verify .env is in .gitignore
git check-ignore .env  # Should output: .env
```

### TrustChain Configuration (TOML)

```toml
# trustchain.toml
[ca]
ca_id = "trustchain-ca-production"
bind_address = "::"
port = 8443

[ct]
log_id = "trustchain-ct-production"
storage_path = "/var/lib/trustchain/ct"

[api]
rate_limit_per_minute = 300
max_body_size = 10485760
```

Load configuration:

```rust
use trustchain::config::TrustChainConfig;

// Load from environment or file
let config = TrustChainConfig::from_file("trustchain.toml")?;

// Override with environment variables
let config = if let Ok(ca_port) = std::env::var("TRUSTCHAIN_CA_PORT") {
    config.ca.port = ca_port.parse()?;
    config
} else {
    config
};
```

## Secret Management Systems

### Recommended Solutions

#### Development
- **dotenv files** (.env) - Simple, file-based
- **Environment variables** - Direct export in shell

#### Staging/Production
- **HashiCorp Vault** - Full-featured secret management
- **AWS Secrets Manager** - AWS-native solution
- **Azure Key Vault** - Azure-native solution
- **Kubernetes Secrets** - Container orchestration
- **Google Secret Manager** - GCP-native solution

### Integration Example: Vault

```rust
use std::env;
use anyhow::{Result, anyhow};

async fn load_stripe_key() -> Result<String> {
    // Try environment variable first
    if let Ok(key) = env::var("STRIPE_API_KEY") {
        return Ok(key);
    }

    // Fall back to Vault
    let vault_addr = env::var("VAULT_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:8200".to_string());
    let vault_token = env::var("VAULT_TOKEN")
        .map_err(|_| anyhow!("VAULT_TOKEN not set"))?;

    // Retrieve from Vault (simplified - use vault client crate)
    let client = vault::Client::new(&vault_addr, &vault_token)?;
    let secret = client.get_secret("secret/caesar/stripe/api-key").await?;

    Ok(secret)
}
```

## Audit and Compliance

### Security Checklist

- [ ] All production secrets use environment variables or secret management
- [ ] .env file is in .gitignore
- [ ] No hardcoded API keys, passwords, or tokens in source code
- [ ] Certificate private keys have 600 permissions
- [ ] API keys rotate every 90 days minimum
- [ ] Certificates rotate according to TrustChain policy (24 hours)
- [ ] Secret management system logs all access
- [ ] Least privilege principle applied to all credentials

### Verification Commands

```bash
# Check for hardcoded secrets in source code
grep -r "sk_live_\|pk_live_\|password.*=.*[\"']" \
  --include="*.rs" \
  --exclude-dir=tests \
  --exclude-dir=examples \
  .

# Should return NO results in production code

# Verify .env is ignored
git check-ignore .env
# Should output: .env

# Check file permissions on certificates
ls -la /var/lib/trustchain/certs/
# server.key should be -rw------- (600)
# server.crt should be -rw-r----- (640)
```

## Mock/Test Data

### Acceptable Test Patterns

Mock providers for testing are acceptable if clearly marked:

```rust
/// Mock Banking Provider for Testing ONLY
/// NEVER use in production
pub struct MockBankingProvider {
    // ...
}

impl BankingApiProvider for MockBankingProvider {
    async fn authenticate(&self, _credentials: &BankingCredentials) -> Result<AuthToken> {
        Ok(AuthToken {
            token: "mock_token".to_string(),  // Clearly a test token
            // ...
        })
    }
}
```

### Example Code Patterns

Example/test code may contain mock data if:

1. Located in `tests/` or `examples/` directories
2. Clearly marked as non-production
3. Uses obviously fake values ("mock_", "test_", etc.)

```rust
// examples/falcon_integration.rs
let secret_data = b"This is secret data encrypted..."; // OK - example code

// catalog/tests/full_system_test.rs
let mock_cert = "-----BEGIN CERTIFICATE-----\nMOCK_CERT\n..."; // OK - test data
```

## Incident Response

### Compromised Credentials

If credentials are compromised:

1. **Immediate**: Revoke/rotate compromised credentials
2. **Audit**: Check logs for unauthorized access
3. **Notify**: Inform security team and affected parties
4. **Remediate**: Update secret management procedures
5. **Document**: Post-mortem analysis

### Example: Stripe API Key Rotation

```bash
# 1. Generate new key in Stripe dashboard
NEW_KEY=sk_live_new_key_xyz

# 2. Update in secret management system
vault kv put secret/caesar/stripe api_key=$NEW_KEY

# 3. Rolling restart of Caesar services
kubectl rollout restart deployment/caesar

# 4. Verify new key is active
kubectl logs -l app=caesar | grep "Stripe API initialized"

# 5. Revoke old key in Stripe dashboard
```

## Additional Resources

- [OWASP Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [Kubernetes Secrets](https://kubernetes.io/docs/concepts/configuration/secret/)
- [AWS Secrets Manager](https://aws.amazon.com/secrets-manager/)

## Contact

For security concerns or questions:
- Security Team: security@hypermesh.online
- Emergency: security-emergency@hypermesh.online
