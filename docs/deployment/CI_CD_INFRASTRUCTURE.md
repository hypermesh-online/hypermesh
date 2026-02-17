# CI/CD Infrastructure Documentation

## Overview

The HyperMesh ecosystem employs a comprehensive CI/CD infrastructure designed to prevent regressions, enforce quality standards, and enable rapid, reliable deployments across all components.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     GitHub Repository                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Commit     │→ │ Quality Gates│→ │  Deployment  │      │
│  │   Trigger    │  │   Pipeline   │  │   Pipeline   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                            ↓                  ↓              │
│                    ┌──────────────┐  ┌──────────────┐      │
│                    │   Security   │  │  Monitoring  │      │
│                    │    Scans     │  │   & Alerts   │      │
│                    └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Quality Gates

### 1. Compilation Gate (100% Required)
- **Threshold**: All components must compile successfully
- **Enforcement**: Blocks all deployments on failure
- **Components**: stoq, trustchain, hypermesh, caesar, catalog

### 2. Security Gate (Zero Tolerance)
- **Threshold**: 0 critical security vulnerabilities
- **Tools**: cargo-audit, clippy security lints, secret scanning
- **Enforcement**: Immediate PR block on violations

### 3. Performance Gate (20% Regression Threshold)
- **Metrics**:
  - STOQ: 2.95 Gbps throughput minimum
  - TrustChain: 35ms latency maximum
  - Catalog: 1.69ms operation time maximum
- **Enforcement**: Warnings on degradation, blocks on severe regression

### 4. Test Coverage Gate (60% Minimum)
- **Threshold**: 60% code coverage across all components
- **Tools**: cargo-tarpaulin, integration test suite
- **Enforcement**: Warning below threshold, educates on improvement

### 5. Documentation Gate
- **Requirements**: All public APIs documented
- **Validation**: cargo doc --no-deps must succeed without warnings
- **Enforcement**: Advisory warnings, blocks on missing critical docs

## Workflows

### Core Workflows

#### 1. Quality Gates (`quality-gates.yml`)
- **Trigger**: Every PR, push to main/develop
- **Duration**: ~10 minutes
- **Purpose**: Enforce all quality standards before merge

#### 2. Performance Monitoring (`performance-monitoring.yml`)
- **Trigger**: Push to main, PRs, every 6 hours
- **Duration**: ~30 minutes
- **Purpose**: Track performance metrics and detect regressions

#### 3. Automated Deployment (`automated-deployment.yml`)
- **Trigger**: Push to main, version tags
- **Duration**: ~20 minutes
- **Strategy**: Blue-green for staging, canary for production

#### 4. Security Scanning (`security.yml`)
- **Trigger**: Daily, on PR
- **Duration**: ~15 minutes
- **Scope**: Dependencies, containers, infrastructure

#### 5. Monitoring & Alerts (`monitoring-alerts.yml`)
- **Trigger**: Every 5 minutes
- **Purpose**: Health checks, performance tracking, incident response

## Deployment Strategy

### Environments

| Environment | Purpose | Deployment Method | Rollback |
|------------|---------|-------------------|----------|
| Development | Testing | Direct | Manual |
| Staging | Pre-production validation | Blue-Green | Automatic |
| Production | Live system | Canary (gradual) | Automatic |

### Deployment Flow

```
1. Code Commit
   ↓
2. Quality Gates Check
   ↓ (Pass all gates)
3. Build Artifacts
   ↓
4. Container Build
   ↓
5. Staging Deployment (Blue-Green)
   ↓ (Smoke tests pass)
6. Production Approval
   ↓
7. Canary Deployment (10% → 25% → 50% → 100%)
   ↓
8. Health Monitoring
   ↓ (If unhealthy)
9. Automatic Rollback
```

### Blue-Green Deployment (Staging)

1. Deploy new version as "green" alongside existing "blue"
2. Run health checks on green deployment
3. Switch traffic to green
4. Monitor for issues (5 minutes)
5. Remove blue deployment if successful

### Canary Deployment (Production)

1. Deploy new version to 10% of infrastructure
2. Monitor error rates and performance (5 minutes)
3. Gradually increase traffic: 25% → 50% → 75% → 100%
4. Full rollback on any anomaly detection

## Scripts

### `deploy-orchestrator.sh`
Orchestrates deployment across all components with dependency management.

**Usage:**
```bash
./scripts/deploy-orchestrator.sh [environment] [version] [dry-run]

# Examples:
./scripts/deploy-orchestrator.sh staging latest false
./scripts/deploy-orchestrator.sh production v1.0.0 true
```

### `quality-gate-enforcer.sh`
Runs comprehensive quality checks locally before commit.

**Usage:**
```bash
./scripts/quality-gate-enforcer.sh

# Output includes:
# - Compilation status
# - Security vulnerabilities
# - Test coverage
# - Code quality metrics
# - Performance benchmarks
```

## Performance Benchmarks

### Current Baselines

