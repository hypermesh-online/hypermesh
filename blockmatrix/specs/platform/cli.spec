# Nexus CLI Specification
# 
# IMPORTANT: This specification is maintained exclusively by @agent-scribe
# All modifications must go through scribe agent - DO NOT edit directly
#
# Component: Nexus CLI - HyperMesh Management Interface
# Version: 1.0

## Overview

The `nexus` CLI is the primary management interface for HyperMesh distributed infrastructure, providing comprehensive cluster management, service orchestration, debugging, and operational capabilities.

## Design Principles

1. **Intuitive Commands**: Follow kubectl-like conventions where appropriate
2. **Rich Output Formats**: Support table, JSON, YAML outputs with structured data
3. **Extensive Debugging**: Built-in debugging and troubleshooting tools
4. **Configuration Management**: Support for multiple contexts and profiles
5. **P2P First**: Native support for distributed, P2P operations
6. **Security by Default**: All operations secured with certificates and encryption

## Command Structure

```
nexus [global-options] <command> <subcommand> [options] [arguments]
```

## Global Options

| Flag | Description | Default |
|------|-------------|---------|
| `--config, -c` | Config file path | `~/.nexus/config.yaml` |
| `--context` | Nexus context to use | `current-context` |
| `--output, -o` | Output format (table/json/yaml/wide) | `table` |
| `--verbose, -v` | Verbosity level (stackable) | `0` |
| `--no-headers` | Suppress headers in table output | `false` |
| `--dry-run` | Preview changes without applying | `false` |
| `--timeout` | Operation timeout | `30s` |

## Core Commands

### 1. Cluster Management (`nexus cluster`)

#### `nexus cluster create`
Creates a new HyperMesh cluster.

```bash
nexus cluster create [NAME] [options]

Options:
  --nodes, -n          Number of initial nodes (default: 3)
  --node-type         Node instance type (default: auto)
  --provider          Cloud provider (aws/gcp/azure/bare-metal)
  --region            Cloud region
  --zones             Availability zones (comma-separated)
  --network-mode      Network mode (p2p/hybrid/traditional)
  --storage-class     Default storage class
  --version           HyperMesh version to install
  --config            Cluster configuration file
  --wait              Wait for cluster ready state
```

#### `nexus cluster list`
Lists all managed clusters.

```bash
nexus cluster list [options]

Options:
  --provider          Filter by cloud provider
  --status           Filter by cluster status
  --labels           Filter by labels (key=value)
```

#### `nexus cluster get`
Get detailed cluster information.

```bash
nexus cluster get [CLUSTER_NAME] [options]

Options:
  --show-nodes       Include node details
  --show-services    Include service details
  --show-metrics     Include performance metrics
```

### 2. Node Management (`nexus node`)

#### `nexus node list`
Lists cluster nodes.

```bash
nexus node list [options]

Options:
  --cluster          Cluster name filter
  --status          Node status filter (ready/not-ready/unknown)
  --zone            Availability zone filter
  --role            Node role filter (master/worker/edge)
```

#### `nexus node describe`
Get detailed node information.

```bash
nexus node describe [NODE_NAME] [options]

Options:
  --show-pods        Show running pods
  --show-metrics     Show resource metrics
  --show-events      Show recent events
```

#### `nexus node cordon/uncordon`
Mark node as (un)schedulable.

```bash
nexus node cordon [NODE_NAME]
nexus node uncordon [NODE_NAME]
```

### 3. Service Management (`nexus service`)

#### `nexus service deploy`
Deploy a service to the cluster.

```bash
nexus service deploy [SERVICE_NAME] [options]

Options:
  --image            Container image
  --replicas         Number of replicas (default: 1)
  --port             Service port
  --env              Environment variables (key=value)
  --cpu              CPU resource request/limit
  --memory           Memory resource request/limit
  --config           Service configuration file
  --namespace        Target namespace
  --labels           Service labels (key=value)
```

#### `nexus service scale`
Scale service replicas.

```bash
nexus service scale [SERVICE_NAME] --replicas [COUNT] [options]

Options:
  --namespace        Service namespace
  --wait            Wait for scaling completion
```

#### `nexus service update`
Update service configuration.

```bash
nexus service update [SERVICE_NAME] [options]

Options:
  --image           New container image
  --env             Update environment variables
  --config          Updated configuration file
  --strategy        Update strategy (rolling/blue-green)
```

### 4. P2P Network Management (`nexus p2p`)

#### `nexus p2p status`
Show P2P network status.

```bash
nexus p2p status [options]

Options:
  --cluster         Cluster name
  --detailed        Show detailed peer information
  --topology        Show network topology
```

#### `nexus p2p peers`
List P2P network peers.

```bash
nexus p2p peers [options]

Options:
  --node           Filter by source node
  --connected      Show only connected peers
  --distance       Show network distance metrics
```

#### `nexus p2p connect`
Establish P2P connection to peer.

```bash
nexus p2p connect [PEER_ID] [options]

Options:
  --address        Peer network address
  --protocol       Connection protocol
  --timeout        Connection timeout
```

### 5. Security Management (`nexus security`)

#### `nexus security cert`
Certificate management commands.

```bash
nexus security cert list                    # List certificates
nexus security cert generate [NAME]         # Generate certificate
nexus security cert rotate [CERT_NAME]      # Rotate certificate
nexus security cert verify [CERT_NAME]      # Verify certificate
```

#### `nexus security policy`
Security policy management.

