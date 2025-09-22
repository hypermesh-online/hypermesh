# ARCHIVED - Content moved to /hypermesh/docs/cli/user-guide.md

*This file has been consolidated as part of documentation compression.*

The `nexus` CLI is the primary command-line interface for managing HyperMesh/Nexus distributed infrastructure. It provides comprehensive cluster management, service orchestration, debugging, and operational capabilities built on top of Nexus's revolutionary P2P mesh architecture.

## Key Features

### 🚀 **Core Management**
- **Cluster Operations**: Create, scale, upgrade, and manage distributed clusters
- **Service Orchestration**: Deploy, manage, and scale containerized applications
- **Node Management**: Monitor, drain, cordon, and troubleshoot cluster nodes
- **P2P Networking**: Native peer-to-peer mesh networking with automatic service discovery

### 🔐 **Security First**
- **Certificate Management**: Automatic certificate generation, rotation, and validation
- **Network Policies**: Fine-grained network security and traffic control
- **RBAC Integration**: Role-based access control with multi-tenancy support
- **Encryption**: End-to-end encryption for all communications

### 📊 **Observability & Debugging**
- **Real-time Monitoring**: Live metrics, resource usage, and performance analytics
- **Log Aggregation**: Centralized logging with advanced filtering and search
- **Distributed Tracing**: Full request tracing across service mesh
- **Troubleshooting Tools**: Network diagnostics, health checks, and debugging utilities

### ⚡ **Performance & Scalability**
- **ML-Powered Scheduling**: Intelligent workload placement using machine learning
- **Auto-scaling**: Predictive horizontal and vertical scaling
- **Resource Optimization**: Automatic resource tuning and cost optimization
- **Edge Computing**: Support for edge deployments and geographic distribution

## Installation

### Binary Installation
```bash
# Download latest release
curl -LO https://github.com/hypermesh/nexus/releases/latest/download/nexus-linux-amd64.tar.gz

# Extract and install
tar -xzf nexus-linux-amd64.tar.gz
sudo mv nexus /usr/local/bin/

# Verify installation
nexus version
```

### From Source
```bash
# Clone repository
git clone https://github.com/hypermesh/nexus.git
cd nexus/interface/phase2-c2/cli

# Build and install
cargo build --release
sudo cp target/release/nexus /usr/local/bin/
```

### Package Managers
```bash
# Homebrew (macOS)
brew install hypermesh/tap/nexus

# Apt (Ubuntu/Debian)
sudo apt-get install nexus-cli

# Yum (RHEL/CentOS)
sudo yum install nexus-cli
```

## Quick Start

### 1. Initial Configuration
```bash
# Create configuration directory
mkdir -p ~/.nexus

# Generate initial configuration
nexus config init

# Set up authentication
nexus config set-credentials production --server=https://nexus.company.com:8443 --token=<your-token>
```

### 2. Connect to Cluster
```bash
# List available contexts
nexus config get-contexts

# Switch to production context
nexus config use-context production

# Verify connection
nexus cluster status
```

### 3. Deploy Your First Service
```bash
# Deploy nginx service
nexus service deploy nginx:latest --name web-server --replicas 3 --port 80

# Check deployment status
nexus service list

# Scale the service
nexus service scale web-server --replicas 5
```

## Command Reference

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `--config, -c` | Configuration file path | `~/.nexus/config.yaml` |
| `--context` | Nexus context to use | current-context |
| `--output, -o` | Output format (table/json/yaml/wide) | table |
| `--verbose, -v` | Verbosity level (stackable: -v, -vv, -vvv) | 0 |
| `--dry-run` | Preview changes without applying | false |
| `--timeout` | Operation timeout | 30s |

### Cluster Management

#### Create a New Cluster
```bash
nexus cluster create production \\
  --nodes 5 \\
  --provider aws \\
  --region us-west-2 \\
  --zones us-west-2a,us-west-2b,us-west-2c \\
  --node-type m5.xlarge \\
  --enable-ebpf \\
  --network-mode p2p
```

#### Scale Cluster
```bash
# Scale to 10 nodes
nexus cluster scale production --nodes 10

# Enable auto-scaling
nexus cluster configure production --auto-scaling --min-nodes 5 --max-nodes 20
```

