// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useNetworkPeers,
  useAssetList,
  useShareInbox,
  useShareAccept,
  useShareReject,
} from '@/lib/hooks/useBlockMatrix';
import { ShareDialog } from '@/components/sharing/ShareDialog';
import {
  Share,
  Users,
  Send,
  Inbox,
  AlertTriangle,
  CheckCircle,
  XCircle,
} from 'lucide-react';

export function SharingManagement() {
  const { data: peers, isLoading: peersLoading, error: peersError } = useNetworkPeers();
  const { data: blockchainAssets, isLoading: assetsLoading } = useAssetList();
  const { data: inbox, isLoading: inboxLoading } = useShareInbox();
  const shareAccept = useShareAccept();
  const shareReject = useShareReject();

  const [shareTarget, setShareTarget] = React.useState<{ id: string; name: string } | null>(null);

  const isLoading = peersLoading && assetsLoading && inboxLoading;
  if (isLoading) return <ModuleLoading />;

  if (peersError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{peersError.message}</p>
        </CardContent>
      </Card>
    );
  }

  const invites = inbox?.invites ?? [];

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Sharing Management</h2>

      {/* Summary cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Connected Peers</CardTitle>
            <Users className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{peers?.length ?? 0}</div>
            <p className="text-xs text-gray-400">Available for sharing</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Shareable Assets</CardTitle>
            <Share className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{blockchainAssets?.length ?? 0}</div>
            <p className="text-xs text-gray-400">Registered on-chain</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Pending Invites</CardTitle>
            <Inbox className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-yellow-400">{invites.length}</div>
            <p className="text-xs text-gray-400">Awaiting your response</p>
          </CardContent>
        </Card>
      </div>

      {/* Share Inbox */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Inbox className="h-5 w-5 text-cyan-400" />
            Share Inbox
          </CardTitle>
          <CardDescription className="text-gray-400">
            Incoming file sharing invitations from other nodes
          </CardDescription>
        </CardHeader>
        <CardContent>
          {inboxLoading ? (
            <div className="space-y-3">
              {[1, 2].map((i) => (
                <div key={i} className="animate-pulse h-16 bg-gray-700 rounded-lg" />
              ))}
            </div>
          ) : invites.length > 0 ? (
            <div className="space-y-3">
              {invites.map((invite) => (
                <div
                  key={invite.invite_id}
                  className="flex items-center justify-between p-4 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-white font-medium truncate">
                      {invite.asset_name}
                    </p>
                    <p className="text-xs text-gray-400">
                      From: {invite.sender_node_id.slice(0, 16)}... | {invite.shard_count} shards |{' '}
                      {(invite.asset_size / 1024).toFixed(1)} KB
                    </p>
                  </div>
                  <div className="flex items-center gap-2 ml-3">
                    <Button
                      size="sm"
                      className="bg-green-600 hover:bg-green-500 text-white"
                      onClick={() => shareAccept.mutate(invite.invite_id)}
                      disabled={shareAccept.isPending}
                    >
                      <CheckCircle className="h-3 w-3 mr-1" />
                      Accept
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="border-red-500/30 text-red-400 hover:bg-red-500/10"
                      onClick={() => shareReject.mutate(invite.invite_id)}
                      disabled={shareReject.isPending}
                    >
                      <XCircle className="h-3 w-3 mr-1" />
                      Reject
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-gray-400">
              <Inbox className="h-10 w-10 text-gray-600 mx-auto mb-2" />
              <p>No pending invitations</p>
              <p className="text-xs text-gray-500 mt-1">
                Invites from other nodes will appear here
              </p>
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
          {peers && peers.length > 0 ? (
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
              <p className="text-xs text-gray-500 mt-1">
                Peers will appear when other nodes join the network
              </p>
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
              <p className="text-xs text-gray-500 mt-1">
                Assets will appear after they are registered on the blockchain
              </p>
            </div>
          )}
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
