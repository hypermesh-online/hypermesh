// @ts-nocheck — Phase 8 will rewrite with useBlockMatrix hooks
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useAssets, useAllocations } from '@/lib/api';
import { useNetworkPeers, useAssetList } from '@/lib/hooks/useBlockMatrix';
import { ShareDialog } from '@/components/sharing/ShareDialog';
import {
  Settings,
  Share,
  Shield,
  Zap,
  Activity,
  Users,
  Send
} from 'lucide-react';

export function SharingManagement() {
  const { allocations, activeAllocations, isLoading } = useAllocations();
  const { assets } = useAssets();
  const { data: peers, isLoading: peersLoading } = useNetworkPeers();
  const { data: blockchainAssets } = useAssetList();
  const [selectedAllocation, setSelectedAllocation] = React.useState<string | null>(null);
  const [shareTarget, setShareTarget] = React.useState<{ id: string; name: string } | null>(null);

  const sharingStats = React.useMemo(() => {
    const total = allocations?.length || 0;
    const active = activeAllocations?.length || 0;
    const pending = allocations?.filter(a => a.status === 'pending').length || 0;
    const completed = allocations?.filter(a => a.status === 'completed').length || 0;

    return { total, active, pending, completed };
  }, [allocations, activeAllocations]);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Sharing Management</h2>

      {/* Sharing Statistics */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Allocations</CardTitle>
            <Activity className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{sharingStats.total}</div>
            <p className="text-xs text-gray-400">All time</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Active</CardTitle>
            <Zap className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{sharingStats.active}</div>
            <p className="text-xs text-gray-400">Currently sharing</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Pending</CardTitle>
            <Settings className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-yellow-400">{sharingStats.pending}</div>
            <p className="text-xs text-gray-400">Awaiting approval</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Completed</CardTitle>
            <Shield className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{sharingStats.completed}</div>
            <p className="text-xs text-gray-400">Successfully shared</p>
          </CardContent>
        </Card>
      </div>

      {/* Active Allocations Management */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Share className="h-5 w-5 text-cyan-400" />
            Active Resource Sharing
          </CardTitle>
          <CardDescription className="text-gray-400">Manage ongoing resource allocations and sharing sessions</CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-3">
              {[1,2,3].map(i => (
                <div key={i} className="animate-pulse h-20 bg-gray-700 rounded-lg"></div>
              ))}
            </div>
          ) : activeAllocations && activeAllocations.length > 0 ? (
            <div className="space-y-4">
              {activeAllocations.map((allocation) => (
                <div
                  key={allocation.id}
                  className="border border-cyan-500/20 rounded-lg p-4 bg-cyan-500/5 hover:bg-cyan-500/10 transition-colors cursor-pointer"
                  onClick={() => setSelectedAllocation(allocation.id)}
                >
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <h4 className="font-medium text-white">Allocation {allocation.id.slice(0, 8)}...</h4>
                      <Badge
                        variant="outline"
                        className={`text-xs ${
                          allocation.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                          allocation.status === 'pending' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                          'bg-cyan-500/20 text-cyan-400 border-cyan-500/30'
                        }`}
                      >
                        {allocation.status}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button variant="ghost" size="sm" className="text-cyan-400 hover:bg-cyan-500/20">
                        <Settings className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="text-red-400 hover:bg-red-500/20"
                        onClick={(e) => {
                          e.stopPropagation();
                          // In production, this would call releaseAllocation API
                          alert(`Terminating allocation ${allocation.id}`);
                        }}
                      >
                        Terminate
                      </Button>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                    <div>
                      <span className="text-gray-400">Resource:</span>
                      <div className="text-white font-mono">{allocation.amount} {allocation.unit}</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Duration:</span>
                      <div className="text-white font-mono">{Math.floor(allocation.duration / 3600)}h</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Requester:</span>
                      <div className="text-white font-mono">{allocation.requesterId.slice(0, 8)}...</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Started:</span>
                      <div className="text-white font-mono">{new Date(allocation.startTime).toLocaleTimeString()}</div>
                    </div>
                  </div>

                  {allocation.proxyAddress && (
                    <div className="mt-3 pt-3 border-t border-cyan-500/20">
                      <span className="text-gray-400 text-sm">Proxy Address:</span>
                      <div className="text-cyan-400 font-mono text-sm">{allocation.proxyAddress}</div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8">
              <Share className="h-12 w-12 text-gray-600 mx-auto mb-3" />
              <h3 className="text-lg font-medium text-white mb-2">No Active Sharing</h3>
              <p className="text-gray-400">Your resources are available but not currently being used by others.</p>
              <p className="text-sm text-gray-500 mt-2">Configure resource limits in the Resources tab to start sharing.</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Connected Peers */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Users className="h-5 w-5 text-cyan-400" />
            Connected Peers
          </CardTitle>
          <CardDescription className="text-gray-400">
            Peers available for resource sharing ({peers?.length ?? 0} connected)
          </CardDescription>
        </CardHeader>
        <CardContent>
          {peersLoading ? (
            <div className="space-y-3">
              {[1,2,3].map(i => (
                <div key={i} className="animate-pulse h-14 bg-gray-700 rounded-lg"></div>
              ))}
            </div>
          ) : peers && peers.length > 0 ? (
            <div className="space-y-2">
              {peers.map((peer) => (
                <div
                  key={peer.node_id}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex items-center gap-3">
                    <div className="w-2 h-2 bg-green-400 rounded-full" />
                    <div>
                      <p className="text-sm font-mono text-white">{peer.node_id.slice(0, 16)}...</p>
                      <p className="text-xs text-gray-400">{peer.address}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {peer.coordinate && (
                      <Badge variant="outline" className="text-xs bg-cyan-500/10 text-cyan-400 border-cyan-500/30">
                        ({peer.coordinate.x},{peer.coordinate.y},{peer.coordinate.z})
                      </Badge>
                    )}
                    <Badge className="bg-green-500/20 text-green-400 border-green-500/30">
                      Connected
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-gray-400">
              <Users className="h-10 w-10 text-gray-600 mx-auto mb-2" />
              <p>No peers connected</p>
              <p className="text-xs text-gray-500 mt-1">Peers will appear when other nodes join the network</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Share an Asset */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Send className="h-5 w-5 text-cyan-400" />
            Share an Asset
          </CardTitle>
          <CardDescription className="text-gray-400">
            Send a file sharing invite to another node on the network
          </CardDescription>
        </CardHeader>
        <CardContent>
          {blockchainAssets && blockchainAssets.length > 0 ? (
            <div className="space-y-2">
              {blockchainAssets.slice(0, 10).map((asset) => (
                <div
                  key={asset.id}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-white truncate font-mono">{asset.id.slice(0, 24)}...</p>
                    <p className="text-xs text-gray-400">{asset.category} - block #{asset.block_index}</p>
                  </div>
                  <Button
                    size="sm"
                    className="bg-cyan-600 hover:bg-cyan-500 text-white shrink-0 ml-3"
                    onClick={() => setShareTarget({ id: asset.id, name: asset.id.slice(0, 16) })}
                  >
                    <Share className="h-3 w-3 mr-1" />
                    Share
                  </Button>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-gray-400">
              <Share className="h-10 w-10 text-gray-600 mx-auto mb-2" />
              <p>No assets available to share</p>
              <p className="text-xs text-gray-500 mt-1">Assets will appear after they are registered on the blockchain</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Resource Allocation History */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Allocation History</CardTitle>
          <CardDescription className="text-gray-400">Recent resource sharing activity and completed allocations</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {allocations && allocations.length > 0 ? (
              allocations.slice(0, 5).map((allocation) => (
                <div key={allocation.id} className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-white font-mono text-sm">{allocation.id.slice(0, 12)}...</span>
                      <Badge
                        variant="outline"
                        className={`text-xs ${
                          allocation.status === 'completed' ? 'bg-green-500/20 text-green-400' :
                          allocation.status === 'cancelled' ? 'bg-red-500/20 text-red-400' :
                          allocation.status === 'failed' ? 'bg-red-500/20 text-red-400' :
                          'bg-gray-500/20 text-gray-400'
                        }`}
                      >
                        {allocation.status}
                      </Badge>
                    </div>
                    <div className="text-sm text-gray-400">
                      {allocation.amount} {allocation.unit} - {Math.floor(allocation.duration / 3600)}h duration
                    </div>
                  </div>
                  <div className="text-xs text-gray-500">
                    {new Date(allocation.startTime).toLocaleDateString()}
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-6 text-gray-400">
                No allocation history available
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Share Dialog */}
      {shareTarget && (
        <ShareDialog
          assetId={shareTarget.id}
          assetName={shareTarget.name}
          isOpen={true}
          onClose={() => setShareTarget(null)}
        />
      )}
    </div>
  );
}