#### Upgrade Cluster
```bash
# Upgrade to latest version
nexus cluster upgrade production --version latest

# Rolling upgrade with custom parameters
nexus cluster upgrade production \\
  --version v1.2.0 \\
  --max-unavailable 1 \\
  --rolling
```

#### Monitor Cluster Health
```bash
# Basic status
nexus cluster status production

# Detailed status with metrics
nexus cluster status production --detailed --show-events --show-metrics

# Watch mode (continuous updates)
nexus cluster status production --watch
```

### Service Management

#### Deploy Services
```bash
# Simple deployment
nexus service deploy nginx:1.20 --name web-app --replicas 3

# Advanced deployment with resource limits
nexus service deploy myapp:v2.1.0 \\
  --name myapp \\
  --replicas 5 \\
  --cpu-limit 1000m \\
  --memory-limit 1Gi \\
  --cpu-request 200m \\
  --memory-request 256Mi \\
  --port 8080:80 \\
  --env \"ENV=production,LOG_LEVEL=info\" \\
  --health-check \"/health\" \\
  --ready-check \"/ready\"
```

#### Service Configuration from File
```yaml
# myapp-service.yaml
apiVersion: nexus.io/v1
kind: Service
metadata:
  name: myapp
  labels:
    app: myapp
    version: v2.1.0
spec:
  replicas: 5
  template:
    spec:
      containers:
      - name: app
        image: myapp:v2.1.0
        ports:
        - containerPort: 8080
        resources:
          requests:
            cpu: 200m
            memory: 256Mi
          limits:
            cpu: 1000m
            memory: 1Gi
```

```bash
# Deploy from configuration file
nexus service create -f myapp-service.yaml
```

#### Update and Rollback
```bash
# Update service image
nexus service update myapp --image myapp:v2.2.0

# Rollback to previous version
nexus service rollback myapp

# Rollback to specific revision
nexus service rollback myapp --revision 3
```

#### Service Scaling and Management
```bash
# Manual scaling
nexus service scale myapp --replicas 10

# Auto-scaling configuration
nexus service autoscale myapp \\
  --min-replicas 3 \\
  --max-replicas 20 \\
  --cpu-percent 70 \\
  --memory-percent 80

# Service logs
nexus debug logs service/myapp --follow --tail 100

# Service metrics
nexus metrics get service/myapp --time-range 1h
```

### Node Management

#### List and Describe Nodes
```bash
# List all nodes
nexus node list

# List with resource information
nexus node list --show-resources --show-conditions

# Describe specific node
nexus node describe worker-node-1 --show-pods --show-metrics --show-events
```

#### Node Maintenance
```bash
# Drain node for maintenance
nexus node drain worker-node-1 --ignore-daemonsets --delete-local-data

# Cordon node (make unschedulable)
nexus node cordon worker-node-1

# Uncordon node (make schedulable)
nexus node uncordon worker-node-1
```

#### Node Labeling and Resource Monitoring
```bash
# Add labels to node
nexus node label worker-node-1 \\
  --labels \"node-type=compute,zone=us-west-2a,dedicated=ml-workloads\"

# Show resource usage
nexus node top --sort-by cpu

# Monitor specific node resources
nexus debug top nodes --sort-by memory --no-headers
```

### Network Management

#### Network Policies
```bash
# Create network policy
nexus network policy create web-app-policy \\
  --allow-ingress \"from=frontend,ports=80,443\" \\
  --allow-egress \"to=database,ports=5432\" \\
  --deny-all

# List network policies
nexus network list --type policy

# Apply policy from file
nexus network policy create -f network-policy.yaml
```

#### Service Mesh Configuration
```bash
# Enable service mesh with mutual TLS
nexus network mesh configure --enable-mtls --traffic-policy strict

# Check service mesh status
nexus network mesh status

# Configure traffic management
nexus network mesh configure \\
  --retry-policy \"attempts=3,timeout=30s\" \\
  --circuit-breaker \"threshold=5,timeout=30s\" \\
  --load-balancer round-robin
```

