// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useShareInbox, useShareAccept, useShareReject } from '@/lib/hooks/useBlockMatrix';
import { toast } from '@/components/ui/use-toast';
import { Inbox, Check, X, Clock, HardDrive } from 'lucide-react';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

function formatTimestamp(ts: number): string {
  const date = new Date(ts * 1000);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  return date.toLocaleDateString();
}

export function InboxPanel() {
  const { data: inbox, isLoading, error } = useShareInbox();
  const acceptMutation = useShareAccept();
  const rejectMutation = useShareReject();

  const handleAccept = (inviteId: string, assetName: string) => {
    acceptMutation.mutate(inviteId, {
      onSuccess: () => {
        toast({
          title: 'Invite accepted',
          description: `Now receiving "${assetName}"`,
        });
      },
      onError: (err) => {
        toast({
          title: 'Failed to accept invite',
          description: err.message,
          variant: 'destructive',
        });
      },
    });
  };

  const handleReject = (inviteId: string) => {
    rejectMutation.mutate(inviteId, {
      onSuccess: () => {
        toast({
          title: 'Invite rejected',
          description: 'The share invite has been declined.',
        });
      },
      onError: (err) => {
        toast({
          title: 'Failed to reject invite',
          description: err.message,
          variant: 'destructive',
        });
      },
    });
  };

  const invites = inbox?.invites ?? [];
  const pendingCount = invites.length;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-white flex items-center gap-3">
          <div className="p-2 rounded-lg bg-gradient-to-r from-cyan-400 to-blue-600">
            <Inbox className="h-6 w-6 text-black" />
          </div>
          Share Inbox
          {pendingCount > 0 && (
            <Badge className="bg-cyan-500/20 text-cyan-400 border-cyan-500/30 ml-2">
              {pendingCount} pending
            </Badge>
          )}
        </h2>
        <p className="text-gray-400 mt-2">
          Received file sharing invites from other nodes
        </p>
      </div>

      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Inbox className="h-5 w-5 text-cyan-400" />
            Pending Invites
          </CardTitle>
          <CardDescription className="text-gray-400">
            Accept to begin downloading shared assets, or reject to decline
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="animate-pulse h-20 bg-gray-700 rounded-lg" />
              ))}
            </div>
          ) : error ? (
            <div className="text-center py-8">
              <p className="text-red-400">Failed to load inbox</p>
              <p className="text-xs text-gray-500 mt-1">{String(error)}</p>
            </div>
          ) : invites.length > 0 ? (
            <div className="space-y-3">
              {invites.map((invite) => {
                const isAccepting = acceptMutation.isPending && acceptMutation.variables === invite.invite_id;
                const isRejecting = rejectMutation.isPending && rejectMutation.variables === invite.invite_id;
                const isBusy = isAccepting || isRejecting;

                return (
                  <div
                    key={invite.invite_id}
                    className="border border-cyan-500/20 rounded-lg p-4 bg-cyan-500/5 hover:bg-cyan-500/10 transition-colors"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-2">
                          <h4 className="font-medium text-white truncate">
                            {invite.asset_name}
                          </h4>
                          <Badge
                            variant="outline"
                            className="text-xs bg-cyan-500/10 text-cyan-400 border-cyan-500/30 shrink-0"
                          >
                            {invite.shard_count} shards
                          </Badge>
                        </div>

                        <div className="grid grid-cols-2 md:grid-cols-3 gap-x-4 gap-y-1 text-sm">
                          <div className="flex items-center gap-1.5 text-gray-400">
                            <HardDrive className="h-3 w-3" />
                            <span>{formatBytes(invite.asset_size)}</span>
                          </div>
                          <div className="flex items-center gap-1.5 text-gray-400">
                            <Clock className="h-3 w-3" />
                            <span>{formatTimestamp(invite.created_at)}</span>
                          </div>
                          <div className="text-gray-500 font-mono text-xs truncate col-span-2 md:col-span-1">
                            from: {invite.sender_name ?? invite.sender_node_id.slice(0, 12) + '...'}
                          </div>
                        </div>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        <Button
                          size="sm"
                          className="bg-green-600 hover:bg-green-500 text-white"
                          onClick={() => handleAccept(invite.invite_id, invite.asset_name)}
                          disabled={isBusy}
                        >
                          {isAccepting ? (
                            'Accepting...'
                          ) : (
                            <>
                              <Check className="h-4 w-4 mr-1" />
                              Accept
                            </>
                          )}
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="text-red-400 hover:bg-red-500/20"
                          onClick={() => handleReject(invite.invite_id)}
                          disabled={isBusy}
                        >
                          {isRejecting ? (
                            'Rejecting...'
                          ) : (
                            <>
                              <X className="h-4 w-4 mr-1" />
                              Reject
                            </>
                          )}
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="text-center py-8">
              <Inbox className="h-12 w-12 text-gray-600 mx-auto mb-3" />
              <h3 className="text-lg font-medium text-white mb-2">No Pending Invites</h3>
              <p className="text-gray-400">
                When someone shares a file with you, it will appear here.
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
