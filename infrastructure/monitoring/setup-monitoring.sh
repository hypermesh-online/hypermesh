#!/bin/bash

# Native HyperMesh Monitoring Setup Script
# This script configures the built-in monitoring system without external dependencies

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check if running in Kubernetes context
    if kubectl cluster-info &>/dev/null; then
        log_success "Kubernetes cluster is accessible"
    else
        log_error "Cannot access Kubernetes cluster. Please configure kubectl."
        exit 1
    fi

    # Check if hypermesh namespace exists
    if kubectl get namespace hypermesh &>/dev/null; then
        log_success "HyperMesh namespace exists"
    else
        log_warning "Creating hypermesh namespace..."
        kubectl create namespace hypermesh
    fi
}

# Deploy monitoring components
deploy_monitoring() {
    log_info "Deploying native monitoring components..."

    # Create monitoring ConfigMap
    cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: monitoring-config
  namespace: hypermesh
data:
  monitoring.toml: |
    [monitoring]
    enable_ebpf = true
    collection_interval = "10s"
    retention_period = "7d"

    [metrics]
    enable_cpu = true
    enable_memory = true
    enable_network = true
    enable_disk = true
    enable_gpu = true

    [ebpf]
    enable_syscall_tracing = true
    enable_network_tracing = true
    enable_file_tracing = false

    [dashboards]
    refresh_interval = "5s"
    default_time_range = "1h"

    [alerts]
    enable_alerts = true
    alert_check_interval = "30s"

    [[alerts.rules]]
    name = "high_cpu_usage"
    metric = "cpu_usage_percent"
    threshold = 80
    duration = "5m"
    severity = "warning"

    [[alerts.rules]]
    name = "high_memory_usage"
    metric = "memory_usage_percent"
    threshold = 90
    duration = "5m"
    severity = "critical"

    [[alerts.rules]]
    name = "high_error_rate"
    metric = "error_rate"
    threshold = 0.05
    duration = "2m"
    severity = "warning"

    [[alerts.rules]]
    name = "slow_response_time"
    metric = "p99_latency_ms"
    threshold = 1000
    duration = "5m"
    severity = "warning"
EOF

    # Deploy monitoring DaemonSet for eBPF collection
    cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: hypermesh-monitor
  namespace: hypermesh
  labels:
    app: hypermesh-monitor
spec:
  selector:
    matchLabels:
      app: hypermesh-monitor
  template:
    metadata:
      labels:
        app: hypermesh-monitor
    spec:
      hostNetwork: true
      hostPID: true
      serviceAccountName: hypermesh-monitor
      containers:
      - name: monitor
        image: ghcr.io/hypermesh-online/hypermesh:latest
        command: ["hypermesh", "monitor", "--ebpf"]
        securityContext:
          privileged: true
          capabilities:
            add:
            - SYS_ADMIN
            - SYS_RESOURCE
            - SYS_PTRACE
            - NET_ADMIN
            - NET_RAW
        env:
        - name: NODE_NAME
          valueFrom:
            fieldRef:
              fieldPath: spec.nodeName
        - name: MONITORING_MODE
          value: "ebpf"
        volumeMounts:
        - name: sys
          mountPath: /sys
          readOnly: true
        - name: proc
          mountPath: /host/proc
          readOnly: true
        - name: config
          mountPath: /etc/monitoring
        - name: data
          mountPath: /var/lib/hypermesh/monitoring
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
      volumes:
      - name: sys
        hostPath:
          path: /sys
      - name: proc
        hostPath:
          path: /proc
      - name: config
        configMap:
          name: monitoring-config
      - name: data
        hostPath:
          path: /var/lib/hypermesh/monitoring
          type: DirectoryOrCreate
EOF

    # Create ServiceAccount and RBAC
    cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ServiceAccount
metadata:
  name: hypermesh-monitor
  namespace: hypermesh
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: hypermesh-monitor
rules:
- apiGroups: [""]
  resources: ["nodes", "pods", "services", "endpoints", "namespaces"]
  verbs: ["get", "list", "watch"]
- apiGroups: ["apps"]
  resources: ["deployments", "daemonsets", "statefulsets", "replicasets"]
  verbs: ["get", "list", "watch"]
- apiGroups: ["batch"]
  resources: ["jobs", "cronjobs"]
  verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: hypermesh-monitor
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: hypermesh-monitor
subjects:
- kind: ServiceAccount
  name: hypermesh-monitor
  namespace: hypermesh
EOF

    log_success "Monitoring components deployed"
}

