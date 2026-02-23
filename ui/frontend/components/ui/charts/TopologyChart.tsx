// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useRef, useEffect, useState } from 'react';
import { cn } from '@/lib/utils';
import {
  getThemeColors,
  getNodeColor,
  getNodeRadius,
  getLinkColor,
  getLinkWidth,
  calculatePositions,
  InfoPanel
} from './topology-chart';
import type { TopologyChartProps } from './topology-chart';

// Re-export types for consumers
export type { TopologyNode, TopologyLink } from './topology-chart';

export function TopologyChart({
  nodes,
  links,
  width = 800,
  height = 600,
  theme = 'cyan',
  layout = 'force',
  interactive = true,
  showMetrics = true,
  showLabels = true,
  onNodeClick,
  onLinkClick,
  className
}: TopologyChartProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [selectedLink, setSelectedLink] = useState<string | null>(null);
  const [nodePositions, setNodePositions] = useState<Record<string, { x: number; y: number }>>({});
  const [draggedNode, setDraggedNode] = useState<string | null>(null);

  const colors = getThemeColors(theme);

  useEffect(() => {
    if (nodes.length === 0) return;

    const newPositions = calculatePositions({
      nodes,
      links,
      width,
      height,
      layout,
      existingPositions: nodePositions,
      draggedNode
    });
    setNodePositions(newPositions);
  }, [nodes, links, layout, width, height]);

  const handleNodeClick = (node: typeof nodes[0]) => {
    if (!interactive) return;
    setSelectedNode(selectedNode === node.id ? null : node.id);
    setSelectedLink(null);
    onNodeClick?.(node);
  };

  const handleLinkClick = (link: typeof links[0], event: React.MouseEvent) => {
    if (!interactive) return;
    event.stopPropagation();
    const linkId = `${link.source}-${link.target}`;
    setSelectedLink(selectedLink === linkId ? null : linkId);
    setSelectedNode(null);
    onLinkClick?.(link);
  };

  return (
    <div className={cn('relative select-none', className)}>
      <svg
        ref={svgRef}
        width={width}
        height={height}
        className="border border-gray-700 rounded-lg cursor-grab"
        style={{ backgroundColor: colors.background }}
        onClick={() => { setSelectedNode(null); setSelectedLink(null); }}
      >
        {/* Grid */}
        <defs>
          <pattern id={`grid-${theme}`} width="20" height="20" patternUnits="userSpaceOnUse">
            <path d="M 20 0 L 0 0 0 20" fill="none" stroke={colors.grid} strokeWidth="1"/>
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill={`url(#grid-${theme})`} />

        {/* Links */}
        {links.map((link, index) => {
          const sourcePos = nodePositions[link.source];
          const targetPos = nodePositions[link.target];
          if (!sourcePos || !targetPos) return null;

          const linkId = `${link.source}-${link.target}`;
          const isSelected = selectedLink === linkId;
          const strokeWidth = getLinkWidth(link);

          return (
            <g key={index}>
              <line
                x1={sourcePos.x} y1={sourcePos.y}
                x2={targetPos.x} y2={targetPos.y}
                stroke={getLinkColor(link, colors)}
                strokeWidth={strokeWidth}
                strokeDasharray={link.type === 'logical' ? '5,5' : undefined}
                className={cn(
                  'transition-all duration-300 cursor-pointer',
                  isSelected && 'drop-shadow-lg',
                  interactive && 'hover:stroke-white hover:opacity-80'
                )}
                onClick={(e) => handleLinkClick(link, e)}
              />
              {showMetrics && (link.latency || link.utilization) && (
                <text
                  x={(sourcePos.x + targetPos.x) / 2}
                  y={(sourcePos.y + targetPos.y) / 2 - 10}
                  textAnchor="middle"
                  className="text-xs fill-gray-300 pointer-events-none"
                >
                  {link.latency && `${link.latency}ms`}
                  {link.latency && link.utilization && ' | '}
                  {link.utilization && `${link.utilization}%`}
                </text>
              )}
            </g>
          );
        })}

        {/* Nodes */}
        {nodes.map((node) => {
          const pos = nodePositions[node.id];
          if (!pos) return null;
          const radius = getNodeRadius(node);
          const isSelected = selectedNode === node.id;

          return (
            <g key={node.id}>
              <circle cx={pos.x + 2} cy={pos.y + 2} r={radius}
                fill="rgba(0, 0, 0, 0.3)" className="pointer-events-none" />
              <circle
                cx={pos.x} cy={pos.y} r={radius}
                fill={getNodeColor(node, colors)}
                stroke={isSelected ? '#ffffff' : 'rgba(255, 255, 255, 0.2)'}
                strokeWidth={isSelected ? 3 : 1}
                className={cn(
                  'transition-all duration-300',
                  interactive && 'cursor-pointer hover:scale-110 hover:brightness-110'
                )}
                onClick={() => handleNodeClick(node)}
              />
              {node.status !== 'active' && (
                <circle
                  cx={pos.x + radius - 4} cy={pos.y - radius + 4} r={3}
                  fill={
                    node.status === 'error' ? '#ef4444' :
                    node.status === 'warning' ? '#fbbf24' :
                    node.status === 'maintenance' ? '#6b7280' : '#4b5563'
                  }
                  stroke="#ffffff" strokeWidth={1} className="pointer-events-none"
                />
              )}
              {showMetrics && node.metrics?.load && (
                <circle
                  cx={pos.x} cy={pos.y} r={radius - 2}
                  fill="none" stroke="#ffffff" strokeWidth={2}
                  strokeDasharray={`${node.metrics.load * 2.5} ${250 - node.metrics.load * 2.5}`}
                  transform={`rotate(-90 ${pos.x} ${pos.y})`}
                  className="opacity-60 pointer-events-none"
                />
              )}
              {showLabels && (
                <text
                  x={pos.x} y={pos.y + radius + 15}
                  textAnchor="middle"
                  className="text-xs fill-white font-medium pointer-events-none"
                >
                  {node.label}
                </text>
              )}
            </g>
          );
        })}
      </svg>

      <InfoPanel
        selectedNode={selectedNode}
        selectedLink={selectedLink}
        nodes={nodes}
        links={links}
      />

      {/* Legend */}
      <div className="absolute bottom-4 left-4 bg-black/80 border border-gray-600 rounded-lg p-3 text-xs">
        <h5 className="font-medium text-white mb-2">Node Types</h5>
        <div className="grid grid-cols-2 gap-2">
          {['core', 'gateway', 'edge', 'service', 'endpoint'].map(type => {
            const color = colors[type as keyof typeof colors] as string;
            return (
              <div key={type} className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full" style={{ backgroundColor: color }} />
                <span className="text-gray-300 capitalize">{type}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
