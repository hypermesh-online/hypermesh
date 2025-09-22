# ARCHIVED - Content moved to /hypermesh/docs/cli/

*This file has been consolidated as part of documentation compression.*

**CLI Documentation now located at:**
- `/hypermesh/docs/cli/specification.md`
- `/hypermesh/docs/cli/user-guide.md`
- `/hypermesh/docs/cli/examples/`

See structured CLI documentation in the docs directory.
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
  --storage-class     Storage class for persistent volumes
  --enable-ebpf       Enable eBPF programs (default: true)
  --bootstrap-node    Bootstrap node for joining existing mesh
  --cert-authority    Custom certificate authority
  --config-file       Cluster configuration file
```

#### `nexus cluster list`
Lists all clusters.

```bash
nexus cluster list [options]

Options:
  --all-contexts      Show clusters from all contexts
  --show-labels       Show labels in output
```

#### `nexus cluster describe`
Shows detailed cluster information.

```bash
nexus cluster describe [NAME] [options]

Options:
  --show-events       Include recent events
  --show-metrics      Include performance metrics
  --show-certificates Show certificate information
```

#### `nexus cluster delete`
Deletes a cluster.

```bash
nexus cluster delete [NAME] [options]

Options:
  --force             Force deletion without confirmation
  --preserve-data     Preserve persistent data
  --drain-timeout     Timeout for draining nodes (default: 5m)
```

#### `nexus cluster upgrade`
Upgrades cluster components.

```bash
nexus cluster upgrade [NAME] [options]

Options:
  --version           Target version
  --rolling           Perform rolling upgrade
  --max-unavailable   Max unavailable nodes during upgrade (default: 1)
  --pre-flight-check  Run pre-flight checks only
```

#### `nexus cluster join`
Joins current node to an existing cluster.

```bash
nexus cluster join [CLUSTER_ENDPOINT] [options]

Options:
  --token            Join token
  --certificate      Client certificate path
  --node-role        Node role (worker/controller)
  --node-labels      Node labels (key=value,...)
```

### 2. Node Management (`nexus node`)

#### `nexus node list`
Lists cluster nodes.

```bash
nexus node list [options]

Options:
  --show-resources    Show resource allocation
  --show-conditions   Show node conditions
  --selector          Label selector
```

#### `nexus node describe`
Shows detailed node information.

```bash
nexus node describe [NODE_NAME] [options]

Options:
  --show-pods         Include running pods
  --show-metrics      Include resource metrics
  --show-events       Include recent events
```

#### `nexus node drain`
Drains a node for maintenance.

```bash
nexus node drain [NODE_NAME] [options]

Options:
  --ignore-daemonsets Skip DaemonSet-managed pods
  --delete-local-data Delete pods with local storage
  --force             Force drain
  --grace-period      Grace period for pod termination (default: 30s)
```

#### `nexus node cordon/uncordon`
Marks node as unschedulable/schedulable.

```bash
nexus node cordon [NODE_NAME]
nexus node uncordon [NODE_NAME]
```

### 3. Service Management (`nexus service`)

#### `nexus service deploy`
Deploys a new service.

```bash
nexus service deploy [IMAGE] [options]

Options:
  --name             Service name
  --replicas         Number of replicas (default: 1)
  --port             Port mapping (hostPort:containerPort)
  --env              Environment variables (KEY=value,...)
  --volume           Volume mounts (hostPath:containerPath)
  --labels           Service labels (key=value,...)
  --annotations      Service annotations (key=value,...)
  --config-file      Service configuration file
  --strategy         Deployment strategy (rolling/blue-green/canary)
  --resource-limits  Resource limits (cpu=1,memory=1Gi)
  --resource-requests Resource requests (cpu=100m,memory=128Mi)
  --health-check     Health check endpoint
  --ready-check      Readiness check endpoint
```

#### `nexus service list`
Lists services.

```bash
nexus service list [options]

Options:
  --namespace        Filter by namespace
  --selector         Label selector
  --show-endpoints   Show service endpoints
  --wide             Show additional information
```

#### `nexus service describe`
Shows detailed service information.

```bash
nexus service describe [SERVICE_NAME] [options]

Options:
  --show-events      Include recent events
  --show-metrics     Include performance metrics
  --show-pods        Include pod information
```

#### `nexus service scale`
Scales service replicas.

```bash
nexus service scale [SERVICE_NAME] --replicas [COUNT] [options]

Options:
  --replicas         Target replica count
  --timeout          Scaling timeout (default: 5m)
  --wait             Wait for scaling to complete
```

#### `nexus service update`
Updates service configuration.

```bash
nexus service update [SERVICE_NAME] [options]

Options:
  --image            New container image
  --set              Set configuration values (key=value,...)
  --config-file      Updated configuration file
  --strategy         Update strategy (rolling/recreate)
  --max-unavailable  Max unavailable during update
  --max-surge        Max surge during update
