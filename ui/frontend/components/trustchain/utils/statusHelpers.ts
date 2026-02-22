// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export function getStatusColor(status: string): string {
  const colors: Record<string, string> = {
    'Connected': 'text-green-400 bg-green-500/20 border-green-500/30',
    'Connecting': 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30',
    'Disconnected': 'text-gray-400 bg-gray-500/20 border-gray-500/30',
    'Error': 'text-red-400 bg-red-500/20 border-red-500/30',
    'online': 'text-green-400 bg-green-500/20 border-green-500/30',
    'warning': 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30',
    'offline': 'text-gray-400 bg-gray-500/20 border-gray-500/30',
    'active': 'bg-green-500/20 text-green-400 border-green-500/30',
    'expired': 'bg-red-500/20 text-red-400 border-red-500/30',
    'revoked': 'bg-red-500/20 text-red-400 border-red-500/30'
  };
  return colors[status] || 'bg-gray-500/20 text-gray-400 border-gray-500/30';
}

export function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    'Public': 'text-cyan-400 bg-cyan-500/20 border-cyan-500/30',
    'P2P': 'text-purple-400 bg-purple-500/20 border-purple-500/30',
    'Federated': 'text-blue-400 bg-blue-500/20 border-blue-500/30'
  };
  return colors[type] || 'text-gray-400 bg-gray-500/20 border-gray-500/30';
}

export function getTrustLevelColor(level: string): string {
  const colors: Record<string, string> = {
    'root': 'text-green-400',
    'intermediate': 'text-blue-400',
    'end-entity': 'text-purple-400'
  };
  return colors[level] || 'text-gray-400';
}
