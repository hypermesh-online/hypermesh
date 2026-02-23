// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { AlertTriangle, Shield } from 'lucide-react';
import type { SecurityAlert } from './types';

interface AlertsTabProps {
  securityAlerts: SecurityAlert[];
}

export function AlertsTab({ securityAlerts }: AlertsTabProps) {
  return (
    <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <AlertTriangle className="h-5 w-5 text-red-400" />
          Active Security Alerts
        </CardTitle>
        <CardDescription className="text-gray-400">Critical security events requiring immediate attention</CardDescription>
      </CardHeader>
      <CardContent>
        {securityAlerts.length > 0 ? (
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {securityAlerts.map((alert) => (
              <div key={alert.id} className={cn(
                'flex items-center justify-between p-3 rounded-lg border',
                alert.severity === 'critical' ? 'bg-red-500/10 border-red-500/30' :
                alert.severity === 'high' ? 'bg-orange-500/10 border-orange-500/30' :
                alert.severity === 'medium' ? 'bg-yellow-500/10 border-yellow-500/30' :
                'bg-blue-500/10 border-blue-500/30'
              )}>
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <AlertTriangle className={cn(
                      'h-4 w-4',
                      alert.severity === 'critical' ? 'text-red-400' :
                      alert.severity === 'high' ? 'text-orange-400' :
                      alert.severity === 'medium' ? 'text-yellow-400' :
                      'text-blue-400'
                    )} />
                    <span className="text-white font-medium">{alert.title}</span>
                    <Badge variant="outline" className={cn(
                      'text-xs',
                      alert.severity === 'critical' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                      alert.severity === 'high' ? 'bg-orange-500/20 text-orange-400 border-orange-500/30' :
                      alert.severity === 'medium' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                      'bg-blue-500/20 text-blue-400 border-blue-500/30'
                    )}>
                      {alert.severity}
                    </Badge>
                    <Badge variant="outline" className="text-xs bg-gray-500/20 text-gray-400">
                      {alert.type}
                    </Badge>
                  </div>
                  <p className="text-sm text-gray-400">{alert.description}</p>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-gray-500">
                    {new Date(alert.timestamp).toLocaleTimeString()}
                  </span>
                  <Button variant="ghost" size="sm" className="text-cyan-400 hover:bg-cyan-500/20">
                    Resolve
                  </Button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Shield className="h-12 w-12 text-green-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">All Clear</h3>
            <p className="text-gray-400">No active security alerts. System is operating securely.</p>
            <div className="mt-4 text-sm text-green-400">
              Certificate health is good<br />
              No Byzantine threats detected<br />
              All services operating normally
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