```

#### `nexus service rollback`
Rolls back service to previous version.

```bash
nexus service rollback [SERVICE_NAME] [options]

Options:
  --revision         Target revision (default: previous)
  --timeout          Rollback timeout
```

#### `nexus service delete`
Deletes a service.

```bash
nexus service delete [SERVICE_NAME] [options]

Options:
  --cascade          Delete dependent resources
  --grace-period     Grace period for termination
  --timeout          Deletion timeout
```

### 4. Workload Management (`nexus workload`)

#### `nexus workload run`
Runs a one-time workload.

```bash
nexus workload run [IMAGE] [options]

Options:
  --name             Workload name
  --restart-policy   Restart policy (Never/OnFailure/Always)
  --command          Override container command
  --args             Override container arguments
  --env              Environment variables
  --volume           Volume mounts
  --ttl              Time-to-live after completion
```

#### `nexus workload create job`
Creates a batch job.

```bash
nexus workload create job [NAME] [options]

Options:
  --image            Container image
  --parallelism      Parallel executions
  --completions      Required completions
  --backoff-limit    Failure retry limit
  --active-deadline  Active deadline seconds
```

#### `nexus workload create cronjob`
Creates a scheduled job.

```bash
nexus workload create cronjob [NAME] [options]

Options:
  --schedule         Cron schedule expression
  --image            Container image
  --concurrency-policy Concurrency policy (Allow/Forbid/Replace)
  --history-limit    History limit for completed jobs
```

### 5. Network Management (`nexus network`)

#### `nexus network list`
Lists network policies and configurations.

```bash
nexus network list [options]

Options:
  --type             Filter by type (policy/mesh/ingress)
  --namespace        Filter by namespace
```

#### `nexus network policy create`
Creates network security policy.

```bash
nexus network policy create [NAME] [options]

Options:
  --allow-ingress    Ingress rules (from=selector,ports=80,443)
  --allow-egress     Egress rules (to=selector,ports=80,443)
  --deny-all         Deny all traffic by default
  --config-file      Policy configuration file
```

#### `nexus network ingress create`
Creates ingress configuration.

```bash
nexus network ingress create [NAME] [options]

Options:
  --host             Host/domain name
  --path             Path rules (path=/, service=name:port)
  --tls              TLS configuration (secret=name)
  --class            Ingress class
  --annotations      Ingress annotations
```

#### `nexus network mesh configure`
Configures service mesh settings.

```bash
nexus network mesh configure [options]

Options:
  --enable-mtls      Enable mutual TLS
  --traffic-policy   Traffic management policy
  --retry-policy     Retry configuration
  --circuit-breaker  Circuit breaker settings
  --load-balancer    Load balancing algorithm
```

### 6. Security Management (`nexus security`)

#### `nexus security certificate list`
Lists certificates.

```bash
nexus security certificate list [options]

Options:
  --type             Certificate type (client/server/ca)
  --expiring-within  Show certificates expiring within duration
```

#### `nexus security certificate create`
Creates a new certificate.

```bash
nexus security certificate create [NAME] [options]

Options:
  --type             Certificate type (client/server/ca)
  --hosts            Subject alternative names
  --validity         Validity period (default: 8760h)
  --ca               CA certificate for signing
  --key-size         Key size in bits (default: 2048)
```

#### `nexus security certificate rotate`
Rotates certificates.

```bash
nexus security certificate rotate [options]

Options:
  --type             Certificate type to rotate
  --all              Rotate all certificates
  --force            Force rotation without confirmation
```

#### `nexus security policy create`
Creates security policy.

```bash
nexus security policy create [NAME] [options]

Options:
  --type             Policy type (rbac/pod/network)
  --rules            Policy rules
  --subjects         Policy subjects
  --config-file      Policy configuration file
```

### 7. Configuration Management (`nexus config`)

#### `nexus config get-contexts`
Lists available contexts.

```bash
nexus config get-contexts
```

#### `nexus config use-context`
Sets current context.

```bash
nexus config use-context [CONTEXT_NAME]
```

#### `nexus config set`
Sets configuration values.

```bash
nexus config set [KEY] [VALUE] [options]

Options:
  --context          Set for specific context
  --global           Set globally across all contexts
```

#### `nexus config view`
Displays configuration.

```bash
nexus config view [options]

Options:
  --minify           Minify output
  --flatten          Flatten nested values
```

### 8. Resource Management (`nexus resource`)

#### `nexus resource quota list`
Lists resource quotas.

```bash
nexus resource quota list [options]

Options:
  --namespace        Filter by namespace
  --show-usage       Show current usage
```

#### `nexus resource quota create`
Creates resource quota.

```bash
nexus resource quota create [NAME] [options]

