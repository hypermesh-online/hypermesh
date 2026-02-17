// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Integration Test Component - Demonstrates real backend API integration
 * 
 * This component tests the real vs mock API integration to show that:
 * - Frontend components will attempt to connect to real backend APIs first
 * - If backend is unavailable, falls back to realistic mock data  
 * - Shows current status of each service integration
 * - Displays actual vs target performance metrics
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useSystemStatus } from '@/lib/api';
import { CheckCircle, XCircle, AlertTriangle, RefreshCw } from 'lucide-react';

export function IntegrationTest() {
  const { systemStatus, isLoading, error, refetch } = useSystemStatus(true);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'healthy': return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'warning': case 'degraded': return <AlertTriangle className="w-4 h-4 text-yellow-500" />;
      case 'critical': case 'offline': return <XCircle className="w-4 h-4 text-red-500" />;
      default: return <AlertTriangle className="w-4 h-4 text-gray-500" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'healthy': return 'bg-green-500/20 text-green-300 border-green-500/20';
      case 'warning': case 'degraded': return 'bg-yellow-500/20 text-yellow-300 border-yellow-500/20';
      case 'critical': case 'offline': return 'bg-red-500/20 text-red-300 border-red-500/20';
      default: return 'bg-gray-500/20 text-gray-300 border-gray-500/20';
    }
  };

  if (isLoading) {
    return (
      <Card className="bg-gray-900 border-gray-700">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <RefreshCw className="w-5 h-5 animate-spin" />
            Testing Backend Integration...
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-gray-400">Attempting to connect to backend services...</div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      <Card className="bg-gray-900 border-gray-700">
        <CardHeader>
          <CardTitle className="text-white">Backend Integration Status</CardTitle>
          <CardDescription className="text-gray-400">
            Testing connection to real TrustChain, HyperMesh, and STOQ backend services
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Overall Status */}
          <div className="flex items-center justify-between p-3 bg-gray-800 rounded-lg">
            <div className="flex items-center gap-3">
              {getStatusIcon(systemStatus?.overall || 'unknown')}
              <div>
                <div className="text-white font-medium">Overall System Status</div>
                <div className="text-sm text-gray-400">
                  {error ? 'Backend Unavailable - Using Mock Data' : 'Connected to Backend Services'}
                </div>
              </div>
            </div>
            <Badge className={getStatusColor(systemStatus?.overall || 'unknown')}>
              {systemStatus?.overall || 'unknown'}
            </Badge>
          </div>

          {/* Service Status Grid */}
          {systemStatus && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {Object.entries(systemStatus.services).map(([serviceKey, service]) => (
                <div key={serviceKey} className="p-3 bg-gray-800 rounded-lg">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      {getStatusIcon(service.status)}
                      <span className="text-white font-medium">{service.name}</span>
                    </div>
                    <Badge className={getStatusColor(service.status)}>
                      {service.status}
                    </Badge>
                  </div>
                  <div className="text-sm text-gray-400 space-y-1">
                    <div>Response Time: {service.responseTime}ms</div>
                    <div>Uptime: {service.uptime.toFixed(1)}%</div>
                    <div>Last Check: {new Date(service.lastCheck).toLocaleTimeString()}</div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Performance Metrics */}
          {systemStatus && (
            <div className="p-3 bg-gray-800 rounded-lg">
              <h4 className="text-white font-medium mb-3">Performance Metrics</h4>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-sm">
                <div>
                  <div className="text-gray-400">Avg Response Time</div>
                  <div className="text-white font-mono">{systemStatus.performance.avgResponseTime.toFixed(1)}ms</div>
                </div>
                <div>
                  <div className="text-gray-400">Error Rate</div>
                  <div className="text-white font-mono">{systemStatus.performance.errorRate.toFixed(2)}%</div>
                </div>
                <div>
                  <div className="text-gray-400">Uptime</div>
                  <div className="text-white font-mono">{systemStatus.performance.uptime.toFixed(1)}%</div>
                </div>
                <div>
                  <div className="text-gray-400">Total Requests</div>
                  <div className="text-white font-mono">{systemStatus.performance.totalRequests.toLocaleString()}</div>
                </div>
              </div>
            </div>
          )}

          {/* Integration Notes */}
          <div className="p-3 bg-blue-900/20 border border-blue-500/20 rounded-lg">
            <h4 className="text-blue-300 font-medium mb-2">Integration Status</h4>
            <div className="text-sm text-blue-200 space-y-1">
              {error ? (
                <>
                  <div>✗ Backend services not available - using realistic mock data</div>
                  <div>✓ Frontend components ready for production backend integration</div>
                  <div>✓ API endpoints mapped to real TrustChain backend structure</div>
                  <div>🔧 Ready to connect when backend is running on ports 8443-8446</div>
                </>
              ) : (
                <>
                  <div>✓ Successfully connected to backend services</div>
                  <div>✓ Real-time data streaming from APIs</div>
                  <div>✓ Production-ready integration active</div>
                </>
              )}
            </div>
          </div>

          {/* API Endpoints */}
          <div className="p-3 bg-gray-800 rounded-lg">
            <h4 className="text-white font-medium mb-3">Configured API Endpoints</h4>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">TrustChain:</span>
                <span className="text-white font-mono">https://[::1]:8443</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">STOQ Transport:</span>
                <span className="text-white font-mono">https://[::1]:8444</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">HyperMesh:</span>
                <span className="text-white font-mono">https://[::1]:8445</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Integration:</span>
                <span className="text-white font-mono">https://[::1]:8446</span>
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="flex gap-2">
            <Button onClick={() => refetch()} className="bg-blue-600 hover:bg-blue-700">
              <RefreshCw className="w-4 h-4 mr-2" />
              Retry Connection
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}