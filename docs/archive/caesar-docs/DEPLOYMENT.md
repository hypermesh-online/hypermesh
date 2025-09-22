# Deployment Guide

## Comprehensive Stripe Connect Integration Deployment

This guide covers the complete deployment of the Caesar Token Stripe Connect integration, from development setup to production deployment.

## 🚀 Quick Deploy with Docker

### Prerequisites
- Docker 20.10+
- Docker Compose v2
- SSL certificates (for production)
- Stripe account with Connect enabled
- AWS account (for document storage)

### 1. Clone and Setup

```bash
cd /home/persist/repos/work/vazio/caesar-token/stripe-gateway

# Create environment file
cp .env.example .env

# Edit configuration
nano .env
```

### 2. Required Environment Variables

```bash
# Stripe Configuration (REQUIRED)
STRIPE_PUBLISHABLE_KEY=pk_live_...
STRIPE_SECRET_KEY=sk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_CONNECT_CLIENT_ID=ca_...

# Security (REQUIRED)
JWT_SECRET=your-256-bit-secret-key
BCRYPT_ROUNDS=12

# Database (REQUIRED)
DATABASE_URL=postgresql://gateway:secure_password@postgres:5432/gateway_stripe
REDIS_URL=redis://:redis_password@redis:6379

# LayerZero Integration (REQUIRED)
LAYERZERO_ENDPOINT_V2=0x1a44076050125825900e736c501f859c50fE728c
GATE_TOKEN_CONTRACT=0x...
USDC_CONTRACT=0xA0b86a33E6411a45C28a4b6a7a7e8F4F8Fcd2Ae1
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/your-project-id
DEPLOYER_PRIVATE_KEY=0x...

# KYC/Compliance (REQUIRED)
JUMIO_API_TOKEN=your-jumio-token
JUMIO_API_SECRET=your-jumio-secret
SANCTIONS_SCREENING_API_KEY=your-sanctions-api-key

# AWS Document Storage (REQUIRED)
AWS_ACCESS_KEY_ID=your-aws-access-key
AWS_SECRET_ACCESS_KEY=your-aws-secret-key
S3_BUCKET_NAME=caesar-token-documents
AWS_REGION=us-east-1
```

### 3. Production Deployment

```bash
# Build and start all services
docker-compose up -d

# Check service health
docker-compose ps
docker-compose logs gateway-stripe

# Verify endpoints
curl http://localhost:9292/health
```

### 4. SSL Setup (Production)

```bash
# Create SSL directory
mkdir -p nginx/ssl

# Copy your SSL certificates
cp your-domain.crt nginx/ssl/
cp your-domain.key nginx/ssl/

# Update nginx configuration
nano nginx/nginx.conf
```

## 🏗️ Architecture Components

### Core Services

| Service | Port | Description |
|---------|------|-------------|
| `gateway-stripe` | 9292 | Main API service |
| `postgres` | 5432 | Primary database |
| `redis` | 6379 | Cache and sessions |
| `nginx` | 80/443 | Reverse proxy |

### Monitoring Stack

| Service | Port | Description |
|---------|------|-------------|
| `prometheus` | 9090 | Metrics collection |
| `grafana` | 3000 | Dashboards |
| `elasticsearch` | 9200 | Log storage |
| `kibana` | 5601 | Log analysis |

## 🔧 Configuration Details

### Database Schema

```sql
-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    wallet_address VARCHAR(42) NOT NULL,
    kyc_status VARCHAR(20) DEFAULT 'PENDING',
    kyc_level INTEGER DEFAULT 0,
    risk_score INTEGER DEFAULT 0,
    stripe_customer_id VARCHAR(50),
    stripe_connect_account_id VARCHAR(50),
    compliance_flags TEXT[],
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Transactions table
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    type VARCHAR(20) NOT NULL,
    status VARCHAR(20) DEFAULT 'PENDING',
    fiat_amount DECIMAL(15,2),
    crypto_amount DECIMAL(36,18),
    currency VARCHAR(3),
    crypto_currency VARCHAR(10),
    exchange_rate DECIMAL(20,8),
    fees JSONB,
    stripe_payment_intent_id VARCHAR(50),
    blockchain_tx_hash VARCHAR(66),
    layerzero_tx_hash VARCHAR(66),
    source_chain INTEGER,
    destination_chain INTEGER,
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- KYC documents table
CREATE TABLE kyc_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    type VARCHAR(20) NOT NULL,
    status VARCHAR(20) DEFAULT 'UPLOADED',
    s3_key VARCHAR(500) NOT NULL,
    verification_result JSONB,
    uploaded_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP
);

-- Compliance events table
CREATE TABLE compliance_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(10) NOT NULL,
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);
```

