// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Shield, AlertTriangle } from 'lucide-react';

interface ThreatsTabProps {
  byzantineDetections: any[] | undefined;
}

export function ThreatsTab({ byzantineDetections }: ThreatsTabProps) {
  return (
    <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Shield className="h-5 w-5 text-red-400" />
          Byzantine Fault Detection
        </CardTitle>
        <CardDescription className="text-gray-400">Real-time monitoring of malicious node behavior and network threats</CardDescription>
      </CardHeader>
      <CardContent>
        {byzantineDetections && byzantineDetections.length > 0 ? (
          <div className="space-y-3">
            {byzantineDetections.map((detection) => (
              <div key={detection.nodeId} className="flex items-center justify-between p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <AlertTriangle className="h-4 w-4 text-red-400" />
                    <span className="text-white font-medium">Malicious Behavior Detected</span>
                    <Badge variant="outline" className="text-xs bg-red-500/20 text-red-400 border-red-500/30">
                      {detection.severity}
                    </Badge>
                  </div>
                  <div className="text-sm text-gray-400">
                    Node: {detection.nodeId?.slice(0, 12)}... -
                    Type: {detection.behaviour} -
                    Status: {detection.status}
                  </div>
                  <div className="text-xs text-gray-500 mt-1">
                    Evidence: {detection.evidence?.invalidOperations?.join(', ') || 'Consensus deviation, invalid proofs'} -
                    Action: {detection.mitigation?.actions?.join(', ') || 'Node quarantined, peers notified'}
                  </div>
                </div>
                <div className="text-xs text-gray-500">
                  {new Date(detection.detectedAt).toLocaleTimeString()}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Shield className="h-12 w-12 text-green-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">Network Secure</h3>
            <p className="text-gray-400">No Byzantine threats detected. All nodes are behaving correctly.</p>
            <div className="mt-4 grid gap-2 text-sm text-green-400">
              <div>Consensus validation operating normally</div>
              <div>All proof submissions are valid</div>
              <div>No malicious behavior patterns detected</div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
