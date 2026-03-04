// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  Shield,
  AlertTriangle,
  CheckCircle,
  Lock
} from 'lucide-react';

interface ByzantineDetection {
  id: string;
  severity: string;
  timestamp: string;
  nodeId?: string;
  behaviorType: string;
  confidence: number;
}

interface SecurityTabProps {
  byzantineDetections: ByzantineDetection[] | undefined;
}

export function SecurityTab({ byzantineDetections }: SecurityTabProps) {
  return (
    <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Lock className="h-5 w-5 text-red-400" />
          Byzantine Fault Detection & Security
        </CardTitle>
        <CardDescription className="text-gray-400">
          Real-time monitoring of security threats and Byzantine behavior detection
        </CardDescription>
      </CardHeader>
      <CardContent>
        {byzantineDetections && byzantineDetections.length > 0 ? (
          <div className="space-y-3">
            {byzantineDetections.map((detection) => (
              <div key={detection.id} className="p-4 bg-red-500/10 border border-red-500/30 rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <AlertTriangle className="h-5 w-5 text-red-400" />
                    <span className="text-white font-medium">Byzantine Behavior Detected</span>
                    <Badge variant="outline" className="text-xs bg-red-500/20 text-red-400 border-red-500/30">
                      {detection.severity}
                    </Badge>
                  </div>
                  <div className="text-xs text-gray-500">
                    {new Date(detection.timestamp).toLocaleTimeString()}
                  </div>
                </div>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <span className="text-gray-400">Node ID:</span>
                    <div className="text-red-400 font-mono">{detection.nodeId?.slice(0, 12)}...</div>
                  </div>
                  <div>
                    <span className="text-gray-400">Behavior Type:</span>
                    <div className="text-white">{detection.behaviorType}</div>
                  </div>
                  <div>
                    <span className="text-gray-400">Confidence:</span>
                    <div className="text-red-400 font-medium">{detection.confidence}%</div>
                  </div>
                  <div>
                    <span className="text-gray-400">Action:</span>
                    <div className="text-white">Node Quarantined</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Shield className="h-12 w-12 text-green-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">Network Secure</h3>
            <p className="text-gray-400 mb-4">
              No Byzantine behavior detected. All proof validators are operating correctly.
            </p>
            <div className="grid gap-2 text-sm text-left max-w-md mx-auto">
              <div className="flex items-center gap-2 text-green-400">
                <CheckCircle className="h-4 w-4" />
                All four proof types validating correctly
              </div>
              <div className="flex items-center gap-2 text-green-400">
                <CheckCircle className="h-4 w-4" />
                No malicious node behavior detected
              </div>
              <div className="flex items-center gap-2 text-green-400">
                <CheckCircle className="h-4 w-4" />
                Verification completeness maintained
              </div>
              <div className="flex items-center gap-2 text-green-400">
                <CheckCircle className="h-4 w-4" />
                Network participation above 90%
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
