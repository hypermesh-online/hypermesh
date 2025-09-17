#!/bin/bash
# TrustChain Instance Initialization Script
# Environment: ${environment}

set -e

# Update system
apt-get update
apt-get upgrade -y

# Install required packages
apt-get install -y \
    curl \
    wget \
    unzip \
    jq \
    htop \
    iotop \
    netstat-nat \
    tcpdump \
    fail2ban \
    ufw \
    chrony \
    rsyslog \
    logrotate

# Configure hostname
hostnamectl set-hostname "${hostname_prefix}-${environment}-$(curl -s http://169.254.169.254/latest/meta-data/instance-id)"

# Configure IPv6-only networking
echo "net.ipv6.conf.all.disable_ipv6 = 0" >> /etc/sysctl.conf
echo "net.ipv6.conf.default.disable_ipv6 = 0" >> /etc/sysctl.conf
echo "net.ipv6.conf.lo.disable_ipv6 = 0" >> /etc/sysctl.conf
sysctl -p

# Configure UFW for IPv6-only
ufw --force enable
ufw default deny incoming
ufw default allow outgoing

# Allow TrustChain services (IPv6 only)
ufw allow from ::/0 to any port 8443 proto tcp comment 'TrustChain CA API'
ufw allow from ::/0 to any port 6962 proto tcp comment 'Certificate Transparency'
ufw allow from ::/0 to any port 8853 proto udp comment 'DNS-over-QUIC'
ufw allow from ::/0 to any port 8444 proto udp comment 'STOQ Protocol'
ufw allow from ::/0 to any port 8445 proto tcp comment 'HyperMesh Integration'
ufw allow from ::/0 to any port 8446 proto tcp comment 'Integration API'
ufw allow from ::/0 to any port 8080 proto tcp comment 'Health Check'

# SSH access (restricted to management networks)
ufw allow from 2001:db8::/32 to any port 22 proto tcp comment 'SSH Management'

# Configure fail2ban
systemctl enable fail2ban
systemctl start fail2ban

# Install CloudWatch Agent
wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/arm64/latest/amazon-cloudwatch-agent.deb
dpkg -i amazon-cloudwatch-agent.deb

# CloudWatch Agent Configuration
cat > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json << 'EOF'
{
    "agent": {
        "metrics_collection_interval": 60,
        "run_as_user": "cwagent"
    },
    "metrics": {
        "namespace": "TrustChain/${environment}",
        "metrics_collected": {
            "cpu": {
                "measurement": [
                    "cpu_usage_idle",
                    "cpu_usage_iowait",
                    "cpu_usage_user",
                    "cpu_usage_system"
                ],
                "metrics_collection_interval": 60,
                "totalcpu": true
            },
            "disk": {
                "measurement": [
                    "used_percent"
                ],
                "metrics_collection_interval": 60,
                "resources": [
                    "*"
                ]
            },
            "diskio": {
                "measurement": [
                    "io_time",
                    "read_bytes",
                    "write_bytes",
                    "reads",
                    "writes"
                ],
                "metrics_collection_interval": 60,
                "resources": [
                    "*"
                ]
            },
            "mem": {
                "measurement": [
                    "mem_used_percent"
                ],
                "metrics_collection_interval": 60
            },
            "netstat": {
                "measurement": [
                    "tcp_established",
                    "tcp_time_wait"
                ],
                "metrics_collection_interval": 60
            },
            "swap": {
                "measurement": [
                    "swap_used_percent"
                ],
                "metrics_collection_interval": 60
            }
        }
    },
    "logs": {
        "logs_collected": {
            "files": {
                "collect_list": [
                    {
                        "file_path": "/var/log/syslog",
                        "log_group_name": "/aws/ec2/trustchain/${environment}/syslog",
                        "log_stream_name": "{instance_id}"
                    },
                    {
                        "file_path": "/var/log/auth.log",
                        "log_group_name": "/aws/ec2/trustchain/${environment}/auth",
                        "log_stream_name": "{instance_id}"
                    },
                    {
                        "file_path": "/var/log/trustchain/*.log",
                        "log_group_name": "/aws/ec2/trustchain/${environment}/application",
                        "log_stream_name": "{instance_id}"
                    }
                ]
            }
        }
    }
}
EOF

# Start CloudWatch Agent
/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl \
    -a fetch-config \
    -m ec2 \
    -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json \
    -s

# Install AWS CLI v2
curl "https://awscli.amazonaws.com/awscli-exe-linux-aarch64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
./aws/install