### Nginx Configuration

```nginx
upstream gateway_backend {
    server gateway-stripe:9292;
}

server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;
    
    ssl_certificate /etc/nginx/ssl/your-domain.crt;
    ssl_certificate_key /etc/nginx/ssl/your-domain.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    
    location / {
        proxy_pass http://gateway_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Stripe webhook specific settings
        proxy_read_timeout 60s;
        proxy_send_timeout 60s;
        client_max_body_size 10m;
    }
    
    location /health {
        access_log off;
        proxy_pass http://gateway_backend;
    }
}
```

## 🛡️ Security Configuration

### Environment Security

```bash
# Generate secure JWT secret
openssl rand -hex 32

# Generate secure database password
openssl rand -base64 32

# Set proper file permissions
chmod 600 .env
chmod 600 nginx/ssl/*
```

### Firewall Rules

```bash
# Allow only necessary ports
ufw allow 22    # SSH
ufw allow 80    # HTTP
ufw allow 443   # HTTPS
ufw deny 5432   # Block direct database access
ufw deny 6379   # Block direct Redis access
ufw enable
```

### Database Security

```sql
-- Create read-only user for monitoring
CREATE USER monitoring WITH PASSWORD 'secure_monitoring_password';
GRANT CONNECT ON DATABASE gateway_stripe TO monitoring;
GRANT USAGE ON SCHEMA public TO monitoring;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO monitoring;

-- Create backup user
CREATE USER backup WITH PASSWORD 'secure_backup_password';
GRANT CONNECT ON DATABASE gateway_stripe TO backup;
GRANT USAGE ON SCHEMA public TO backup;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backup;
```

## 📊 Monitoring Setup

### Prometheus Configuration

```yaml
# monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "alert_rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  - job_name: 'gateway-stripe'
    static_configs:
      - targets: ['gateway-stripe:9292']
    metrics_path: '/metrics'
    scrape_interval: 10s

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres:5432']
    
  - job_name: 'redis'
    static_configs:
      - targets: ['redis:6379']
```

### Grafana Dashboard

Access Grafana at `http://localhost:3000` (admin/admin123)

Key dashboards include:
- **System Overview**: CPU, memory, disk usage
- **API Metrics**: Request rate, response time, error rate
- **Business Metrics**: Transaction volume, success rates
- **Compliance Dashboard**: KYC status, risk distribution
- **Reserve Monitoring**: Cross-chain balances, peg status

### Alert Rules

```yaml
# monitoring/alert_rules.yml
groups:
- name: gateway-stripe-alerts
  rules:
  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      
  - alert: PegDeviation
    expr: abs(gate_peg_ratio - 1.0) > 0.02
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "GATE peg deviation detected"
      
  - alert: DatabaseConnections
    expr: postgres_connections_active > 80
    for: 1m
    labels:
      severity: warning
    annotations:
      summary: "High database connection usage"
```

## 🔄 Backup and Recovery

### Automated Backups

```bash
# Database backup script
#!/bin/bash
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
pg_dump -h localhost -U gateway gateway_stripe | gzip > backups/gateway_stripe_$TIMESTAMP.sql.gz

# Keep only last 30 days
find backups/ -name "*.sql.gz" -mtime +30 -delete

# Upload to S3
aws s3 cp backups/gateway_stripe_$TIMESTAMP.sql.gz s3://gateway-backups/database/
```

### Redis Backup

```bash
# Create Redis backup
docker exec redis redis-cli BGSAVE
docker cp redis:/data/dump.rdb backups/redis_$(date +%Y%m%d_%H%M%S).rdb
```

### Recovery Procedures

```bash
# Database recovery
gunzip < backups/gateway_stripe_TIMESTAMP.sql.gz | psql -h localhost -U gateway gateway_stripe

# Redis recovery
docker cp backups/redis_TIMESTAMP.rdb redis:/data/dump.rdb
docker restart redis
```

## 🚀 Production Checklist

### Pre-deployment

- [ ] Environment variables configured
- [ ] SSL certificates installed
- [ ] Database migrations run
- [ ] KYC provider configured
- [ ] Stripe webhooks configured
- [ ] AWS S3 bucket created
- [ ] LayerZero contracts deployed
- [ ] Monitoring stack configured

### Security

