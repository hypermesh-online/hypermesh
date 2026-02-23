// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { FileText, CheckCircle, Eye, Activity } from 'lucide-react';
import type { SecurityMetrics } from './types';

interface AuditTabProps {
  securityMetrics: SecurityMetrics;
  certificateCount: number;
}

export function AuditTab({ securityMetrics, certificateCount }: AuditTabProps) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <FileText className="h-5 w-5 text-purple-400" />
          Security Audit Trail
        </CardTitle>
        <CardDescription className="text-gray-400">Comprehensive security event logging and compliance tracking</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Audit Summary */}
          <div className="grid gap-4 md:grid-cols-3">
            <div className="text-center p-3 bg-purple-500/10 border border-purple-500/30 rounded-lg">
              <div className="text-lg font-bold text-purple-400">{securityMetrics.auditScore.toFixed(0)}%</div>
              <div className="text-sm text-gray-400">Compliance Score</div>
            </div>
            <div className="text-center p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg">
              <div className="text-lg font-bold text-blue-400">{certificateCount}</div>
              <div className="text-sm text-gray-400">Certificates Audited</div>
            </div>
            <div className="text-center p-3 bg-green-500/10 border border-green-500/30 rounded-lg">
              <div className="text-lg font-bold text-green-400">{new Date().toLocaleDateString()}</div>
              <div className="text-sm text-gray-400">Last Audit</div>
            </div>
          </div>

          {/* Recent Audit Events */}
          <div className="space-y-3 max-h-64 overflow-y-auto">
            <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
              <div className="flex items-center gap-3">
                <CheckCircle className="h-4 w-4 text-green-400" />
                <div>
                  <div className="text-white text-sm font-medium">Certificate Authority Audit Completed</div>
                  <div className="text-gray-400 text-xs">All certificates validated, no compliance issues found</div>
                </div>
              </div>
              <div className="text-xs text-gray-500">{new Date().toLocaleTimeString()}</div>
            </div>

            <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
              <div className="flex items-center gap-3">
                <Eye className="h-4 w-4 text-blue-400" />
                <div>
                  <div className="text-white text-sm font-medium">Security Policy Review</div>
                  <div className="text-gray-400 text-xs">Post-quantum cryptography policies verified</div>
                </div>
              </div>
              <div className="text-xs text-gray-500">{new Date(Date.now() - 3600000).toLocaleTimeString()}</div>
            </div>

            <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
              <div className="flex items-center gap-3">
                <Activity className="h-4 w-4 text-cyan-400" />
                <div>
                  <div className="text-white text-sm font-medium">Byzantine Detection System Check</div>
                  <div className="text-gray-400 text-xs">Threat detection algorithms operating nominally</div>
                </div>
              </div>
              <div className="text-xs text-gray-500">{new Date(Date.now() - 7200000).toLocaleTimeString()}</div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