```bash
nexus security policy create [POLICY_NAME] --config [FILE]
nexus security policy list
nexus security policy apply [POLICY_NAME]
nexus security policy delete [POLICY_NAME]
```

#### `nexus security scan`
Security vulnerability scanning.

```bash
nexus security scan [TARGET] [options]

Options:
  --type           Scan type (container/network/config)
  --severity       Minimum severity level
  --format         Output format (table/json/sarif)
```

### 6. Monitoring and Observability (`nexus monitor`)

#### `nexus monitor metrics`
View cluster and service metrics.

```bash
nexus monitor metrics [TARGET] [options]

Options:
  --metric-name    Specific metric to query
  --time-range     Time range for metrics (5m/1h/24h)
  --node           Filter by node
  --service        Filter by service
  --namespace      Filter by namespace
```

#### `nexus monitor logs`
View service and system logs.

```bash
nexus monitor logs [TARGET] [options]

Options:
  --follow, -f     Follow log output
  --tail           Number of lines to tail
  --since          Show logs since timestamp
  --container      Specific container logs
  --previous       Show previous container logs
```

#### `nexus monitor events`
View cluster events.

```bash
nexus monitor events [options]

Options:
  --watch         Watch for new events
  --namespace     Filter by namespace
  --type          Filter by event type
  --since         Events since timestamp
```

### 7. Configuration Management (`nexus config`)

#### `nexus config view`
Display configuration.

```bash
nexus config view [options]

Options:
  --raw           Show raw configuration
  --context       Show specific context
  --flatten       Flatten nested configuration
```

#### `nexus config set-context`
Set current context.

```bash
nexus config set-context [CONTEXT_NAME]
```

#### `nexus config use-context`
Switch to context.

```bash
nexus config use-context [CONTEXT_NAME]
```

### 8. Debugging Commands (`nexus debug`)

#### `nexus debug network`
Network connectivity debugging.

```bash
nexus debug network [TARGET] [options]

Options:
  --trace-route    Show network path
  --dns-lookup     Test DNS resolution  
  --port-check     Check port connectivity
  --bandwidth      Test bandwidth
```

#### `nexus debug performance`
Performance analysis and debugging.

```bash
nexus debug performance [TARGET] [options]

Options:
  --cpu           CPU utilization analysis
  --memory        Memory usage analysis
  --io            I/O performance analysis
  --network       Network performance analysis
  --duration      Analysis duration
```

#### `nexus debug trace`
Distributed tracing and debugging.

```bash
nexus debug trace [SERVICE] [options]

Options:
  --trace-id      Specific trace ID
  --span-id       Specific span ID
  --duration      Trace duration
  --sample-rate   Trace sampling rate
```

## Configuration File Structure

### Main Configuration (~/.nexus/config.yaml)
```yaml
apiVersion: nexus.hypermesh.io/v1
kind: Config
current-context: production

contexts:
- name: production
  context:
    cluster: prod-cluster
    user: admin
    namespace: default
    
- name: development
  context:
    cluster: dev-cluster
    user: developer
    namespace: dev

clusters:
- name: prod-cluster
  cluster:
    server: https://nexus.example.com:6443
    certificate-authority: /path/to/ca.crt
    
users:
- name: admin
  user:
    client-certificate: /path/to/admin.crt
    client-key: /path/to/admin.key
```

## Output Formats

### Table Format (Default)
```
NAME          STATUS    NODES   VERSION   PROVIDER   REGION
prod-cluster  Ready     5       v1.0.0    aws        us-west-2
dev-cluster   Pending   3       v1.0.0    gcp        us-central1
```

### JSON Format
```json
{
  "clusters": [
    {
      "name": "prod-cluster",
      "status": "Ready",
      "nodes": 5,
      "version": "v1.0.0",
      "provider": "aws",
      "region": "us-west-2"
    }
  ]
}
```

### YAML Format
```yaml
clusters:
- name: prod-cluster
  status: Ready
  nodes: 5
  version: v1.0.0
  provider: aws
  region: us-west-2
```

## Error Handling

### Exit Codes
- `0` - Success
- `1` - General error
- `2` - Invalid arguments
- `3` - Connection error
- `4` - Authentication error
- `5` - Authorization error
- `6` - Resource not found
- `7` - Timeout error

### Error Output Format
```json
{
  "error": {
    "code": "CLUSTER_NOT_FOUND",
    "message": "Cluster 'test-cluster' not found",
    "details": {
      "cluster": "test-cluster",
      "available_clusters": ["prod-cluster", "dev-cluster"]
    }
  }
}
```

## Shell Integration

### Bash Completion
```bash
source <(nexus completion bash)
```

### Zsh Completion
```bash
source <(nexus completion zsh)
```

### Fish Completion
```bash
nexus completion fish | source
```

## Plugin System

### Plugin Directory Structure
```
~/.nexus/plugins/
├── kubectl-nexus/          # kubectl integration plugin
├── prometheus-nexus/       # Prometheus integration plugin
└── custom-plugin/          # Custom user plugin
```

### Plugin Configuration
```yaml
plugins:
- name: kubectl-nexus
  enabled: true
  config:
    kubectl_path: /usr/local/bin/kubectl
    
- name: prometheus-nexus
  enabled: true
  config:
    prometheus_url: http://prometheus.example.com:9090
```

This specification defines the complete CLI interface for HyperMesh management, providing a comprehensive, intuitive, and powerful command-line experience for distributed infrastructure operations.