#### Ingress Management
```bash
# Create ingress
nexus network ingress create web-app-ingress \\
  --host myapp.company.com \\
  --path \"/\" \\
  --service web-app:80 \\
  --tls web-app-tls

# List ingresses
nexus network list --type ingress
```

### Security Management

#### Certificate Management
```bash
# List certificates
nexus security certificate list

# Show certificates expiring within 30 days
nexus security certificate list --expiring-within 30d

# Create new certificate
nexus security certificate create web-app-cert \\
  --hosts \"myapp.company.com,api.company.com\" \\
  --validity 8760h

# Rotate all certificates
nexus security certificate rotate --all
```

#### Security Policies
```bash
# Create RBAC policy
nexus security policy create developer-policy \\
  --type rbac \\
  --subjects \"group:developers\" \\
  --rules \"get,list,watch:pods,services\"

# Create pod security policy
nexus security policy create restricted-policy \\
  --type pod \\
  --config-file pod-security-policy.yaml

# List security policies
nexus security policy list
```

### Storage Management

#### Volume Management
```bash
# List storage resources
nexus storage list volumes

# Create persistent volume
nexus storage create volume myapp-data \\
  --size 100Gi \\
  --storage-class fast-ssd \\
  --access-mode ReadWriteOnce

# Create volume snapshot
nexus storage snapshot myapp-data --name myapp-backup-$(date +%Y%m%d)
```

#### Storage Classes
```bash
# List storage classes
nexus storage list classes

# Create storage class
nexus storage create class fast-nvme \\
  --provisioner ebs.csi.aws.com \\
  --parameters \"type=gp3,iops=3000,encrypted=true\"
```

### Workload Management

#### Jobs and CronJobs
```bash
# Run one-time job
nexus workload run ubuntu:latest \\
  --name data-migration \\
  --restart-policy Never \\
  --command \"bash\" \\
  --args \"-c,/scripts/migrate-data.sh\"

# Create batch job
nexus workload create job batch-processor \\
  --image batch-app:v1.0.0 \\
  --completions 10 \\
  --parallelism 3

# Create scheduled job
nexus workload create cronjob nightly-backup \\
  --schedule \"0 2 * * *\" \\
  --image backup-tool:latest \\
  --command \"backup.sh\"
```

### Debugging and Troubleshooting

#### Log Management
```bash
# View service logs
nexus debug logs service/web-app --follow --tail 50

# View logs from specific container
nexus debug logs pod/web-app-123 --container sidecar --since 1h

# View logs from previous container instance
nexus debug logs pod/web-app-123 --previous
```

#### Interactive Debugging
```bash
# Execute command in container
nexus debug exec web-app-123 -- /bin/bash

# Execute with TTY and stdin
nexus debug exec web-app-123 --tty --stdin -- sh

# Port forwarding
nexus debug port-forward web-app-123 8080:80

# Start API proxy
nexus debug proxy --port 8080
```

#### System Diagnostics
```bash
# Show cluster events
nexus debug events --watch --since 1h

# Show resource usage
nexus debug top pods --sort-by cpu --namespace production
nexus debug top nodes --sort-by memory

# Network tracing
nexus debug trace --from pod/web-app-1 --to pod/database-1 --port 5432

# Comprehensive troubleshooting
nexus debug troubleshoot service/web-app --network --dns --certs
```

#### Cluster Information Dump
```bash
# Create comprehensive cluster dump
nexus debug dump --output-dir /tmp/cluster-dump \\
  --include-logs \\
  --include-metrics

# The dump includes:
# - Cluster configuration and state
# - Node information and status
# - Service definitions and status
# - Network policies and configurations
# - Storage information
# - Recent logs (if --include-logs)
# - Performance metrics (if --include-metrics)
```

### Metrics and Monitoring

#### Resource Metrics
```bash
# Get service metrics
nexus metrics get service/web-app --time-range 24h --metric-name cpu_usage

# Get node metrics with custom aggregation
nexus metrics get node/worker-1 \\
  --time-range 7d \\
  --aggregation avg \\
  --format csv

# Real-time metrics dashboard
nexus metrics dashboard --port 3000 --no-browser
```

