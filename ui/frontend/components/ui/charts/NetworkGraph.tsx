// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useRef, useState } from 'react';
import { cn } from '@/lib/utils';

interface Node {
  id: string;
  label: string;
  type?: 'primary' | 'secondary' | 'endpoint';
  status?: 'active' | 'inactive' | 'warning' | 'error';
  metrics?: Record<string, number>;
}

interface Edge {
  source: string;
  target: string;
  weight?: number;
  type?: 'strong' | 'weak' | 'data' | 'control';
  status?: 'active' | 'inactive' | 'congested';
}

interface NetworkGraphProps {
  nodes: Node[];
  edges: Edge[];
  width?: number;
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  interactive?: boolean;
  showLabels?: boolean;
  showMetrics?: boolean;
  className?: string;
}

export function NetworkGraph({
  nodes,
  edges,
  width = 800,
  height = 600,
  theme = 'cyan',
  interactive = true,
  showLabels = true,
  showMetrics = false,
  className
}: NetworkGraphProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [nodePositions, setNodePositions] = useState<Record<string, { x: number; y: number }>>({});

  const getThemeColors = () => {
    const themes = {
      cyan: {
        primary: '#22d3ee',
        secondary: '#06b6d4',
        endpoint: '#0891b2',
        edge: 'rgba(34, 211, 238, 0.4)',
        edgeActive: '#22d3ee',
        background: 'rgba(34, 211, 238, 0.05)'
      },
      green: {
        primary: '#4ade80',
        secondary: '#22c55e',
        endpoint: '#16a34a',
        edge: 'rgba(74, 222, 128, 0.4)',
        edgeActive: '#4ade80',
        background: 'rgba(74, 222, 128, 0.05)'
      },
      purple: {
        primary: '#a855f7',
        secondary: '#9333ea',
        endpoint: '#7c3aed',
        edge: 'rgba(168, 85, 247, 0.4)',
        edgeActive: '#a855f7',
        background: 'rgba(168, 85, 247, 0.05)'
      },
      red: {
        primary: '#f87171',
        secondary: '#ef4444',
        endpoint: '#dc2626',
        edge: 'rgba(248, 113, 113, 0.4)',
        edgeActive: '#f87171',
        background: 'rgba(248, 113, 113, 0.05)'
      },
      yellow: {
        primary: '#fbbf24',
        secondary: '#f59e0b',
        endpoint: '#d97706',
        edge: 'rgba(251, 191, 36, 0.4)',
        edgeActive: '#fbbf24',
        background: 'rgba(251, 191, 36, 0.05)'
      }
    };
    return themes[theme];
  };

  const getStatusColor = (status?: string) => {
    const statusColors = {
      active: '#4ade80',
      inactive: '#6b7280',
      warning: '#fbbf24',
      error: '#f87171'
    };
    return statusColors[status as keyof typeof statusColors] || statusColors.active;
  };

  const getNodeColor = (node: Node) => {
    const colors = getThemeColors();
    if (node.status) {
      return getStatusColor(node.status);
    }
    switch (node.type) {
      case 'primary': return colors.primary;
      case 'secondary': return colors.secondary;
      case 'endpoint': return colors.endpoint;
      default: return colors.secondary;
    }
  };

  const getEdgeColor = (edge: Edge) => {
    const colors = getThemeColors();
    if (edge.status === 'congested') return '#f87171';
    if (edge.status === 'inactive') return '#6b7280';
    return colors.edge;
  };

  const getNodeRadius = (node: Node) => {
    switch (node.type) {
      case 'primary': return 12;
      case 'secondary': return 8;
      case 'endpoint': return 6;
      default: return 8;
    }
  };

  // Simple force-directed layout algorithm
  useEffect(() => {
    if (nodes.length === 0) return;

    const positions = { ...nodePositions };
    
    // Initialize positions if not set
    nodes.forEach((node, index) => {
      if (!positions[node.id]) {
        const angle = (index / nodes.length) * 2 * Math.PI;
        const radius = Math.min(width, height) * 0.3;
        positions[node.id] = {
          x: width / 2 + Math.cos(angle) * radius,
          y: height / 2 + Math.sin(angle) * radius
        };
      }
    });

    // Simple spring physics simulation
    const iterations = 50;
    const springStrength = 0.1;
    const repelStrength = 1000;
    const damping = 0.9;

    for (let iter = 0; iter < iterations; iter++) {
      const forces: Record<string, { x: number; y: number }> = {};
      
      // Initialize forces
      nodes.forEach(node => {
        forces[node.id] = { x: 0, y: 0 };
      });

      // Repel forces between all nodes
      nodes.forEach(nodeA => {
        nodes.forEach(nodeB => {
          if (nodeA.id === nodeB.id) return;
          
          const posA = positions[nodeA.id];
          const posB = positions[nodeB.id];
          const dx = posA.x - posB.x;
          const dy = posA.y - posB.y;
          const distance = Math.sqrt(dx * dx + dy * dy) || 1;
          
          const force = repelStrength / (distance * distance);
          forces[nodeA.id].x += (dx / distance) * force;
          forces[nodeA.id].y += (dy / distance) * force;
        });
      });

      // Attract forces for connected nodes
      edges.forEach(edge => {
        const posSource = positions[edge.source];
        const posTarget = positions[edge.target];
        
        if (!posSource || !posTarget) return;
        
        const dx = posTarget.x - posSource.x;
        const dy = posTarget.y - posSource.y;
        const distance = Math.sqrt(dx * dx + dy * dy) || 1;
        const targetDistance = 100;
        
        const force = springStrength * (distance - targetDistance);
        forces[edge.source].x += (dx / distance) * force;
        forces[edge.source].y += (dy / distance) * force;
        forces[edge.target].x -= (dx / distance) * force;
        forces[edge.target].y -= (dy / distance) * force;
      });

      // Apply forces
      nodes.forEach(node => {
        const pos = positions[node.id];
        const force = forces[node.id];
        
        pos.x += force.x * damping;
        pos.y += force.y * damping;
        
        // Keep nodes within bounds
        pos.x = Math.max(50, Math.min(width - 50, pos.x));
        pos.y = Math.max(50, Math.min(height - 50, pos.y));
      });
    }

    setNodePositions(positions);
  }, [nodes, edges, width, height]);

  const colors = getThemeColors();

  return (
    <div className={cn('relative', className)}>
      <svg
        ref={svgRef}
        width={width}
        height={height}
        className="border border-gray-700 rounded-lg"
        style={{ backgroundColor: colors.background }}
      >
        {/* Edges */}
        {edges.map((edge, index) => {
          const sourcePos = nodePositions[edge.source];
          const targetPos = nodePositions[edge.target];
          
          if (!sourcePos || !targetPos) return null;

          const strokeWidth = edge.weight ? Math.max(1, edge.weight * 3) : 2;
          
          return (
            <line
              key={index}
              x1={sourcePos.x}
              y1={sourcePos.y}
              x2={targetPos.x}
              y2={targetPos.y}
              stroke={getEdgeColor(edge)}
              strokeWidth={strokeWidth}
              className="transition-all duration-300"
            />
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
              <circle
                cx={pos.x}
                cy={pos.y}
                r={radius}
                fill={getNodeColor(node)}
                stroke={isSelected ? '#ffffff' : 'transparent'}
                strokeWidth={isSelected ? 2 : 0}
                className={cn(
                  'transition-all duration-300',
                  interactive && 'cursor-pointer hover:opacity-80'
                )}
                onClick={() => interactive && setSelectedNode(isSelected ? null : node.id)}
              />
              
              {showLabels && (
                <text
                  x={pos.x}
                  y={pos.y + radius + 15}
                  textAnchor="middle"
                  className="text-xs fill-white"
                >
                  {node.label}
                </text>
              )}

              {showMetrics && node.metrics && isSelected && (
                <foreignObject
                  x={pos.x + radius + 10}
                  y={pos.y - 20}
                  width={120}
                  height={60}
                >
                  <div className="bg-black/90 border border-gray-600 rounded p-2 text-xs text-white">
                    {Object.entries(node.metrics).map(([key, value]) => (
                      <div key={key} className="flex justify-between">
                        <span>{key}:</span>
                        <span className="text-cyan-400">{value}</span>
                      </div>
                    ))}
                  </div>
                </foreignObject>
              )}
            </g>
          );
        })}
      </svg>

      {selectedNode && (
        <div className="absolute top-4 right-4 bg-black/90 border border-gray-600 rounded p-3 text-sm text-white max-w-xs">
          {(() => {
            const node = nodes.find(n => n.id === selectedNode);
            if (!node) return null;
            
            return (
              <div>
                <h4 className="font-medium mb-2">{node.label}</h4>
                <p className="text-gray-400 text-xs mb-2">Type: {node.type || 'node'}</p>
                {node.status && (
                  <p className="text-gray-400 text-xs mb-2">Status: {node.status}</p>
                )}
                {node.metrics && (
                  <div className="space-y-1">
                    {Object.entries(node.metrics).map(([key, value]) => (
                      <div key={key} className="flex justify-between text-xs">
                        <span className="text-gray-400">{key}:</span>
                        <span className="text-cyan-400">{value}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })()}
        </div>
      )}
    </div>
  );
}
