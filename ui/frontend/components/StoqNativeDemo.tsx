// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * STOQ Native Protocol Demo Component
 * 
 * Demonstrates the STOQ native WebAssembly client integration
 * with TrustChain certificate authentication and real-time
 * dashboard updates via pure QUIC protocol.
 */

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { AlertCircle, CheckCircle, Globe, Zap, Shield, Activity } from 'lucide-react';
import {
  useStoqNative,
  useStoqSystemStatus,
  useStoqPerformanceMetrics,
  useStoqDashboardData,
  useStoqNativePreference,
} from '@/lib/api/hooks/useStoqNative';

interface StoqNativeDemoProps {
  certificatePem?: string;
}

export function StoqNativeDemo({ certificatePem }: StoqNativeDemoProps) {
  const [selectedDashboard, setSelectedDashboard] = useState('overview');
  
  // STOQ native connection management
  const {
    connectionState,
    initialize,
    disconnect,
    isInitializing,
    isDisconnecting,
  } = useStoqNative(certificatePem);

  // STOQ native preference check
  const nativePreference = useStoqNativePreference();

  // Data hooks (only enabled when connected)
  const systemStatus = useStoqSystemStatus(connectionState.isAuthenticated);
  const performanceMetrics = useStoqPerformanceMetrics('1h', connectionState.isAuthenticated);
  const dashboardData = useStoqDashboardData(selectedDashboard, connectionState.isAuthenticated);

  // Mock certificate for demo
  const mockCertificate = `-----BEGIN CERTIFICATE-----
MIIDQTCCAimgAwIBAgITBmyfz5m/jAo54vB4ikPmljZbyjANBgkqhkiG9w0BAQsF
ADA5MQswCQYDVQQGEwJVUzEPMA0GA1UEChMGQW1hem9uMRkwFwYDVQQDExBBbWF6
b24gUm9vdCBDQSAxMB4XDTE1MDUyNjAwMDAwMFoXDTM4MDExNzAwMDAwMFowOTEL
MAkGA1UEBhMCVVMxDzANBgNVBAoTBkFtYXpvbjEZMBcGA1UEAxMQQW1hem9uIFJv
b3QgQ0EgMTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALJ4gHHKeNXj
ca9HgFB0fW7Y14h29Jlo91ghYPl0hAEvrAIthtOgQ3pOsqTQNroBvo3bSMgHFzZM
9O6II8c+6zf1tRn4SWiw3te5djgdYZ6k/oI2peVKVuRF4fn9tBb6dNqcmzU5L/qw
IFAGbHrQgLKm+a/sRxmPUDgH3KKHOVj4utWp+UhnMJbulHheb4mjUcAwhmahRWa6
VOujw5H5SNz/0egwLX0tdHA114gk957EWW67c4cX8jJGKLhD+rcdqsq08p8kDi1L
93FcXmn/6pUCyziKrlA4b9v7LWIbxcceVOF34GfID5yHI9Y/QCB/IIDEgEw+OyQm
jgSubJrIqg0CAwEAAaNCMEAwDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMC
AYYwHQYDVR0OBBYEFIQYzIU07LwMlJQuCFmcx7IQTgoIMA0GCSqGSIb3DQEBCwUA
A4IBAQCY8jdaQZChGsV2USggNiMOruYou6r4lK5IpDB/G/wkjUu0yKGX9rbxenDI
U5PMCCjjmCXPI6T53iHTfIuJruydjsw2hUwsOjO4kkv5XqIEkPmZUjlJOI9KNKar
vLLxJKtdQ+N9F4J8eHrh+b5iGcdJZPZA1AKF5YglO1h5pIKO+DbZHvqtV9WCc7g0
Ap3w0vQ4kLuB2LrEiJCXSJhYMIKmYOAe5LjpvyD8Zy7IYqP3Jdy+4HFa+eCrCtJP
CqKEBP8VLHqU2YmfvV9OaqNlJW3YBG6XO5Qz5SKL8+K6aNY4k8+mBqJ4rOdGj5n1
qxcONaAKfqjgH+3c6VAhZFZQ3kNY
-----END CERTIFICATE-----`;

  const handleConnect = () => {
    const cert = certificatePem || mockCertificate;
    initialize(cert);
  };

  const handleDisconnect = () => {
    disconnect();
  };

  const getStatusIcon = () => {
    if (!connectionState.isAvailable) {
      return <AlertCircle className="h-5 w-5 text-red-500" />;
    }
    if (connectionState.isAuthenticated) {
      return <CheckCircle className="h-5 w-5 text-green-500" />;
    }
    if (isInitializing) {
      return <Activity className="h-5 w-5 text-blue-500 animate-spin" />;
    }
    return <Globe className="h-5 w-5 text-gray-400" />;
  };

  const getStatusText = () => {
    if (!connectionState.isAvailable) {
      return 'WebAssembly not available';
    }
    if (connectionState.isAuthenticated) {
      return `Connected via ${connectionState.protocol}`;
    }
    if (isInitializing) {
      return 'Connecting to STOQ server...';
    }
    return 'Ready to connect';
  };

  const getStatusBadge = () => {
    if (!connectionState.isAvailable) {
      return <Badge variant="destructive">Unavailable</Badge>;
    }
    if (connectionState.isAuthenticated) {
      return <Badge variant="default" className="bg-green-600">Connected</Badge>;
    }
    if (isInitializing) {
      return <Badge variant="secondary">Connecting</Badge>;
    }
    return <Badge variant="outline">Disconnected</Badge>;
  };

  return (
    <div className="space-y-6 p-6">
      {/* Header */}
      <div className="text-center space-y-2">
        <h1 className="text-3xl font-bold flex items-center justify-center gap-2">
          <Shield className="h-8 w-8 text-blue-500" />
          STOQ Native Protocol Demo
        </h1>
        <p className="text-gray-600 max-w-2xl mx-auto">
          Experience Internet 2.0 with pure QUIC protocol communication, TrustChain certificate authentication,
          and WebAssembly-powered browser integration. No HTTP, no REST APIs - just pure Internet 2.0.
        </p>
      </div>

      {/* Connection Status */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span className="flex items-center gap-2">
              {getStatusIcon()}
              Connection Status
            </span>
            {getStatusBadge()}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <span>{getStatusText()}</span>
            {connectionState.connectionId && (
              <span className="text-sm text-gray-500 font-mono">
                ID: {connectionState.connectionId.slice(0, 8)}...
              </span>
            )}
          </div>

          {connectionState.error && (
            <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
              {connectionState.error}
            </div>
          )}

          <div className="flex gap-2">
            {!connectionState.isAuthenticated ? (
              <Button 
                onClick={handleConnect} 
                disabled={isInitializing || !connectionState.isAvailable}
                className="flex items-center gap-2"
              >
                <Zap className="h-4 w-4" />
                {isInitializing ? 'Connecting...' : 'Connect via STOQ'}
              </Button>
            ) : (
              <Button 
                onClick={handleDisconnect} 
                disabled={isDisconnecting}
                variant="outline"
              >
                {isDisconnecting ? 'Disconnecting...' : 'Disconnect'}
              </Button>
            )}
          </div>

          {/* Protocol Preference Info */}
          {nativePreference.canUpgrade && (
            <div className="p-3 bg-blue-50 border border-blue-200 rounded-lg text-blue-700 text-sm">
              <div className="flex items-center gap-2">
                <Zap className="h-4 w-4" />
                <strong>Upgrade Available:</strong> {nativePreference.reason}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* System Status */}
      {connectionState.isAuthenticated && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* System Health */}
          <Card>
            <CardHeader>
              <CardTitle>System Health</CardTitle>
            </CardHeader>
            <CardContent>
              {systemStatus.isLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Activity className="h-6 w-6 animate-spin" />
                </div>
              ) : systemStatus.error ? (
                <div className="text-red-500 text-sm">{systemStatus.error.message}</div>
              ) : systemStatus.data ? (
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="font-medium">Overall Health</span>
                    <Badge variant={systemStatus.data.overall_health === 'good' ? 'default' : 'destructive'}>
                      {systemStatus.data.overall_health}
                    </Badge>
                  </div>
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span>Health Score</span>
                      <span>{systemStatus.data.score}%</span>
                    </div>
                    <Progress value={systemStatus.data.score} className="h-2" />
                  </div>
                  <div className="space-y-2">
                    {Object.entries(systemStatus.data.services).map(([service, status]) => (
                      <div key={service} className="flex items-center justify-between text-sm">
                        <span className="capitalize">{service}</span>
                        <Badge variant={status.status === 'healthy' ? 'default' : 'destructive'}>
                          {status.status}
                        </Badge>
                      </div>
                    ))}
                  </div>
                </div>
              ) : (
                <div className="text-gray-500 text-sm">No data available</div>
              )}
            </CardContent>
          </Card>

          {/* Performance Metrics */}
          <Card>
            <CardHeader>
              <CardTitle>Performance Metrics</CardTitle>
            </CardHeader>
            <CardContent>
              {performanceMetrics.isLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Activity className="h-6 w-6 animate-spin" />
                </div>
              ) : performanceMetrics.error ? (
                <div className="text-red-500 text-sm">{performanceMetrics.error.message}</div>
              ) : performanceMetrics.data ? (
                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span>Throughput</span>
                      <span>{performanceMetrics.data.throughput.current}/{performanceMetrics.data.throughput.target} Mbps</span>
                    </div>
                    <Progress value={performanceMetrics.data.throughput.efficiency} className="h-2" />
                    <div className="text-xs text-gray-500 mt-1">
                      {performanceMetrics.data.throughput.efficiency}% efficiency
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <div className="font-medium">Latency</div>
                      <div className="text-gray-600">{performanceMetrics.data.latency.average} ms avg</div>
                    </div>
                    <div>
                      <div className="font-medium">Connections</div>
                      <div className="text-gray-600">{performanceMetrics.data.connections.active} active</div>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-gray-500 text-sm">No data available</div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Dashboard Data */}
      {connectionState.isAuthenticated && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span>Dashboard Data</span>
              <select
                value={selectedDashboard}
                onChange={(e) => setSelectedDashboard(e.target.value)}
                className="px-3 py-1 border border-gray-300 rounded-md text-sm"
              >
                <option value="overview">Overview</option>
                <option value="hypermesh">HyperMesh</option>
                <option value="trustchain">TrustChain</option>
                <option value="stoq">STOQ</option>
              </select>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {dashboardData.isLoading ? (
              <div className="flex items-center justify-center py-8">
                <Activity className="h-6 w-6 animate-spin" />
              </div>
            ) : dashboardData.error ? (
              <div className="text-red-500 text-sm">{dashboardData.error.message}</div>
            ) : dashboardData.data ? (
              <pre className="bg-gray-50 p-4 rounded-lg text-xs overflow-auto">
                {JSON.stringify(dashboardData.data, null, 2)}
              </pre>
            ) : (
              <div className="text-gray-500 text-sm">No data available</div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Protocol Information */}
      <Card>
        <CardHeader>
          <CardTitle>Internet 2.0 Protocol Information</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div className="p-3 bg-blue-50 rounded-lg">
              <div className="font-medium text-blue-700">Transport</div>
              <div className="text-blue-600">Pure QUIC over IPv6</div>
            </div>
            <div className="p-3 bg-green-50 rounded-lg">
              <div className="font-medium text-green-700">Authentication</div>
              <div className="text-green-600">TrustChain Certificates</div>
            </div>
            <div className="p-3 bg-purple-50 rounded-lg">
              <div className="font-medium text-purple-700">Runtime</div>
              <div className="text-purple-600">WebAssembly in Browser</div>
            </div>
          </div>
          
          <div className="text-sm text-gray-600">
            <p>
              This demo showcases a complete Internet 2.0 implementation where the browser connects directly
              to the STOQ server using pure QUIC protocol with TrustChain certificate authentication.
              No HTTP, no REST APIs - just pure Internet 2.0 architecture compiled to WebAssembly.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}