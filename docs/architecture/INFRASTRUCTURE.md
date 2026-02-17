# HyperMesh Infrastructure Deployment Guide

## Overview

This document provides comprehensive instructions for deploying the HyperMesh Web3 ecosystem infrastructure, including CI/CD pipelines, container orchestration, monitoring, and auto-scaling capabilities.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         CloudFront CDN                       │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────────┐
│                    Application Load Balancer                 │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────────┐
│                      Kubernetes Cluster                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    Ingress Controller                │   │
│  └────────┬──────────┬──────────┬──────────┬──────────┘   │
│           │          │          │          │                │
│  ┌────────┴───┐ ┌───┴────┐ ┌──┴───┐ ┌────┴────┐          │
│  │   STOQ     │ │TrustCh │ │HyperM│ │ Caesar  │          │
│  │  Protocol  │ │  CA    │ │ esh  │ │Economic │          │
│  └────────────┘ └────────┘ └──────┘ └─────────┘          │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │            Native Monitoring System (eBPF)           │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
                       │                    │
              ┌────────┴─────────┐ ┌───────┴────────┐
              │   PostgreSQL     │ │     Redis      │
              └──────────────────┘ └────────────────┘
```

## Quick Start

### Prerequisites

1. **Required Tools**:
   ```bash
   # Install required tools
   brew install git docker kubectl helm terraform aws-cli gh

   # Install Rust toolchain
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Configure Credentials**:
   ```bash
   # AWS credentials
   aws configure

   # GitHub CLI
   gh auth login

   # Docker Hub
   docker login
   ```

### Deployment Options

#### Option 1: Automated Deployment (Recommended)

```bash
# Deploy to staging
./infrastructure/deploy.sh staging

# Deploy to production
./infrastructure/deploy.sh production v1.0.0

# Dry run (preview changes)
./infrastructure/deploy.sh staging latest true
```

#### Option 2: Manual Deployment

1. **Build Components**:
   ```bash
   ./build-all.sh
   ```

2. **Deploy Infrastructure**:
   ```bash
   cd infrastructure/terraform
   terraform init
   terraform apply -var="environment=staging"
   ```

3. **Deploy to Kubernetes**:
   ```bash
   kubectl apply -f infrastructure/kubernetes/
   ```

4. **Setup Monitoring**:
   ```bash
   ./infrastructure/monitoring/setup-monitoring.sh
   ```

## CI/CD Pipeline

### GitHub Actions Workflows

The project includes comprehensive CI/CD pipelines:

1. **CI Pipeline** (`.github/workflows/ci.yml`):
   - Linting and formatting checks
   - Unit and integration tests
   - Security audits
   - Performance benchmarks
   - Docker image builds

2. **Deploy Pipeline** (`.github/workflows/deploy.yml`):
   - Infrastructure provisioning
   - Kubernetes deployments
   - Smoke tests
   - Automatic rollback on failure

3. **Release Pipeline** (`.github/workflows/release.yml`):
   - GitHub release creation
   - Binary builds for multiple platforms
   - Docker image publishing
   - Crate publishing to crates.io

4. **Security Pipeline** (`.github/workflows/security.yml`):
   - Dependency vulnerability scanning
   - Container image scanning
   - Static code analysis
   - Secret detection

### Triggering Deployments

```bash
# Create a new release
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# Manual deployment
gh workflow run deploy.yml -f environment=production
```

## Container Management

### Docker Compose (Local Development)

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Clean up
docker-compose down -v
```

### Docker Registry

Images are published to GitHub Container Registry:

```bash
# Pull images
docker pull ghcr.io/hypermesh-online/hypermesh:latest
docker pull ghcr.io/hypermesh-online/stoq:latest
docker pull ghcr.io/hypermesh-online/trustchain:latest
```

## Kubernetes Operations

### Helm Deployment

```bash
# Add Helm dependencies
helm dependency update infrastructure/helm

# Install HyperMesh stack
helm install hypermesh-stack infrastructure/helm \
  --namespace hypermesh \
  --create-namespace \
  --values infrastructure/helm/values.yaml

# Upgrade deployment
helm upgrade hypermesh-stack infrastructure/helm \
  --namespace hypermesh \
  --values infrastructure/helm/values.yaml

# Rollback if needed
helm rollback hypermesh-stack -n hypermesh
```

### Scaling

```bash
# Manual scaling
kubectl scale deployment/hypermesh -n hypermesh --replicas=5

# Auto-scaling configuration
kubectl apply -f infrastructure/kubernetes/hpa.yaml
```

### Monitoring

```bash
# Access monitoring dashboard
kubectl port-forward -n hypermesh service/nexus-ui 3000:80