#### Alerting
```bash
# List active alerts
nexus metrics alerts --active

# Show all alerts with history
nexus metrics alerts
```

### Configuration Management

#### Context and Cluster Configuration
```bash
# List available contexts
nexus config get-contexts

# Switch context
nexus config use-context staging

# Set configuration values
nexus config set output json
nexus config set editor vim

# View current configuration
nexus config view

# Set cluster credentials
nexus config set-cluster production \\
  --server https://nexus-prod.company.com:8443 \\
  --certificate-authority /path/to/ca.crt

# Set user credentials
nexus config set-credentials admin \\
  --client-certificate /path/to/admin.crt \\
  --client-key /path/to/admin.key

# Create new context
nexus config set-context production \\
  --cluster production \\
  --user admin \\
  --namespace default
```

## Configuration Files

### Main CLI Configuration (`~/.nexus/config.yaml`)
```yaml
apiVersion: nexus.io/v1
kind: Config
current-context: production

contexts:
- name: production
  context:
    cluster: prod-cluster
    namespace: default
    user: admin

clusters:
- name: prod-cluster
  cluster:
    server: https://nexus-api.production.company.com:8443
    certificate-authority: ~/.nexus/ca-prod.crt

users:
- name: admin
  user:
    client-certificate: ~/.nexus/admin.crt
    client-key: ~/.nexus/admin.key

preferences:
  colors: true
  editor: vim
  output: table
```

### Cluster Configuration
```yaml
apiVersion: nexus.io/v1
kind: Cluster
metadata:
  name: production-cluster
spec:
  version: v1.0.0
  provider:
    type: aws
    region: us-west-2
    zones: [us-west-2a, us-west-2b, us-west-2c]
  
  nodes:
    controller:
      count: 3
      instanceType: m5.large
    worker:
      count: 5
      instanceType: m5.xlarge
      autoScaling:
        enabled: true
        minNodes: 3
        maxNodes: 20
  
  networking:
    mode: p2p
    mesh:
      enabled: true
      mtls: true
  
  security:
    rbac:
      enabled: true
    networkPolicy:
      enabled: true
    encryption:
      atRest: true
      inTransit: true
```

### Service Configuration
```yaml
apiVersion: nexus.io/v1
kind: Service
metadata:
  name: web-application
  labels:
    app: web-app
    version: v2.1.0
spec:
  replicas: 5
  template:
    spec:
      containers:
      - name: web-app
        image: company/web-app:v2.1.0
        ports:
        - containerPort: 8080
        resources:
          requests:
            cpu: 200m
            memory: 256Mi
          limits:
            cpu: 1000m
            memory: 1Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
```

## Advanced Usage

### Automation and Scripting
```bash
#!/bin/bash

# Deploy application stack
nexus service create -f database.yaml
nexus service create -f backend.yaml
nexus service create -f frontend.yaml

# Wait for services to be ready
nexus service wait backend --condition=Ready --timeout=300s
nexus service wait frontend --condition=Ready --timeout=300s

# Create ingress
nexus network ingress create app-ingress \\
  --host myapp.company.com \\
  --service frontend:80

# Set up monitoring
nexus metrics configure --enable-alerting \\
  --alert-webhook https://alerts.company.com/webhook

echo "Application deployed successfully!"
```

### Batch Operations
```bash
# Scale multiple services
nexus service scale web-app --replicas 10 &
nexus service scale api-server --replicas 5 &
nexus service scale worker-queue --replicas 8 &
wait

# Update multiple services
for service in web-app api-server worker-queue; do
  nexus service update $service --image $service:v2.0.0 &
done
wait
```

### Custom Output Formats
```bash
# JSON output for scripting
SERVICES=$(nexus service list -o json | jq -r '.items[].name')

# YAML output for configuration management
nexus service describe web-app -o yaml > web-app-backup.yaml

# Wide format for detailed information
nexus node list -o wide
```

## Plugin Development

### Creating a Custom Plugin
```bash
# Plugin directory structure
~/.nexus/plugins/my-plugin/
├── plugin.yaml
└── bin/
    └── nexus-my-plugin
```

