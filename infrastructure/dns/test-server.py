#!/usr/bin/env python3
"""
Simple HTTPS test server for validating local DNS setup
This server demonstrates domain-based routing for the Web3 ecosystem
"""

import ssl
import json
import os
from http.server import HTTPServer, BaseHTTPRequestHandler
import threading
import time
from datetime import datetime

class Web3TestHandler(BaseHTTPRequestHandler):
    """HTTP handler that provides different responses based on the domain"""
    
    def do_GET(self):
        """Handle GET requests with domain-specific responses"""
        host = self.headers.get('Host', 'unknown')
        domain = host.split(':')[0]  # Remove port if present
        
        # Domain-specific responses
        service_info = {
            'hypermesh.online': {
                'service': 'HyperMesh Main Dashboard',
                'description': 'Universal asset management and orchestration',
                'features': ['Asset System', 'VM Execution', 'Resource Allocation'],
                'status': 'operational'
            },
            'trust.hypermesh.online': {
                'service': 'TrustChain Authority',
                'description': 'Certificate authority and DNS resolution',
                'features': ['Certificate Issuance', 'DNS Resolution', 'Falcon-1024 PQC'],
                'status': 'operational'
            },
            'caesar.hypermesh.online': {
                'service': 'Caesar Economics',
                'description': 'Economic incentive and reward system',
                'features': ['Asset Rewards', 'Economic Incentives', 'Token Management'],
                'status': 'operational'
            },
            'catalog.hypermesh.online': {
                'service': 'Catalog VM System',
                'description': 'Virtual machine execution and management',
                'features': ['Julia VM', 'Remote Execution', 'Asset Integration'],
                'status': 'operational'
            },
            'stoq.hypermesh.online': {
                'service': 'STOQ Transport',
                'description': 'High-performance QUIC transport protocol',
                'features': ['40 Gbps Target', 'IPv6 Only', 'Zero-Copy Operations'],
                'status': 'performance-limited',
                'note': 'Currently 2.95 Gbps (7.4% of target)'
            },
            'ngauge.hypermesh.online': {
                'service': 'NGauge Platform',
                'description': 'User engagement and interaction platform',
                'features': ['User Analytics', 'Engagement Metrics', 'Platform Integration'],
                'status': 'development'
            }
        }
        
        if self.path == '/health':
            self.send_health_response()
        elif self.path == '/api/status':
            self.send_api_response(domain, service_info.get(domain, self.get_unknown_service(domain)))
        elif self.path == '/':
            self.send_dashboard_response(domain, service_info.get(domain, self.get_unknown_service(domain)))
        else:
            self.send_404_response()
    
    def get_unknown_service(self, domain):
        """Return a default response for unknown domains"""
        return {
            'service': f'Unknown Service ({domain})',
            'description': 'Domain configured but service not recognized',
            'features': ['DNS Resolution Working'],
            'status': 'unknown'
        }
    
    def send_health_response(self):
        """Send a simple health check response"""
        self.send_response(200)
        self.send_header('Content-type', 'text/plain')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(b'healthy\n')
    
    def send_api_response(self, domain, service_info):
        """Send JSON API response"""
        response_data = {
            'timestamp': datetime.now().isoformat(),
            'domain': domain,
            'dns_status': 'resolved',
            'https_status': 'working',
            'service': service_info,
            'infrastructure': {
                'protocol': 'HTTPS (test)',
                'port': 8443,
                'certificates': 'self-signed',
                'ipv6_support': True,
                'internet2_stack': False  # This is the test server
            }
        }
        
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(json.dumps(response_data, indent=2).encode())
    
    def send_dashboard_response(self, domain, service_info):
        """Send HTML dashboard response"""
        html = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{service_info['service']} - Web3 Ecosystem</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            min-height: 100vh;
        }}
        .container {{
            max-width: 800px;
            margin: 0 auto;
            background: rgba(255, 255, 255, 0.1);
            padding: 30px;
            border-radius: 15px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .domain {{
            font-size: 1.2em;
            opacity: 0.8;
            margin-bottom: 10px;
        }}
        .service-name {{
            font-size: 2.5em;
            font-weight: bold;
            margin-bottom: 10px;
        }}
        .description {{
            font-size: 1.1em;
            opacity: 0.9;
        }}
        .features {{
            margin: 30px 0;
        }}
        .features h3 {{
            margin-bottom: 15px;
        }}
        .feature-list {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 10px;
        }}
        .feature {{
            background: rgba(255, 255, 255, 0.1);
            padding: 10px 15px;
            border-radius: 8px;
            border-left: 4px solid #00ff88;
        }}
        .status {{
            display: inline-block;
            padding: 5px 15px;
            border-radius: 20px;
            font-weight: bold;
            text-transform: uppercase;
            font-size: 0.9em;
        }}
        .status.operational {{ background: #00ff88; color: #000; }}
        .status.development {{ background: #ffaa00; color: #000; }}
        .status.performance-limited {{ background: #ff6b6b; color: #fff; }}
        .status.unknown {{ background: #888; color: #fff; }}
        .infrastructure {{
            margin-top: 30px;
            background: rgba(0, 0, 0, 0.2);
            padding: 20px;
            border-radius: 10px;
        }}
        .infrastructure h3 {{
            margin-top: 0;
        }}
        .infrastructure-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 15px;
            margin-top: 15px;
        }}
        .infrastructure-item {{
            text-align: center;
        }}
        .infrastructure-label {{
            font-size: 0.9em;
            opacity: 0.7;
        }}
        .infrastructure-value {{
            font-weight: bold;
            font-size: 1.1em;
        }}
        .note {{
            background: rgba(255, 255, 255, 0.1);
            padding: 15px;
            border-radius: 8px;
            margin-top: 20px;
            border-left: 4px solid #ffaa00;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="domain">🌐 {domain}</div>
            <div class="service-name">{service_info['service']}</div>
            <div class="description">{service_info['description']}</div>
            <div style="margin-top: 15px;">
                <span class="status {service_info['status']}">{service_info['status']}</span>
            </div>
        </div>
        
        <div class="features">
            <h3>Features</h3>
            <div class="feature-list">
                {"".join(f'<div class="feature">{feature}</div>' for feature in service_info['features'])}
            </div>
        </div>
        
        <div class="infrastructure">
            <h3>🏗️ Infrastructure Status</h3>
            <div class="infrastructure-grid">
                <div class="infrastructure-item">
                    <div class="infrastructure-label">DNS Resolution</div>
                    <div class="infrastructure-value">✅ Working</div>
                </div>
                <div class="infrastructure-item">
                    <div class="infrastructure-label">HTTPS</div>
                    <div class="infrastructure-value">✅ Working</div>
                </div>
                <div class="infrastructure-item">
                    <div class="infrastructure-label">Certificate</div>
                    <div class="infrastructure-value">🔐 Self-Signed</div>
                </div>
                <div class="infrastructure-item">
                    <div class="infrastructure-label">Port</div>
                    <div class="infrastructure-value">8443</div>
                </div>
                <div class="infrastructure-item">
                    <div class="infrastructure-label">IPv6</div>
                    <div class="infrastructure-value">✅ Supported</div>
                </div>
                <div class="infrastructure-item">
                    <div class="infrastructure-label">Internet 2.0</div>
                    <div class="infrastructure-value">🚧 Test Mode</div>
                </div>
            </div>
        </div>
        
        {'<div class="note"><strong>Note:</strong> ' + service_info.get('note', '') + '</div>' if service_info.get('note') else ''}
        
        <div style="margin-top: 30px; text-align: center; opacity: 0.7;">
            <p>🧪 <strong>Test Server</strong> - Validating DNS routing for Web3 ecosystem</p>
            <p>📡 API Endpoint: <a href="/api/status" style="color: #00ff88;">{domain}/api/status</a></p>
            <p>❤️ Health Check: <a href="/health" style="color: #00ff88;">{domain}/health</a></p>
        </div>
    </div>
</body>
</html>
        """
        
        self.send_response(200)
        self.send_header('Content-type', 'text/html; charset=utf-8')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(html.encode())
    
    def send_404_response(self):
        """Send 404 response"""
        self.send_response(404)
        self.send_header('Content-type', 'text/plain')
        self.end_headers()
        self.wfile.write(b'404 - Not Found\n')
    
    def log_message(self, format, *args):
        """Custom log format"""
        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        host = self.headers.get('Host', 'unknown')
        print(f"[{timestamp}] {host} - {format % args}")

def start_test_server():
    """Start the HTTPS test server"""
    server_address = ('', 8443)
    httpd = HTTPServer(server_address, Web3TestHandler)
    
    # Configure SSL
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain('certificates/hypermesh-server.crt', 'certificates/hypermesh-server.key')
    httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
    
    print("🌐 Web3 Ecosystem DNS Test Server")
    print("=" * 50)
    print(f"🚀 Server starting on https://0.0.0.0:8443")
    print(f"📡 Protocol: HTTPS with self-signed certificates")
    print(f"🔐 Certificate: certificates/hypermesh-server.crt")
    print(f"📝 Logs: Real-time request logging enabled")
    print()
    print("🌍 Available domains:")
    domains = [
        'hypermesh.online',
        'trust.hypermesh.online',
        'caesar.hypermesh.online',
        'catalog.hypermesh.online',
        'stoq.hypermesh.online',
        'ngauge.hypermesh.online'
    ]
    for domain in domains:
        print(f"  • https://{domain}:8443")
    print()
    print("🔗 API endpoints:")
    print("  • /api/status - JSON service information")
    print("  • /health - Health check")
    print("  • / - Service dashboard")
    print()
    print("⚠️  Use --insecure flag with curl or import CA certificate for browsers")
    print("🛑 Press Ctrl+C to stop the server")
    print()
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n🛑 Server shutting down...")
        httpd.shutdown()

if __name__ == '__main__':
    # Check if certificates exist
    if not os.path.exists('certificates/hypermesh-server.crt') or not os.path.exists('certificates/hypermesh-server.key'):
        print("❌ SSL certificates not found!")
        print("🔧 Run: ./infrastructure/dns/local-dns-setup.sh setup")
        exit(1)
    
    start_test_server()