| Component | Metric | Target | Current | Status |
|-----------|--------|--------|---------|--------|
| STOQ | Throughput | 10 Gbps | 2.95 Gbps | ⚠️ Optimization needed |
| TrustChain | Latency | 5ms | 35ms | ✅ Acceptable |
| Catalog | Operations | 3ms | 1.69ms | ✅ Exceeds target |
| HyperMesh | Consensus | 100ms | 95ms | ✅ Within target |
| Caesar | Transaction | 50ms | 48ms | ✅ Within target |

## Monitoring & Alerts

### Alert Thresholds

| Metric | Warning | Critical | Action |
|--------|---------|----------|--------|
| Error Rate | 0.5% | 1.0% | Page on-call |
| Latency | 80ms | 100ms | Scale up |
| CPU Usage | 70% | 80% | Add nodes |
| Memory | 80% | 90% | Investigate leak |
| Disk Usage | 75% | 85% | Cleanup/expand |

### Incident Response

1. **Detection**: Automated monitoring every 5 minutes
2. **Alert**: Slack notification, GitHub issue creation
3. **Triage**: Automated recovery attempt
4. **Escalation**: Page on-call if auto-recovery fails
5. **Resolution**: Deploy fix or rollback
6. **Post-mortem**: Document and prevent recurrence

## Security Measures

### Automated Security Scanning

- **Dependency Audit**: Daily via cargo-audit
- **Container Scanning**: Trivy on every build
- **Secret Detection**: TruffleHog + GitLeaks
- **SAST**: Semgrep for code analysis
- **Infrastructure**: Terraform security scanning

### Security Gate Enforcement

```yaml
Critical Issues: 0 tolerance - blocks deployment
High Issues: Manual review required
Medium Issues: Must be acknowledged
Low Issues: Tracked for resolution
```

## Rollback Procedures

### Automatic Rollback Triggers

1. Health check failures (3 consecutive)
2. Error rate > 1%
3. Latency > 2x baseline
4. Security vulnerability detected
5. Smoke test failures

### Manual Rollback

```bash
# Staging rollback
kubectl rollout undo deployment/[component] -n hypermesh-staging

# Production rollback
./scripts/deploy-orchestrator.sh production [previous-version] false
```

## Continuous Improvement

### Metrics Tracked

- **Deployment Frequency**: Target 10+ per week
- **Lead Time**: Target < 1 hour
- **MTTR**: Target < 15 minutes
- **Change Failure Rate**: Target < 5%
- **Test Coverage**: Target > 80%

### Weekly Reviews

1. Performance trending analysis
2. Security vulnerability review
3. Deployment success rate
4. Incident post-mortems
5. Quality gate effectiveness

## Best Practices

### For Developers

1. **Run quality gates locally** before pushing:
   ```bash
   ./scripts/quality-gate-enforcer.sh
   ```

2. **Write tests** for all new features (minimum 60% coverage)

3. **Document APIs** - all public functions need documentation

4. **Performance test** significant changes:
   ```bash
   cargo bench --bench [component]
   ```

5. **Security review** dependencies before adding:
   ```bash
   cargo audit
   ```

### For Operations

1. **Monitor dashboards** during deployments
2. **Verify health checks** after each deployment
3. **Document incidents** for post-mortem analysis
4. **Test rollback procedures** regularly
5. **Update runbooks** with new procedures

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Build timeout | Large dependencies | Increase cache, optimize Dockerfile |
| Test flakiness | Race conditions | Add retries, fix async tests |
| Deploy failure | Resource limits | Scale cluster, optimize resources |
| Rollback failure | State corruption | Manual intervention, restore backup |
| Alert fatigue | Low thresholds | Tune thresholds, reduce noise |

### Debug Commands

```bash
# Check workflow status
gh workflow view

# View recent runs
gh run list --workflow=quality-gates.yml

# Debug failed job
gh run view [run-id] --log-failed

# Re-run failed jobs
gh run rerun [run-id] --failed

# Download artifacts
gh run download [run-id]
```

## Contact & Support

- **On-call Rotation**: See PagerDuty schedule
- **Slack Channels**: #monitoring, #incidents, #deployments
- **Documentation**: This file + runbooks/
- **Issues**: GitHub Issues with `ci-cd` label

## Appendix

### Required Secrets

Configure these in GitHub repository settings:

```yaml
AWS_ACCESS_KEY_ID: AWS deployment credentials
AWS_SECRET_ACCESS_KEY: AWS deployment credentials
STAGING_KUBE_CONFIG: Base64 encoded kubeconfig
PRODUCTION_KUBE_CONFIG: Base64 encoded kubeconfig
DOCKER_USERNAME: Docker Hub credentials
DOCKER_PASSWORD: Docker Hub credentials
CRATES_IO_TOKEN: For publishing to crates.io
SLACK_WEBHOOK: For notifications
```

### Tools & Dependencies

- **Rust**: stable toolchain
- **Docker**: 20.10+
- **Kubernetes**: 1.28+
- **Helm**: 3.12+
- **GitHub CLI**: 2.0+
- **cargo-audit**: Latest
- **cargo-tarpaulin**: Latest

---

**Last Updated**: December 2024
**Version**: 1.0.0
**Status**: Production Ready