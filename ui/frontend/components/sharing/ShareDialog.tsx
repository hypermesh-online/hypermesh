// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

import React, { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useShareSend } from '@/lib/hooks/useBlockMatrix';
import { toast } from '@/components/ui/use-toast';
import { Send, X } from 'lucide-react';

interface ShareDialogProps {
  assetId: string;
  assetName: string;
  isOpen: boolean;
  onClose: () => void;
}

export function ShareDialog({ assetId, assetName, isOpen, onClose }: ShareDialogProps) {
  const [recipient, setRecipient] = useState('');
  const shareSend = useShareSend();

  if (!isOpen) return null;

  const handleShare = () => {
    if (!recipient.trim()) return;

    shareSend.mutate(
      { assetId, recipient: recipient.trim() },
      {
        onSuccess: () => {
          toast({
            title: 'Share invite sent',
            description: `Invited ${recipient} to access "${assetName}"`,
          });
          setRecipient('');
          onClose();
        },
        onError: (error) => {
          toast({
            title: 'Failed to send invite',
            description: error.message,
            variant: 'destructive',
          });
        },
      }
    );
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && recipient.trim()) {
      handleShare();
    }
    if (e.key === 'Escape') {
      onClose();
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      role="dialog"
      aria-modal="true"
      aria-label={`Share asset ${assetName}`}
    >
      <div className="w-full max-w-md rounded-lg border border-cyan-500/30 bg-black/90 p-6 shadow-xl">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">Share Asset</h3>
          <Button
            variant="ghost"
            size="sm"
            className="text-gray-400 hover:text-white"
            onClick={onClose}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        <div className="mb-4 p-3 rounded-md border border-cyan-500/20 bg-cyan-500/5">
          <p className="text-xs text-gray-400 mb-1">Asset</p>
          <p className="text-sm font-mono text-cyan-400 truncate">{assetName}</p>
          <p className="text-xs text-gray-500 font-mono mt-1 truncate">{assetId}</p>
        </div>

        <div className="mb-6">
          <label
            htmlFor="share-recipient"
            className="block text-sm font-medium text-gray-300 mb-2"
          >
            Recipient Node ID
          </label>
          <Input
            id="share-recipient"
            type="text"
            placeholder="Enter node ID or peer name"
            value={recipient}
            onChange={(e) => setRecipient(e.target.value)}
            onKeyDown={handleKeyDown}
            className="bg-black/40 border-gray-600 text-white placeholder:text-gray-500 focus-visible:border-cyan-500 focus-visible:ring-cyan-500/30"
            autoFocus
          />
        </div>

        <div className="flex justify-end gap-3">
          <Button
            variant="ghost"
            className="text-gray-400 hover:text-white"
            onClick={onClose}
          >
            Cancel
          </Button>
          <Button
            className="bg-cyan-600 hover:bg-cyan-500 text-white"
            onClick={handleShare}
            disabled={!recipient.trim() || shareSend.isPending}
          >
            {shareSend.isPending ? (
              'Sending...'
            ) : (
              <>
                <Send className="h-4 w-4 mr-2" />
                Share
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
