// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

import React, { useState } from 'react';
import { cn } from '@/lib/utils';
import type { MessageItem } from '@/lib/blockmatrix-api';

function formatTimestamp(ts: number): string {
  const date = new Date(ts * 1000);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);

  if (diffMins < 1) return 'Now';
  if (diffMins < 60) return `${diffMins}m`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h`;
  return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

interface ConversationListProps {
  peers: [string, MessageItem][];
  selected: string | null;
  onSelect: (peer: string) => void;
}

export function ConversationList({ peers, selected, onSelect }: ConversationListProps) {
  const [newRecipient, setNewRecipient] = useState('');

  const handleNewConversation = () => {
    const trimmed = newRecipient.trim();
    if (trimmed) {
      onSelect(trimmed);
      setNewRecipient('');
    }
  };

  return (
    <div className="w-72 border-r border-cyan-500/20 flex flex-col bg-black/20">
      {/* New conversation input */}
      <div className="p-3 border-b border-cyan-500/20">
        <input
          type="text"
          placeholder="New message to node ID..."
          value={newRecipient}
          onChange={(e) => setNewRecipient(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleNewConversation();
          }}
          className="w-full px-3 py-2 rounded-lg border border-cyan-500/30 bg-black/40 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400/60"
          aria-label="Start new conversation with node ID"
        />
      </div>

      {/* Peer list */}
      <div className="flex-1 overflow-y-auto">
        {peers.map(([peerId, lastMsg]) => (
          <button
            key={peerId}
            onClick={() => onSelect(peerId)}
            className={cn(
              'w-full p-3 text-left border-b border-cyan-500/10 transition-colors',
              selected === peerId
                ? 'bg-cyan-500/15 border-l-2 border-l-cyan-400'
                : 'hover:bg-cyan-500/5',
            )}
            aria-label={`Conversation with ${lastMsg.sender_name ?? peerId.slice(0, 12)}`}
            aria-current={selected === peerId ? 'true' : undefined}
          >
            <div className="flex items-center justify-between gap-2">
              <span className="font-medium text-sm text-white truncate">
                {lastMsg.sender_name ?? peerId.slice(0, 12)}
              </span>
              <span className="text-[10px] text-gray-500 shrink-0">
                {formatTimestamp(lastMsg.created_at)}
              </span>
            </div>
            <div className="text-xs text-gray-400 truncate mt-1">
              {lastMsg.body?.slice(0, 60) || '[encrypted]'}
            </div>
          </button>
        ))}

        {peers.length === 0 && (
          <div className="p-4 text-sm text-gray-500 text-center">
            No conversations yet. Enter a node ID above to start messaging.
          </div>
        )}
      </div>
    </div>
  );
}
