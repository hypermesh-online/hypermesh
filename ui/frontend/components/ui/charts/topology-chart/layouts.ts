// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { TopologyNode, TopologyLink } from './types';

type PositionMap = Record<string, { x: number; y: number }>;

interface LayoutParams {
  nodes: TopologyNode[];
  links: TopologyLink[];
  width: number;
  height: number;
  layout: string;
  existingPositions: PositionMap;
  draggedNode: string | null;
}

function circularLayout(params: LayoutParams, positions: PositionMap): void {
  const { nodes, width, height } = params;
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
}

function gridLayout(params: LayoutParams, positions: PositionMap): void {
  const { nodes, width, height } = params;
  const padding = 50;
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
}

function hierarchicalLayout(params: LayoutParams, positions: PositionMap): void {
  const { nodes, width } = params;
  const padding = 50;
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
}

function forceDirectedLayout(params: LayoutParams, positions: PositionMap): void {
  const { nodes, links, width, height, draggedNode } = params;
  const padding = 50;

  nodes.forEach(node => {
    if (!positions[node.id]) {
      positions[node.id] = {
        x: padding + Math.random() * (width - padding * 2),
        y: padding + Math.random() * (height - padding * 2)
      };
    }
  });

  const iterations = 100;
  const springLength = 100;
  const springStrength = 0.1;
  const repelStrength = 2000;
  const damping = 0.85;

  for (let iter = 0; iter < iterations; iter++) {
    const forces: Record<string, { x: number; y: number }> = {};
    nodes.forEach(node => { forces[node.id] = { x: 0, y: 0 }; });

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
      if (draggedNode === node.id) return;
      const pos = positions[node.id];
      const force = forces[node.id];
      pos.x += force.x * damping;
      pos.y += force.y * damping;
      pos.x = Math.max(padding, Math.min(width - padding, pos.x));
      pos.y = Math.max(padding, Math.min(height - padding, pos.y));
    });
  }
}

export function calculatePositions(params: LayoutParams): PositionMap {
  const positions = { ...params.existingPositions };

  switch (params.layout) {
    case 'circular':
      circularLayout(params, positions);
      break;
    case 'grid':
      gridLayout(params, positions);
      break;
    case 'hierarchical':
      hierarchicalLayout(params, positions);
      break;
    case 'force':
    default:
      forceDirectedLayout(params, positions);
      break;
  }

  return positions;
}
