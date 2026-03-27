// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useTopologyInfo,
  useTopologyNeighbors,
} from '@/lib/hooks/useBlockMatrix';
import {
  Map,
  AlertTriangle,
  Navigation,
  Compass,
} from 'lucide-react';

export function TopologyView() {
  const { data: topology, isLoading: topoLoading, error: topoError } = useTopologyInfo();
  const { data: neighbors, isLoading: neighborsLoading } = useTopologyNeighbors();

  if (topoLoading) return <ModuleLoading />;

  if (topoError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{topoError.message}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Matrix Topology</h2>

      {/* This node */}
      {topology && (
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Compass className="h-5 w-5 text-cyan-400" />
              This Node
            </CardTitle>
            <CardDescription className="text-gray-400">
              Your position in the Block-MATRIX
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">Matrix Coordinate</span>
                  <span className="text-cyan-400 font-mono text-lg">
                    ({topology.coordinate.x}, {topology.coordinate.y}, {topology.coordinate.z})
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Node ID</span>
                  <span className="text-white font-mono text-sm truncate max-w-[200px]">
                    {topology.node_id}
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-center">
                <div className="text-center p-4 rounded-lg border border-cyan-500/20 bg-cyan-500/5">
                  <Map className="h-8 w-8 text-cyan-400 mx-auto mb-2" />
                  <p className="text-xs text-gray-400">
                    Position determines routing, shard placement, and neighbor relationships
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Neighbors */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Navigation className="h-5 w-5 text-cyan-400" />
            Matrix Neighbors
          </CardTitle>
          <CardDescription className="text-gray-400">
            Nodes adjacent in the matrix topology ({neighbors?.length ?? 0} neighbors)
          </CardDescription>
        </CardHeader>
        <CardContent>
          {neighborsLoading ? (
            <div className="space-y-2">
              {[1, 2, 3].map((i) => (
                <div key={i} className="animate-pulse h-12 bg-gray-700 rounded-lg" />
              ))}
            </div>
          ) : neighbors && neighbors.length > 0 ? (
            <div className="space-y-2">
              {neighbors.map((neighbor, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex items-center gap-3">
                    <Navigation className="h-4 w-4 text-cyan-400" />
                    <span className="text-white font-mono">
                      ({neighbor.coordinate.x}, {neighbor.coordinate.y}, {neighbor.coordinate.z})
                    </span>
                  </div>
                  <Badge variant="outline" className="text-xs bg-cyan-500/10 text-cyan-400 border-cyan-500/30">
                    Distance: {neighbor.distance.toFixed(2)}
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-gray-400">
              <Navigation className="h-10 w-10 text-gray-600 mx-auto mb-2" />
              <p>No neighbors discovered</p>
              <p className="text-xs text-gray-500 mt-1">
                Neighbors appear when other nodes join the network
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
