// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

import React, { useState, useMemo } from 'react';
import { MessageSquare } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { useMessageInbox } from '@/lib/hooks/useBlockMatrix';
import type { MessageItem } from '@/lib/blockmatrix-api';
import { ConversationList } from './ConversationList';
import { MessageThread } from './MessageThread';

export function MessengerPanel() {
  const [selectedPeer, setSelectedPeer] = useState<string | null>(null);
  const { data: inbox, isLoading, error } = useMessageInbox();

  // Extract unique peers from inbox, keeping the latest message per peer
  const peers = useMemo((): [string, MessageItem][] => {
    if (!inbox?.messages) return [];

    const peerMap = new Map<string, MessageItem>();
    for (const msg of inbox.messages) {
      // Use sender for received messages, recipient for sent messages
      const peerId = msg.sender_node_id === 'self'
        ? msg.recipient_node_id
        : msg.sender_node_id;

      const existing = peerMap.get(peerId);
      if (!existing || msg.created_at > existing.created_at) {
        peerMap.set(peerId, msg);
      }
    }

    return Array.from(peerMap.entries())
      .sort((a, b) => b[1].created_at - a[1].created_at);
  }, [inbox]);

  const messageCount = inbox?.count ?? 0;

  return (
    <div className="space-y-6">
      {/* Page header */}
      <div>
        <h2 className="text-2xl font-bold text-white flex items-center gap-3">
          <div className="p-2 rounded-lg bg-gradient-to-r from-cyan-400 to-blue-600">
            <MessageSquare className="h-6 w-6 text-black" />
          </div>
          Messages
          {messageCount > 0 && (
            <Badge className="bg-cyan-500/20 text-cyan-400 border-cyan-500/30 ml-2">
              {messageCount}
            </Badge>
          )}
        </h2>
        <p className="text-gray-400 mt-2">
          Private peer-to-peer messaging over the HyperMesh network
        </p>
      </div>

      {/* Messenger layout */}
      <div className="border border-cyan-500/20 rounded-lg overflow-hidden bg-black/40 backdrop-blur-lg h-[calc(100vh-220px)] min-h-[400px] flex">
        {isLoading ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-gray-400 text-sm">Loading messages...</div>
          </div>
        ) : error ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <p className="text-red-400 text-sm">Failed to load messages</p>
              <p className="text-xs text-gray-500 mt-1">{String(error)}</p>
            </div>
          </div>
        ) : (
          <>
            <ConversationList
              peers={peers}
              selected={selectedPeer}
              onSelect={setSelectedPeer}
            />
            {selectedPeer ? (
              <MessageThread peer={selectedPeer} />
            ) : (
              <div className="flex-1 flex items-center justify-center text-gray-500">
                <div className="text-center">
                  <MessageSquare className="h-12 w-12 text-gray-600 mx-auto mb-3" />
                  <h3 className="text-lg font-medium text-white mb-2">No Conversation Selected</h3>
                  <p className="text-gray-400 text-sm">
                    Select a conversation or enter a node ID to start messaging.
                  </p>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