# Deploy Nexus UI Dashboard
deploy_nexus_ui() {
    log_info "Deploying Nexus UI Dashboard..."

    cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nexus-ui
  namespace: hypermesh
  labels:
    app: nexus-ui
spec:
  replicas: 2
  selector:
    matchLabels:
      app: nexus-ui
  template:
    metadata:
      labels:
        app: nexus-ui
    spec:
      containers:
      - name: nexus-ui
        image: ghcr.io/hypermesh-online/nexus-ui:latest
        ports:
        - containerPort: 3000
          name: http
        env:
        - name: API_URL
          value: "http://hypermesh.hypermesh.svc.cluster.local:8080"
        - name: MONITORING_URL
          value: "http://hypermesh.hypermesh.svc.cluster.local:9090"
        - name: ENABLE_REAL_TIME
          value: "true"
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
        livenessProbe:
          httpGet:
            path: /
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: nexus-ui
  namespace: hypermesh
  labels:
    app: nexus-ui
spec:
  type: ClusterIP
  selector:
    app: nexus-ui
  ports:
  - name: http
    port: 80
    targetPort: 3000
    protocol: TCP
EOF

    log_success "Nexus UI Dashboard deployed"
}

# Configure ingress for monitoring
configure_ingress() {
    log_info "Configuring ingress for monitoring dashboard..."

    cat <<EOF | kubectl apply -f -
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: monitoring-ingress
  namespace: hypermesh
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - monitoring.hypermesh.online
    secretName: monitoring-tls
  rules:
  - host: monitoring.hypermesh.online
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: nexus-ui
            port:
              number: 80
      - path: /api
        pathType: Prefix
        backend:
          service:
            name: hypermesh
            port:
              number: 8080
      - path: /metrics
        pathType: Prefix
        backend:
          service:
            name: hypermesh
            port:
              number: 9090
EOF

    log_success "Ingress configured"
}

# Setup alerts
setup_alerts() {
    log_info "Setting up alert notifications..."

    # Create alert processor deployment
    cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alert-processor
  namespace: hypermesh
  labels:
    app: alert-processor
spec:
  replicas: 1
  selector:
    matchLabels:
      app: alert-processor
  template:
    metadata:
      labels:
        app: alert-processor
    spec:
      containers:
      - name: processor
        image: ghcr.io/hypermesh-online/hypermesh:latest
        command: ["hypermesh", "alerts", "--processor"]
        env:
        - name: ALERT_WEBHOOK_URL
          valueFrom:
            secretKeyRef:
              name: alert-secrets
              key: webhook-url
              optional: true
        - name: ALERT_EMAIL
          valueFrom:
            secretKeyRef:
              name: alert-secrets
              key: email
              optional: true
        resources:
          requests:
            cpu: 50m
            memory: 64Mi
          limits:
            cpu: 200m
            memory: 256Mi
EOF

    log_success "Alert processor deployed"
}

# Verify monitoring setup
verify_monitoring() {
    log_info "Verifying monitoring setup..."

    # Check if monitoring pods are running
    kubectl wait --for=condition=ready pod -l app=hypermesh-monitor -n hypermesh --timeout=60s || {
        log_warning "Monitor pods not ready yet"
    }

    kubectl wait --for=condition=ready pod -l app=nexus-ui -n hypermesh --timeout=60s || {
        log_warning "Nexus UI pods not ready yet"
    }

    # Get monitoring status
    log_info "Monitoring pod status:"
    kubectl get pods -n hypermesh -l app=hypermesh-monitor

    log_info "Nexus UI status:"
    kubectl get pods -n hypermesh -l app=nexus-ui

    # Port forward for local access
    log_info "Setting up port forwarding for local access..."
    log_info "Dashboard will be available at: http://localhost:3000"
    log_info "Metrics will be available at: http://localhost:9090/metrics"

    kubectl port-forward -n hypermesh service/nexus-ui 3000:80 &
    kubectl port-forward -n hypermesh service/hypermesh 9090:9090 &

    log_success "Monitoring setup complete!"
}

# Main execution
main() {
    log_info "Starting HyperMesh native monitoring setup..."

    check_prerequisites
    deploy_monitoring
    deploy_nexus_ui
    configure_ingress
    setup_alerts
    verify_monitoring

    log_success "Native monitoring system deployed successfully!"
    log_info "Access the dashboard at: http://localhost:3000"
    log_info "For production access: https://monitoring.hypermesh.online"
}

# Run main function
main "$@"