// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

import React, { useState, useRef, useEffect } from 'react';
import { cn } from '@/lib/utils';
import { useMessageHistory, useMessageSend } from '@/lib/hooks/useBlockMatrix';
import { Send } from 'lucide-react';
import { toast } from '@/components/ui/use-toast';

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
  });
}

interface MessageThreadProps {
  peer: string;
}

export function MessageThread({ peer }: MessageThreadProps) {
  const { data: history, isLoading, error } = useMessageHistory(peer);
  const sendMutation = useMessageSend();
  const [body, setBody] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [history?.messages?.length]);

  const handleSend = () => {
    const trimmed = body.trim();
    if (!trimmed) return;

    sendMutation.mutate(
      { recipient: peer, body: trimmed },
      {
        onSuccess: () => {
          setBody('');
        },
        onError: (err) => {
          toast({
            title: 'Failed to send message',
            description: err.message,
            variant: 'destructive',
          });
        },
      },
    );
  };

  const messages = history?.messages ?? [];

  return (
    <div className="flex-1 flex flex-col min-w-0">
      {/* Header */}
      <div className="p-3 border-b border-cyan-500/20 bg-black/20">
        <div className="font-medium text-white text-sm truncate">
          {peer.slice(0, 24)}{peer.length > 24 ? '...' : ''}
        </div>
        <div className="text-[10px] text-gray-500 font-mono mt-0.5">
          {messages.length} message{messages.length !== 1 ? 's' : ''}
        </div>
      </div>

      {/* Messages area */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto p-4 space-y-3"
        role="log"
        aria-label="Message history"
      >
        {isLoading ? (
          <div className="space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="animate-pulse h-12 bg-gray-800 rounded-lg w-2/3" />
            ))}
          </div>
        ) : error ? (
          <div className="text-center py-8">
            <p className="text-red-400 text-sm">Failed to load messages</p>
            <p className="text-xs text-gray-500 mt-1">{String(error)}</p>
          </div>
        ) : messages.length > 0 ? (
          messages.map((msg) => {
            const isSent = msg.sender_node_id !== peer;
            return (
              <div
                key={msg.message_id}
                className={cn('flex', isSent ? 'justify-end' : 'justify-start')}
              >
                <div
                  className={cn(
                    'max-w-[70%] px-3 py-2 rounded-lg text-sm',
                    isSent
                      ? 'bg-cyan-600/80 text-white'
                      : 'bg-gray-700/80 text-gray-100',
                  )}
                >
                  <div className="break-words whitespace-pre-wrap">
                    {msg.body || '[encrypted]'}
                  </div>
                  <div
                    className={cn(
                      'text-[10px] mt-1',
                      isSent ? 'text-cyan-200/60' : 'text-gray-400',
                    )}
                  >
                    {formatTime(msg.created_at)}
                  </div>
                </div>
              </div>
            );
          })
        ) : (
          <div className="text-center py-8 text-gray-500 text-sm">
            No messages yet. Send one to start the conversation.
          </div>
        )}
      </div>

      {/* Compose area */}
      <div className="p-3 border-t border-cyan-500/20 bg-black/20">
        <div className="flex gap-2">
          <input
            type="text"
            value={body}
            onChange={(e) => setBody(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Type a message..."
            disabled={sendMutation.isPending}
            className="flex-1 px-3 py-2 rounded-lg border border-cyan-500/30 bg-black/40 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400/60 disabled:opacity-50"
            aria-label="Message input"
          />
          <button
            onClick={handleSend}
            disabled={!body.trim() || sendMutation.isPending}
            className="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-1.5"
            aria-label="Send message"
          >
            <Send className="h-4 w-4" />
            <span className="hidden sm:inline">Send</span>
          </button>
        </div>
      </div>
    </div>
  );
}