# View metrics
kubectl port-forward -n hypermesh service/hypermesh 9090:9090
```

## Infrastructure as Code

### Terraform Structure

```
infrastructure/terraform/
├── main.tf              # Main configuration
├── variables.tf         # Input variables
├── outputs.tf          # Output values
├── modules/
│   ├── vpc/           # VPC configuration
│   ├── eks/           # EKS cluster
│   ├── rds/           # PostgreSQL database
│   ├── elasticache/   # Redis cache
│   ├── s3/            # Object storage
│   ├── alb/           # Load balancer
│   ├── cloudfront/    # CDN
│   └── monitoring/    # CloudWatch alarms
```

### Managing Infrastructure

```bash
# Initialize Terraform
terraform init

# Plan changes
terraform plan -out=tfplan

# Apply changes
terraform apply tfplan

# Destroy infrastructure (careful!)
terraform destroy
```

## Monitoring and Observability

### Native Monitoring System

The HyperMesh stack includes a built-in monitoring system with:

- **eBPF-based collection**: Kernel-level metrics without overhead
- **Real-time dashboards**: Nexus UI for visualization
- **Custom alerts**: Configurable thresholds and notifications
- **No external dependencies**: No Prometheus/Grafana required

### Accessing Metrics

```bash
# Metrics endpoints
curl http://localhost:9090/metrics          # Prometheus-format metrics
curl http://localhost:8080/health          # Health check
curl http://localhost:8080/ready           # Readiness check
```

### Setting Up Alerts

Edit `infrastructure/monitoring/alerts.yaml`:

```yaml
alerts:
  - name: high_cpu_usage
    metric: cpu_usage_percent
    threshold: 80
    duration: 5m
    severity: warning

  - name: service_down
    metric: up
    threshold: 0
    duration: 1m
    severity: critical
```

## Security

### Certificate Management

TrustChain handles all certificate operations:

```bash
# View certificate status
curl https://trust.hypermesh.online/api/v1/status

# Rotate certificates manually
kubectl exec -n hypermesh trustchain-0 -- trustchain rotate-certs
```

### Network Policies

```bash
# Apply network policies
kubectl apply -f infrastructure/kubernetes/network-policies.yaml

# Verify policies
kubectl get networkpolicies -n hypermesh
```

### Secret Management

```bash
# Create secrets
kubectl create secret generic hypermesh-secrets \
  --from-literal=db-password=$DB_PASSWORD \
  --from-literal=redis-password=$REDIS_PASSWORD \
  -n hypermesh

# Rotate secrets
./scripts/rotate-secrets.sh
```

## Troubleshooting

### Common Issues

1. **Pod CrashLoopBackOff**:
   ```bash
   kubectl logs -n hypermesh <pod-name> --previous
   kubectl describe pod -n hypermesh <pod-name>
   ```

2. **Service Unavailable**:
   ```bash
   kubectl get endpoints -n hypermesh
   kubectl get svc -n hypermesh
   ```

3. **High Resource Usage**:
   ```bash
   kubectl top nodes
   kubectl top pods -n hypermesh
   ```

### Debug Commands

```bash
# Enter container shell
kubectl exec -it -n hypermesh <pod-name> -- /bin/bash

# View events
kubectl get events -n hypermesh --sort-by='.lastTimestamp'

# Check cluster status
kubectl cluster-info
kubectl get nodes
```

## Performance Tuning

### Optimization Settings

1. **STOQ Protocol**:
   - Adaptive bandwidth tiers: 100 Mbps, 1 Gbps, 2.5 Gbps
   - Memory pool: 1GB pre-allocated
   - Max connections: 10,000

2. **HyperMesh Core**:
   - CPU affinity for high-performance nodes
   - NUMA-aware memory allocation
   - GPU acceleration enabled

3. **Database**:
   - Connection pooling: 100 connections
   - Query caching enabled
   - Auto-vacuum configured

## Backup and Recovery

### Automated Backups

```bash
# Configure backup schedule
kubectl apply -f infrastructure/kubernetes/backup-cronjob.yaml

# Manual backup
./scripts/backup.sh

# Restore from backup
./scripts/restore.sh <backup-id>
```

### Disaster Recovery

1. **Multi-region deployment**:
   - Primary: us-east-1
   - Secondary: eu-west-1
   - Automatic failover configured

2. **Data replication**:
   - PostgreSQL streaming replication
   - Redis master-replica setup
   - S3 cross-region replication

## Cost Optimization

### Resource Recommendations

- **Development**: t3.medium instances (2 vCPU, 4GB RAM)
- **Staging**: t3.large instances (2 vCPU, 8GB RAM)
- **Production**: c6i.2xlarge instances (8 vCPU, 16GB RAM)

### Auto-scaling Policies

```yaml
# CPU-based scaling
targetCPUUtilizationPercentage: 70

# Memory-based scaling
targetMemoryUtilizationPercentage: 80

# Custom metrics
customMetrics:
  - name: requests_per_second
    targetValue: 1000
```

## Support

### Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [API Documentation](docs/API.md)
- [Security Guidelines](docs/SECURITY.md)

### Getting Help

- GitHub Issues: https://github.com/hypermesh-online/hypermesh/issues
- Documentation: https://docs.hypermesh.online
- Community: https://discord.gg/hypermesh

---

**Version**: 1.0.0
**Last Updated**: November 2024
**Status**: Production Ready