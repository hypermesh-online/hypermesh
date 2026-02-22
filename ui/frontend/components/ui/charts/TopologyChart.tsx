// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useRef, useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

export interface TopologyNode {
  id: string;
  label: string;
  type: 'core' | 'edge' | 'endpoint' | 'gateway' | 'service';
  status: 'active' | 'inactive' | 'warning' | 'error' | 'maintenance';
  position?: { x: number; y: number };
  metrics?: {
    load?: number;
    connections?: number;
    latency?: number;
    throughput?: number;
  };
  groups?: string[];
}

export interface TopologyLink {
  source: string;
  target: string;
  type: 'physical' | 'logical' | 'data' | 'control' | 'tunnel';
  status: 'active' | 'inactive' | 'congested' | 'degraded';
  bandwidth?: number;
  latency?: number;
  utilization?: number;
  weight?: number;
}

interface TopologyChartProps {
  nodes: TopologyNode[];
  links: TopologyLink[];
  width?: number;
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  layout?: 'force' | 'hierarchical' | 'circular' | 'grid';
  interactive?: boolean;
  showMetrics?: boolean;
  showLabels?: boolean;
  onNodeClick?: (node: TopologyNode) => void;
  onLinkClick?: (link: TopologyLink) => void;
  className?: string;
}

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

  const getThemeColors = () => {
    const themes = {
      cyan: {
        core: '#22d3ee',
        edge: '#06b6d4',
        endpoint: '#0891b2',
        gateway: '#0e7490',
        service: '#155e75',
        linkActive: 'rgba(34, 211, 238, 0.8)',
        linkInactive: 'rgba(34, 211, 238, 0.3)',
        linkCongested: '#ef4444',
        background: 'rgba(34, 211, 238, 0.05)',
        grid: 'rgba(34, 211, 238, 0.1)'
      },
      green: {
        core: '#4ade80',
        edge: '#22c55e',
        endpoint: '#16a34a',
        gateway: '#15803d',
        service: '#166534',
        linkActive: 'rgba(74, 222, 128, 0.8)',
        linkInactive: 'rgba(74, 222, 128, 0.3)',
        linkCongested: '#ef4444',
        background: 'rgba(74, 222, 128, 0.05)',
        grid: 'rgba(74, 222, 128, 0.1)'
      },
      purple: {
        core: '#a855f7',
        edge: '#9333ea',
        endpoint: '#7c3aed',
        gateway: '#6d28d9',
        service: '#5b21b6',
        linkActive: 'rgba(168, 85, 247, 0.8)',
        linkInactive: 'rgba(168, 85, 247, 0.3)',
        linkCongested: '#ef4444',
        background: 'rgba(168, 85, 247, 0.05)',
        grid: 'rgba(168, 85, 247, 0.1)'
      },
      red: {
        core: '#f87171',
        edge: '#ef4444',
        endpoint: '#dc2626',
        gateway: '#b91c1c',
        service: '#991b1b',
        linkActive: 'rgba(248, 113, 113, 0.8)',
        linkInactive: 'rgba(248, 113, 113, 0.3)',
        linkCongested: '#fbbf24',
        background: 'rgba(248, 113, 113, 0.05)',
        grid: 'rgba(248, 113, 113, 0.1)'
      },
      yellow: {
        core: '#fbbf24',
        edge: '#f59e0b',
        endpoint: '#d97706',
        gateway: '#b45309',
        service: '#92400e',
        linkActive: 'rgba(251, 191, 36, 0.8)',
        linkInactive: 'rgba(251, 191, 36, 0.3)',
        linkCongested: '#ef4444',
        background: 'rgba(251, 191, 36, 0.05)',
        grid: 'rgba(251, 191, 36, 0.1)'
      }
    };
    return themes[theme];
  };

  const getNodeColor = (node: TopologyNode) => {
    const colors = getThemeColors();
    
    // Status-based colors override type colors
    switch (node.status) {
      case 'error': return '#ef4444';
      case 'warning': return '#fbbf24';
      case 'maintenance': return '#6b7280';
      case 'inactive': return '#4b5563';
    }

    // Type-based colors
    switch (node.type) {
      case 'core': return colors.core;
      case 'edge': return colors.edge;
      case 'endpoint': return colors.endpoint;
      case 'gateway': return colors.gateway;
      case 'service': return colors.service;
      default: return colors.edge;
    }
  };

  const getNodeRadius = (node: TopologyNode) => {
    const baseRadius = {
      core: 16,
      gateway: 12,
      edge: 10,
      service: 8,
      endpoint: 6
    };
    
    let radius = baseRadius[node.type] || 8;
    
    // Scale based on connections or load
    if (node.metrics?.connections) {
      radius += Math.min(node.metrics.connections / 5, 8);
    }
    
    return radius;
  };

  const getLinkColor = (link: TopologyLink) => {
    const colors = getThemeColors();
    
    switch (link.status) {
      case 'congested': return colors.linkCongested;
      case 'degraded': return '#fbbf24';
      case 'inactive': return colors.linkInactive;
      default: return colors.linkActive;
    }
  };

  const getLinkWidth = (link: TopologyLink) => {
    let width = 2;
    
    if (link.bandwidth) {
      width = Math.max(1, Math.min(link.bandwidth / 1000, 8));
    }
    
    if (link.utilization && link.utilization > 80) {
      width += 2;
    }
    
    return width;
  };

  // Layout algorithms
  const calculatePositions = () => {
    const positions = { ...nodePositions };
    const padding = 50;

    switch (layout) {
      case 'circular':
        nodes.forEach((node, index) => {
          if (!positions[node.id]) {
            const angle = (index / nodes.length) * 2 * Math.PI;
            const radius = Math.min(width, height) * 0.35;
            positions[node.id] = {
              x: width / 2 + Math.cos(angle) * radius,
              y: height / 2 + Math.sin(angle) * radius
            };
          }
        });
        break;

      case 'grid':
        const cols = Math.ceil(Math.sqrt(nodes.length));
        const cellWidth = (width - padding * 2) / cols;
        const cellHeight = (height - padding * 2) / Math.ceil(nodes.length / cols);
        
        nodes.forEach((node, index) => {
          if (!positions[node.id]) {
            const col = index % cols;
            const row = Math.floor(index / cols);
            positions[node.id] = {
              x: padding + col * cellWidth + cellWidth / 2,
              y: padding + row * cellHeight + cellHeight / 2
            };
          }
        });
        break;

      case 'hierarchical':
        // Group nodes by type
        const typeGroups: Record<string, TopologyNode[]> = {};
        nodes.forEach(node => {
          if (!typeGroups[node.type]) typeGroups[node.type] = [];
          typeGroups[node.type].push(node);
        });

        const typeOrder = ['core', 'gateway', 'edge', 'service', 'endpoint'];
        let currentY = padding;
        
        typeOrder.forEach(type => {
          if (typeGroups[type]) {
            const nodesInType = typeGroups[type];
            const typeWidth = (width - padding * 2) / nodesInType.length;
            
            nodesInType.forEach((node, index) => {
              if (!positions[node.id]) {
                positions[node.id] = {
                  x: padding + index * typeWidth + typeWidth / 2,
                  y: currentY + 50
                };
              }
            });
            currentY += 120;
          }
        });
        break;

      case 'force':
      default:
        // Initialize random positions if not set
        nodes.forEach(node => {
          if (!positions[node.id]) {
            positions[node.id] = {
              x: padding + Math.random() * (width - padding * 2),
              y: padding + Math.random() * (height - padding * 2)
            };
          }
        });

        // Simple force-directed layout
        const iterations = 100;
        const springLength = 100;
        const springStrength = 0.1;
        const repelStrength = 2000;
        const damping = 0.85;

        for (let iter = 0; iter < iterations; iter++) {
          const forces: Record<string, { x: number; y: number }> = {};
          
          nodes.forEach(node => {
            forces[node.id] = { x: 0, y: 0 };
          });

          // Repulsive forces
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

          // Attractive forces for connected nodes
          links.forEach(link => {
            const posSource = positions[link.source];
            const posTarget = positions[link.target];
            
            if (!posSource || !posTarget) return;
            
            const dx = posTarget.x - posSource.x;
            const dy = posTarget.y - posSource.y;
            const distance = Math.sqrt(dx * dx + dy * dy) || 1;
            
            const force = springStrength * (distance - springLength);
            forces[link.source].x += (dx / distance) * force;
            forces[link.source].y += (dy / distance) * force;
            forces[link.target].x -= (dx / distance) * force;
            forces[link.target].y -= (dy / distance) * force;
          });

          // Apply forces
          nodes.forEach(node => {
            if (draggedNode === node.id) return; // Don't move dragged node
            
            const pos = positions[node.id];
            const force = forces[node.id];
            
            pos.x += force.x * damping;
            pos.y += force.y * damping;
            
            // Keep within bounds
            pos.x = Math.max(padding, Math.min(width - padding, pos.x));
            pos.y = Math.max(padding, Math.min(height - padding, pos.y));
          });
        }
        break;
    }

    return positions;
  };

  useEffect(() => {
    if (nodes.length === 0) return;
    
    const newPositions = calculatePositions();
    setNodePositions(newPositions);
  }, [nodes, links, layout, width, height]);

  const handleNodeClick = (node: TopologyNode) => {
    if (!interactive) return;
    
    setSelectedNode(selectedNode === node.id ? null : node.id);
    setSelectedLink(null);
    onNodeClick?.(node);
  };

  const handleLinkClick = (link: TopologyLink, event: React.MouseEvent) => {
    if (!interactive) return;
    
    event.stopPropagation();
    const linkId = `${link.source}-${link.target}`;
    setSelectedLink(selectedLink === linkId ? null : linkId);
    setSelectedNode(null);
    onLinkClick?.(link);
  };

  const colors = getThemeColors();

  return (
    <div className={cn('relative select-none', className)}>
      <svg
        ref={svgRef}
        width={width}
        height={height}
        className="border border-gray-700 rounded-lg cursor-grab"
        style={{ backgroundColor: colors.background }}
        onClick={() => {
          setSelectedNode(null);
          setSelectedLink(null);
        }}
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
                x1={sourcePos.x}
                y1={sourcePos.y}
                x2={targetPos.x}
                y2={targetPos.y}
                stroke={getLinkColor(link)}
                strokeWidth={strokeWidth}
                strokeDasharray={link.type === 'logical' ? '5,5' : undefined}
                className={cn(
                  'transition-all duration-300 cursor-pointer',
                  isSelected && 'drop-shadow-lg',
                  interactive && 'hover:stroke-white hover:opacity-80'
                )}
                onClick={(e) => handleLinkClick(link, e)}
              />
              
              {/* Link metrics */}
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
              {/* Node shadow */}
              <circle
                cx={pos.x + 2}
                cy={pos.y + 2}
                r={radius}
                fill="rgba(0, 0, 0, 0.3)"
                className="pointer-events-none"
              />
              
              {/* Main node */}
              <circle
                cx={pos.x}
                cy={pos.y}
                r={radius}
                fill={getNodeColor(node)}
                stroke={isSelected ? '#ffffff' : 'rgba(255, 255, 255, 0.2)'}
                strokeWidth={isSelected ? 3 : 1}
                className={cn(
                  'transition-all duration-300',
                  interactive && 'cursor-pointer hover:scale-110 hover:brightness-110'
                )}
                onClick={() => handleNodeClick(node)}
              />

              {/* Status indicator */}
              {node.status !== 'active' && (
                <circle
                  cx={pos.x + radius - 4}
                  cy={pos.y - radius + 4}
                  r={3}
                  fill={
                    node.status === 'error' ? '#ef4444' :
                    node.status === 'warning' ? '#fbbf24' :
                    node.status === 'maintenance' ? '#6b7280' : '#4b5563'
                  }
                  stroke="#ffffff"
                  strokeWidth={1}
                  className="pointer-events-none"
                />
              )}

              {/* Load indicator */}
              {showMetrics && node.metrics?.load && (
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={radius - 2}
                  fill="none"
                  stroke="#ffffff"
                  strokeWidth={2}
                  strokeDasharray={`${node.metrics.load * 2.5} ${250 - node.metrics.load * 2.5}`}
                  transform={`rotate(-90 ${pos.x} ${pos.y})`}
                  className="opacity-60 pointer-events-none"
                />
              )}
              
              {/* Node label */}
              {showLabels && (
                <text
                  x={pos.x}
                  y={pos.y + radius + 15}
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

      {/* Info panel for selected node/link */}
      {(selectedNode || selectedLink) && (
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
                    <span className="ml-1 font-mono">{link.source} → {link.target}</span>
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
      )}

      {/* Legend */}
      <div className="absolute bottom-4 left-4 bg-black/80 border border-gray-600 rounded-lg p-3 text-xs">
        <h5 className="font-medium text-white mb-2">Node Types</h5>
        <div className="grid grid-cols-2 gap-2">
          {['core', 'gateway', 'edge', 'service', 'endpoint'].map(type => {
            const colors = getThemeColors();
            const color = colors[type as keyof typeof colors] as string;
            return (
              <div key={type} className="flex items-center gap-2">
                <div 
                  className="w-3 h-3 rounded-full" 
                  style={{ backgroundColor: color }}
                />
                <span className="text-gray-300 capitalize">{type}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