- [ ] Firewall rules configured
- [ ] Strong passwords generated
- [ ] File permissions secured
- [ ] Rate limiting enabled
- [ ] CORS properly configured
- [ ] Input validation tested
- [ ] SQL injection tests passed

### Testing

- [ ] Unit tests passing (>80% coverage)
- [ ] Integration tests passing
- [ ] Load testing completed
- [ ] Security scanning completed
- [ ] Penetration testing completed
- [ ] Compliance audit completed

### Monitoring

- [ ] Health checks working
- [ ] Logging configured
- [ ] Metrics collection active
- [ ] Alerts configured
- [ ] Dashboard accessible
- [ ] Backup procedures tested

## 🔧 Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
docker-compose logs gateway-stripe

# Check configuration
docker-compose config

# Restart services
docker-compose restart gateway-stripe
```

#### Database Connection Issues

```bash
# Check database status
docker-compose logs postgres

# Test connection
docker exec postgres psql -U gateway -d gateway_stripe -c "SELECT 1;"
```

#### Stripe Integration Issues

```bash
# Check webhook endpoint
curl -X POST http://localhost:9292/webhooks/stripe

# Verify webhook secret
# Check Stripe dashboard for failed webhooks
```

#### Performance Issues

```bash
# Check resource usage
docker stats

# Analyze slow queries
docker exec postgres psql -U gateway -d gateway_stripe -c "
SELECT query, calls, total_time, mean_time 
FROM pg_stat_statements 
ORDER BY mean_time DESC LIMIT 10;"
```

### Log Analysis

```bash
# Application logs
docker-compose logs -f gateway-stripe

# Database logs
docker-compose logs -f postgres

# Nginx access logs
docker-compose logs -f nginx

# Search for errors
docker-compose logs gateway-stripe | grep -i error
```

## 📈 Scaling

### Horizontal Scaling

```yaml
# docker-compose.scale.yml
version: '3.8'
services:
  gateway-stripe:
    deploy:
      replicas: 3
    environment:
      - NODE_ENV=production
```

### Load Balancing

```nginx
upstream gateway_backend {
    server gateway-stripe-1:9292;
    server gateway-stripe-2:9292;
    server gateway-stripe-3:9292;
}
```

### Database Scaling

```bash
# Read replicas
docker run -d --name postgres-replica \
  -e POSTGRES_PASSWORD=gateway123 \
  -e PGUSER=replica \
  postgres:15-alpine
```

## 🔐 Compliance Deployment

### Data Retention

```sql
-- Automated data retention
CREATE OR REPLACE FUNCTION cleanup_old_data()
RETURNS void AS $$
BEGIN
    -- Delete old compliance events (2 years)
    DELETE FROM compliance_events 
    WHERE created_at < NOW() - INTERVAL '2 years';
    
    -- Archive old transactions (5 years)
    INSERT INTO transactions_archive 
    SELECT * FROM transactions 
    WHERE created_at < NOW() - INTERVAL '5 years';
    
    DELETE FROM transactions 
    WHERE created_at < NOW() - INTERVAL '5 years';
END;
$$ LANGUAGE plpgsql;

-- Schedule cleanup
SELECT cron.schedule('data-cleanup', '0 2 * * 0', 'SELECT cleanup_old_data();');
```

### Audit Logging

```javascript
// Enhanced audit logging
const auditLog = {
  userId: req.user.id,
  action: 'KYC_DOCUMENT_UPLOAD',
  resource: 'documents',
  resourceId: document.id,
  ip: req.ip,
  userAgent: req.get('User-Agent'),
  timestamp: new Date(),
  details: {
    documentType: document.type,
    fileSize: file.size,
    success: true
  }
};
```

## 📞 Support

### Emergency Contacts

- **System Administrator**: admin@caesar-token.io
- **Security Team**: security@caesar-token.io
- **Compliance Officer**: compliance@caesar-token.io

### Escalation Procedures

1. **Level 1**: Development team response (15 minutes)
2. **Level 2**: Senior engineer response (30 minutes)  
3. **Level 3**: CTO response (1 hour)
4. **Level 4**: CEO response (2 hours)

### Maintenance Windows

- **Regular Maintenance**: Sundays 2:00-4:00 AM UTC
- **Emergency Maintenance**: As needed with 1-hour notice
- **Security Updates**: Immediate deployment

---

## 🎯 Performance Targets

- **API Response Time**: <200ms (95th percentile)
- **Fiat-to-GATE Conversion**: <30 seconds
- **System Uptime**: >99.95%
- **Transaction Success Rate**: >99.9%
- **Database Query Time**: <50ms average

Deployment completed successfully! 🚀