# Create TrustChain application directory
mkdir -p /opt/trustchain/{bin,config,data,logs}
chown -R ubuntu:ubuntu /opt/trustchain

# Create systemd service directory
mkdir -p /etc/systemd/system

# Set up log rotation for TrustChain
cat > /etc/logrotate.d/trustchain << 'EOF'
/opt/trustchain/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    copytruncate
}
EOF

# Performance tuning for high-throughput networking
echo 'net.core.rmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_rmem = 4096 87380 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_wmem = 4096 65536 134217728' >> /etc/sysctl.conf
echo 'net.core.netdev_max_backlog = 5000' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_window_scaling = 1' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_timestamps = 1' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_sack = 1' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_no_metrics_save = 1' >> /etc/sysctl.conf
sysctl -p

# Install Docker for containerized TrustChain services
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh
usermod -aG docker ubuntu

# Install Docker Compose
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Create health check script
cat > /opt/trustchain/bin/health-check.sh << 'EOF'
#!/bin/bash
# TrustChain Health Check Script

HEALTH_ENDPOINT="http://[::1]:8080/health"
MAX_RETRIES=5
RETRY_DELAY=5

for i in $(seq 1 $MAX_RETRIES); do
    if curl -s -f "$HEALTH_ENDPOINT" > /dev/null; then
        echo "Health check passed"
        exit 0
    fi
    echo "Health check failed (attempt $i/$MAX_RETRIES)"
    sleep $RETRY_DELAY
done

echo "Health check failed after $MAX_RETRIES attempts"
exit 1
EOF

chmod +x /opt/trustchain/bin/health-check.sh

# Configure chrony for accurate time synchronization (critical for certificates)
cat > /etc/chrony/chrony.conf << 'EOF'
# Amazon Time Sync Service
server 169.254.169.123 prefer iburst minpoll 4 maxpoll 4

# Additional NTP servers for redundancy
pool 2.amazon.pool.ntp.org iburst maxsources 3
pool 1.ubuntu.pool.ntp.org iburst maxsources 1
pool 0.ubuntu.pool.ntp.org iburst maxsources 1

driftfile /var/lib/chrony/chrony.drift
makestep 1.0 3
rtcsync
logdir /var/log/chrony
log measurements statistics tracking
EOF

systemctl restart chrony
systemctl enable chrony

# Set up initial directory structure for TrustChain data
mkdir -p /opt/trustchain/data/{certificates,ct-logs,dns,keys}
mkdir -p /opt/trustchain/config/{ca,ct,dns,stoq}

# Create mount point for additional EBS volume
mkdir -p /mnt/trustchain-data
echo '/dev/nvme1n1 /mnt/trustchain-data ext4 defaults,nofail 0 2' >> /etc/fstab

# Format and mount additional EBS volume if not already formatted
if ! blkid /dev/nvme1n1; then
    mkfs -t ext4 /dev/nvme1n1
fi
mount -a

# Create symlinks to additional storage
ln -sf /mnt/trustchain-data /opt/trustchain/data/persistent

# Set ownership
chown -R ubuntu:ubuntu /opt/trustchain
chown -R ubuntu:ubuntu /mnt/trustchain-data

# Create startup script that will be called by systemd
cat > /opt/trustchain/bin/startup.sh << 'EOF'
#!/bin/bash
# TrustChain Startup Script

set -e

# Get instance metadata
INSTANCE_ID=$(curl -s http://169.254.169.254/latest/meta-data/instance-id)
IPV6_ADDRESS=$(curl -s http://169.254.169.254/latest/meta-data/network/interfaces/macs/$(curl -s http://169.254.169.254/latest/meta-data/mac)/ipv6s)

# Wait for AWS services to be available
aws sts get-caller-identity

# Retrieve configuration from Systems Manager
aws ssm get-parameters-by-path \
    --path "/trustchain/${environment}/" \
    --recursive \
    --decrypt \
    --query 'Parameters[*].[Name,Value]' \
    --output text > /opt/trustchain/config/aws-parameters.txt

# Log startup
echo "$(date): TrustChain instance $INSTANCE_ID starting with IPv6 $IPV6_ADDRESS" >> /opt/trustchain/logs/startup.log

# Additional initialization will be added when TrustChain binaries are deployed
EOF

chmod +x /opt/trustchain/bin/startup.sh

# Signal that user data execution is complete
/opt/aws/bin/cfn-signal -e $? --stack ${environment} --resource AutoScalingGroup --region $(curl -s http://169.254.169.254/latest/meta-data/placement/region) || true

echo "TrustChain instance initialization completed at $(date)" >> /opt/trustchain/logs/startup.log