Options:
  --cpu              CPU limit
  --memory           Memory limit
  --storage          Storage limit
  --pods             Pod count limit
  --services         Service count limit
```

### 9. Monitoring and Debugging (`nexus debug`)

#### `nexus debug logs`
Retrieves logs.

```bash
nexus debug logs [RESOURCE] [options]

Options:
  --follow, -f       Follow log output
  --tail             Number of lines to show from end
  --since            Show logs since timestamp
  --container        Specific container in pod
  --previous         Show logs from previous instance
```

#### `nexus debug exec`
Executes command in container.

```bash
nexus debug exec [POD] [options] -- [COMMAND]

Options:
  --container, -c    Container name
  --stdin, -i        Pass stdin to container
  --tty, -t          Allocate pseudo-TTY
```

#### `nexus debug port-forward`
Forwards local port to pod.

```bash
nexus debug port-forward [POD] [LOCAL_PORT]:[REMOTE_PORT] [options]

Options:
  --address          Local address to bind (default: localhost)
```

#### `nexus debug proxy`
Runs proxy to cluster API.

```bash
nexus debug proxy [options]

Options:
  --port             Local port (default: 8080)
  --address          Local address (default: 127.0.0.1)
```

#### `nexus debug events`
Shows cluster events.

```bash
nexus debug events [options]

Options:
  --watch            Watch for new events
  --since            Show events since timestamp
  --field-selector   Field selector
```

#### `nexus debug top`
Shows resource usage.

```bash
nexus debug top nodes [options]
nexus debug top pods [options]

Options:
  --sort-by          Sort by field (cpu/memory)
  --no-headers       Don't print headers
```

### 10. Data and Storage (`nexus storage`)

#### `nexus storage list`
Lists storage resources.

```bash
nexus storage list [TYPE] [options]

Types: volumes, claims, classes, snapshots

Options:
  --namespace        Filter by namespace
  --show-capacity    Show capacity information
```

#### `nexus storage create`
Creates storage resources.

```bash
nexus storage create [TYPE] [NAME] [options]

Options:
  --size             Volume size
  --storage-class    Storage class
  --access-mode      Access mode (ReadWriteOnce/ReadOnlyMany/ReadWriteMany)
  --config-file      Storage configuration file
```

#### `nexus storage snapshot`
Creates volume snapshot.

```bash
nexus storage snapshot [VOLUME] [options]

Options:
  --name             Snapshot name
  --description      Snapshot description
```

### 11. Metrics and Performance (`nexus metrics`)

#### `nexus metrics get`
Retrieves performance metrics.

```bash
nexus metrics get [RESOURCE] [options]

Options:
  --metric-name      Specific metric to retrieve
  --time-range       Time range (1h/24h/7d)
  --aggregation      Aggregation method (avg/min/max/sum)
  --format           Output format (table/json/csv)
```

#### `nexus metrics dashboard`
Opens metrics dashboard.

```bash
nexus metrics dashboard [options]

Options:
  --port             Local port for dashboard
  --no-browser       Don't open browser automatically
```

### 12. Plugin Management (`nexus plugin`)

#### `nexus plugin list`
Lists installed plugins.

```bash
nexus plugin list [options]

Options:
  --available        Show available plugins
  --updates          Check for plugin updates
```

#### `nexus plugin install`
Installs a plugin.

```bash
nexus plugin install [PLUGIN] [options]

Options:
  --version          Plugin version
  --force            Force reinstallation
```

## Configuration File Format

### Main Configuration (`~/.nexus/config.yaml`)

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
- name: development
  context:
    cluster: dev-cluster
    namespace: default
    user: dev-user

clusters:
- name: prod-cluster
  cluster:
    server: https://nexus.production.example.com:8080
    certificate-authority: /path/to/ca.crt
    insecure-skip-tls-verify: false
- name: dev-cluster
  cluster:
    server: https://nexus.dev.example.com:8080
    certificate-authority-data: LS0tLS1CRUdJTi...

users:
- name: admin
  user:
    client-certificate: /path/to/admin.crt
    client-key: /path/to/admin.key
- name: dev-user
  user:
    token: eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...

preferences:
  colors: true
  editor: vim
  output: table
```

### Service Specification (`service.yaml`)