### Plugin Specification (`plugin.yaml`)
```yaml
apiVersion: nexus.io/v1
kind: Plugin
metadata:
  name: my-plugin
  version: v1.0.0
spec:
  shortDescription: Custom plugin for specialized tasks
  commands:
  - name: my-command
    shortDescription: Custom command description
  platforms:
  - os: linux
    arch: amd64
    uri: https://releases.example.com/my-plugin-linux-amd64.tar.gz
```

### Plugin Management
```bash
# Install plugin
nexus plugin install my-plugin

# List installed plugins
nexus plugin list

# Update plugin
nexus plugin install my-plugin --version v1.1.0 --force

# Use plugin
nexus my-plugin my-command --arg value
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `NEXUS_CONFIG` | Configuration file path | `~/.nexus/config.yaml` |
| `NEXUS_CONTEXT` | Current context override | From config file |
| `NEXUS_NAMESPACE` | Default namespace | `default` |
| `NEXUS_API_URL` | API server URL override | From config file |
| `NEXUS_TOKEN` | Authentication token | From config file |
| `NEXUS_EDITOR` | Default editor for edit commands | `$EDITOR` or `vim` |
| `NEXUS_NO_COLOR` | Disable colored output | `false` |

## Shell Integration

### Bash Completion
```bash
# Add to ~/.bashrc
source <(nexus completion bash)

# Or install system-wide
nexus completion bash > /etc/bash_completion.d/nexus
```

### Zsh Completion
```bash
# Add to ~/.zshrc
source <(nexus completion zsh)

# Or install system-wide
nexus completion zsh > "${fpath[1]}/_nexus"
```

### Fish Completion
```bash
nexus completion fish > ~/.config/fish/completions/nexus.fish
```

## Troubleshooting

### Common Issues

#### Connection Problems
```bash
# Check cluster connectivity
nexus cluster status

# Verify certificates
nexus security certificate list --expiring-within 7d

# Test network connectivity
nexus debug trace --from local --to cluster-api --port 8443
```

#### Service Issues
```bash
# Check service status
nexus service describe problematic-service

# View recent events
nexus debug events --field-selector involvedObject.name=problematic-service

# Check logs
nexus debug logs service/problematic-service --since 1h
```

#### Performance Issues
```bash
# Check resource usage
nexus debug top nodes
nexus debug top pods --all-namespaces

# Get detailed metrics
nexus metrics get cluster --time-range 24h --metric-name cpu_usage
```

### Debugging Commands
```bash
# Enable verbose logging
nexus --verbose cluster status

# Dry run to preview changes
nexus service update web-app --image new-image:v2.0.0 --dry-run

# Output raw API responses
nexus service list -o json -v 8
```

## Best Practices

### Security
- Always use TLS certificates for production clusters
- Regularly rotate certificates using `nexus security certificate rotate`
- Implement network policies to restrict traffic flow
- Use RBAC to limit user permissions
- Enable audit logging for compliance

### Resource Management
- Set resource requests and limits for all services
- Use horizontal pod autoscaling for variable workloads
- Monitor resource usage with `nexus debug top`
- Implement proper storage classes for different performance needs

### Operational Excellence
- Use configuration files for reproducible deployments
- Implement health checks for all services
- Set up monitoring and alerting
- Regular cluster maintenance and updates
- Backup configurations and persistent data

### Development Workflow
- Use different contexts for different environments
- Implement CI/CD pipelines with nexus CLI
- Use dry-run mode to validate changes
- Tag and version all deployments
- Monitor application performance and logs

## Support and Community

- **Documentation**: https://docs.hypermesh.io
- **GitHub**: https://github.com/hypermesh/nexus
- **Community**: https://community.hypermesh.io
- **Discord**: https://discord.gg/hypermesh
- **Support**: support@hypermesh.io

## Version History

- **v1.0.0**: Initial release with core functionality
- **v1.1.0**: Added advanced networking and security features
- **v1.2.0**: ML-powered scheduling and auto-scaling
- **v1.3.0**: Enhanced observability and debugging tools
- **v2.0.0**: P2P mesh networking and edge computing support

---

This guide provides comprehensive coverage of the nexus CLI capabilities. For the most up-to-date information and additional examples, please refer to the official documentation at https://docs.hypermesh.io.