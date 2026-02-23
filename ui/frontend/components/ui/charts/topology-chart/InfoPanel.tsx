// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import type { TopologyNode, TopologyLink } from './types';

interface InfoPanelProps {
  selectedNode: string | null;
  selectedLink: string | null;
  nodes: TopologyNode[];
  links: TopologyLink[];
}

export function InfoPanel({ selectedNode, selectedLink, nodes, links }: InfoPanelProps) {
  if (!selectedNode && !selectedLink) return null;

  return (
    <div className="absolute top-4 right-4 bg-black/90 border border-gray-600 rounded-lg p-4 text-sm text-white max-w-xs">
      {selectedNode && (() => {
        const node = nodes.find(n => n.id === selectedNode);
        if (!node) return null;

        return (
          <div className="space-y-2">
            <h4 className="font-medium text-lg">{node.label}</h4>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div>
                <span className="text-gray-400">Type:</span>
                <span className="ml-1 capitalize">{node.type}</span>
              </div>
              <div>
                <span className="text-gray-400">Status:</span>
                <span className="ml-1 capitalize">{node.status}</span>
              </div>
            </div>

            {node.metrics && (
              <div className="space-y-1">
                <h5 className="font-medium text-xs text-gray-300">Metrics</h5>
                {Object.entries(node.metrics).map(([key, value]) => (
                  <div key={key} className="flex justify-between text-xs">
                    <span className="text-gray-400 capitalize">{key}:</span>
                    <span className="text-cyan-400">{value}{key === 'load' ? '%' : ''}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })()}

      {selectedLink && (() => {
        const link = links.find(l => `${l.source}-${l.target}` === selectedLink);
        if (!link) return null;

        return (
          <div className="space-y-2">
            <h4 className="font-medium text-lg">Link Details</h4>
            <div className="text-xs">
              <div className="mb-2">
                <span className="text-gray-400">Route:</span>
                <span className="ml-1 font-mono">{link.source} &rarr; {link.target}</span>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <span className="text-gray-400">Type:</span>
                  <span className="ml-1 capitalize">{link.type}</span>
                </div>
                <div>
                  <span className="text-gray-400">Status:</span>
                  <span className="ml-1 capitalize">{link.status}</span>
                </div>
              </div>

              {(link.bandwidth || link.latency || link.utilization) && (
                <div className="mt-2 space-y-1">
                  <h5 className="font-medium text-xs text-gray-300">Performance</h5>
                  {link.bandwidth && (
                    <div className="flex justify-between">
                      <span className="text-gray-400">Bandwidth:</span>
                      <span className="text-cyan-400">{link.bandwidth} Mbps</span>
                    </div>
                  )}
                  {link.latency && (
                    <div className="flex justify-between">
                      <span className="text-gray-400">Latency:</span>
                      <span className="text-cyan-400">{link.latency}ms</span>
                    </div>
                  )}
                  {link.utilization && (
                    <div className="flex justify-between">
                      <span className="text-gray-400">Utilization:</span>
                      <span className="text-cyan-400">{link.utilization}%</span>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        );
      })()}
    </div>
  );
}