```yaml
apiVersion: nexus.io/v1
kind: Service
metadata:
  name: web-app
  labels:
    app: web-app
    version: v1.0.0
  annotations:
    nexus.io/description: "Web application service"
spec:
  replicas: 3
  selector:
    matchLabels:
      app: web-app
  template:
    metadata:
      labels:
        app: web-app
        version: v1.0.0
    spec:
      containers:
      - name: web
        image: nginx:1.20
        ports:
        - containerPort: 80
          name: http
        env:
        - name: ENV
          value: production
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
        livenessProbe:
          httpGet:
            path: /health
            port: 80
          initialDelaySeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 80
          initialDelaySeconds: 5
      strategy:
        type: RollingUpdate
        rollingUpdate:
          maxUnavailable: 1
          maxSurge: 1
  networking:
    ingress:
      hosts:
      - host: web-app.example.com
        paths:
        - path: /
          service:
            name: web-app
            port: 80
      tls:
      - secretName: web-app-tls
        hosts:
        - web-app.example.com
    mesh:
      mtls:
        mode: STRICT
      trafficPolicy:
        loadBalancer:
          simple: ROUND_ROBIN
        connectionPool:
          tcp:
            maxConnections: 100
        outlierDetection:
          consecutiveErrors: 5
          interval: 30s
          baseEjectionTime: 30s
```

### Cluster Specification (`cluster.yaml`)

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
    zones:
    - us-west-2a
    - us-west-2b
    - us-west-2c
  
  nodes:
    controller:
      count: 3
      instanceType: m5.large
      storage:
        type: gp3
        size: 100Gi
    worker:
      count: 5
      instanceType: m5.xlarge
      storage:
        type: gp3
        size: 200Gi
      autoScaling:
        enabled: true
        minNodes: 3
        maxNodes: 20
  
  networking:
    mode: p2p
    cidr: 10.0.0.0/16
    serviceCidr: 10.1.0.0/16
    podCidr: 10.2.0.0/16
    mesh:
      enabled: true
      mtls: true
      policy: strict
  
  storage:
    defaultClass: fast-ssd
    classes:
    - name: fast-ssd
      type: gp3
      parameters:
        type: gp3
        iops: "3000"
        throughput: "125"
  
  security:
    rbac:
      enabled: true
    networkPolicy:
      enabled: true
    podSecurityPolicy:
      enabled: true
    encryption:
      atRest: true
      inTransit: true
  
  monitoring:
    enabled: true
    retention: 30d
    alerting:
      enabled: true
      slackWebhook: https://hooks.slack.com/...
  
  backup:
    enabled: true
    schedule: "0 2 * * *"
    retention: 30d
    destination: s3://backups/nexus-cluster
```

## Plugin Architecture

The nexus CLI supports plugins for extending functionality:

### Plugin Structure
```
~/.nexus/plugins/
├── my-plugin/
│   ├── plugin.yaml
│   └── bin/
│       └── nexus-my-plugin
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
  longDescription: |
    Detailed description of what this plugin does
    and how to use it.
  homepage: https://github.com/example/nexus-my-plugin
  
  commands:
  - name: my-command
    shortDescription: Custom command description
    usage: nexus my-plugin my-command [options]
  
  platforms:
  - os: linux
    arch: amd64
    uri: https://releases.example.com/my-plugin-linux-amd64.tar.gz
  - os: darwin
    arch: amd64
    uri: https://releases.example.com/my-plugin-darwin-amd64.tar.gz
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `NEXUS_CONFIG` | Config file path | `~/.nexus/config.yaml` |
| `NEXUS_CONTEXT` | Current context | From config file |
| `NEXUS_NAMESPACE` | Default namespace | `default` |
| `NEXUS_API_URL` | API server URL | From config file |
| `NEXUS_TOKEN` | Authentication token | From config file |
| `NEXUS_EDITOR` | Default editor for edit commands | `$EDITOR` or `vim` |
| `NEXUS_NO_COLOR` | Disable colored output | `false` |
| `NEXUS_COMPLETION` | Enable shell completion | `true` |

## Shell Completion

Generate completion scripts for various shells:

```bash
# Bash
nexus completion bash > /etc/bash_completion.d/nexus

# Zsh
nexus completion zsh > "${fpath[1]}/_nexus"

# Fish
nexus completion fish > ~/.config/fish/completions/nexus.fish

# PowerShell
nexus completion powershell > nexus.ps1
```

## Error Handling

The CLI provides structured error messages with:
- Error codes for programmatic handling
- Suggested remediation steps
- Links to documentation
- Debug information when verbose mode is enabled

## Examples

### Complete Deployment Workflow

```bash
# 1. Create a new cluster
nexus cluster create production --nodes 5 --provider aws --region us-west-2

# 2. Verify cluster is ready
nexus cluster describe production

# 3. Deploy a service
nexus service deploy nginx:1.20 --name web-app --replicas 3 --port 80

# 4. Scale the service
nexus service scale web-app --replicas 5

# 5. Create ingress
nexus network ingress create web-app-ingress --host myapp.com --path "/" --service web-app:80

# 6. Monitor logs
nexus debug logs service/web-app -f

# 7. Check metrics
nexus metrics get service/web-app --time-range 1h
```

This specification provides a comprehensive foundation for a production-ready distributed infrastructure management CLI that aligns with HyperMesh's vision of secure, high-performance, P2P-enabled cloud infrastructure.