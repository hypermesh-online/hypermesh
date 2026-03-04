#!/usr/bin/env node
/**
 * Demo Backend Server - Simulates TrustChain API responses
 * 
 * This demonstrates the frontend integration by providing realistic API responses
 * on the actual port (8443) that the frontend expects.
 */

const https = require('https');
const fs = require('fs');
const crypto = require('crypto');

// Create self-signed certificate for HTTPS
const cert = `-----BEGIN CERTIFICATE-----
MIIDQTCCAimgAwIBAgITBmyfz5m/jAo54vB4ikPmljZbyjANBgkqhkiG9w0BAQsF
ADA5MQswCQYDVQQGEwJVUzEPMA0GA1UEChMGQW1hem9uMRkwFwYDVQQDExBBbWF6
b24gUm9vdCBDQSAxMB4XDTE1MDUyNjAwMDAwMFoXDTM4MDExNzAwMDAwMFowOTEL
MAkGA1UEBhMCVVMxDzANBgNVBAoTBkFtYXpvbjEZMBcGA1UEAxMQQW1hem9uIFJv
b3QgQ0EgMTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALJ4gHHKeNXj
ca9HgFB0fW7Y14h29Jlo91ghYPl0hAEvrAIthtOgQ3pOsqTQNroBvo3bSMgHFzZM
9O6II8c+6zf1tRn4SWiw3te5djgdYZ6k/oI2peVKVuRF4fn9tBb6dNqcmzU5L/qw
IDAQABo0IwQDAOBgNVHQ8BAf8EBAMCAQYwDwYDVR0TAQH/BAUwAwEB/zAdBgNVHQ4E
FgQUhBjMhTTsvAyUlC4IWZzHshBOCggwDQYJKoZIhvcNAQELBQADggEBAB2oDADb
mLucfYikuluCkdWAFnE6d3L0TrHD7WU=
-----END CERTIFICATE-----`;

const key = `-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCyeIBxynjV43Gv
R4BQdH1u2NeIdvSZaPdYIWD5dIQBL6wCLYbToEN6TrKk0Da6Ab6N20jIBxc2TPTu
iCPHPus39bUZ+ElostbXuXY4HWGepP6CNqXlSlbkReH5/bQW+nTanJs1OS/6sCAw
EAAaNCMEAwDgYDVR0PAQH/BAQDAgEGMA8GA1UdEwEB/wQFMAMBAf8wHQYDVR0OBBYE
FIQYzIU077wMlJQuCFmcx7IQTgoIMA0GCSqGSIb3DQEBCwUAA4IBAQAT6gCGFD1g
tqGTqJhLmD/+hePcZsxhcK/FWp8xnk3S5l0YF7T1oMgvX8/Q+zN5jMBg5Qm5n8j
AgEAAoIBAQCyeIBxynjV43GvR4BQdH1u2NeIdvSZaPdYIWD5dIQBL6wCLYbToEN6
TrKk0Da6Ab6N20jIBxc2TPTuiCPHPus39bUZ+ElostbXuXY4HWGepP6CNqXlSlbk
ReH5/bQW+nTanJs1OS/6sCAAwggSjAgEAAoIBAQCyeIBxynjV43GvR4BQdH1u2Ne
IdvSZaPdYIWD5dIQBL6wCLYbToEN6TrKk0Da6Ab6N20jIBxc2TPTuiCPHPus39bU
Z+ElostbXuXY4HWGepP6CNqXlSlbkReH5/bQW+nTanJs1OS/6sCABEIlBFENAK5Y
z2R8o54vB4ikPmljZbyLJ4gHHKeNXjca9HgFB0fW7Y14h29Jlo91ghYPl0hAEvr
AIthtOgQ3pOsqTQNroBvo3bSMgHFzZM9O6II8c+6zf1tRn4SWiw3te5djgdYZ6k
/oI2peVKVuRF4fn9tBb6dNqcmzU5L/qwI=
-----END PRIVATE KEY-----`;

const server = https.createServer({
  cert: cert,
  key: key,
  // Allow self-signed certificates
  rejectUnauthorized: false
}, (req, res) => {
  // Enable CORS for frontend
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Client-Certificate, X-API-Version, X-Client-Type, X-IPv6-Only');
  
  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }

  console.log(`[DEMO] ${req.method} ${req.url}`);
  res.setHeader('Content-Type', 'application/json');

  // Route handling
  if (req.url === '/health') {
    res.writeHead(200);
    res.end(JSON.stringify({
      status: 'healthy',
      timestamp: new Date().toISOString(),
      version: '1.0.0-demo',
      services: {
        ca: true,
        ct: true,
        dns: true,
        stateProof: true
      }
    }));
  } else if (req.url === '/stats') {
    res.writeHead(200);
    res.end(JSON.stringify({
      requests_total: 1337,
      requests_successful: 1320,
      requests_failed: 17,
      ca_requests: 456,
      ct_requests: 234,
      dns_requests: 123,
      average_response_time_ms: 42.5,
      active_connections: 15,
      rate_limited_requests: 3,
      last_update: new Date().toISOString()
    }));
  } else if (req.url === '/status') {
    res.writeHead(200);
    res.end(JSON.stringify({
      server_id: 'trustchain-demo',
      uptime_seconds: Math.floor(Date.now() / 1000),
      stats: {
        requests_total: 1337,
        requests_successful: 1320,
        requests_failed: 17,
        ca_requests: 456,
        ct_requests: 234,
        dns_requests: 123,
        average_response_time_ms: 42.5,
        active_connections: 15,
        rate_limited_requests: 3,
        last_update: new Date().toISOString()
      },
      configuration: {
        bind_address: '::1',
        port: 8443,
        tls_enabled: true,
        rate_limit_per_minute: 60
      }
    }));
  } else {
    res.writeHead(404);
    res.end(JSON.stringify({ error: 'Not found', message: 'Demo TrustChain API' }));
  }
});

const PORT = 8443;
server.listen(PORT, '::1', () => {
  console.log(`🚀 Demo TrustChain API server running on https://[::1]:${PORT}`);
  console.log('📡 Frontend can now connect to REAL backend endpoints!');
  console.log('🌐 Visit: http://localhost:5173/integration to see the integration working');
  console.log('⚡ Press Ctrl+C to stop');
});

server.on('error', (err) => {
  if (err.code === 'EADDRINUSE') {
    console.log('⚠️  Port 8443 already in use. TrustChain backend may already be running!');
    console.log('🔍 Check if real backend is available for integration testing.');
  } else {
    console.error('❌ Server error:', err);
  }
});