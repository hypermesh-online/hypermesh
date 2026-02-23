// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Shield, Zap, Network, Coins, Server, Gauge, Star, Trophy } from 'lucide-react';
import type { JourneyStep, Achievement } from './types';

export const defaultSteps: JourneyStep[] = [
  {
    id: 'identity-setup', title: 'Identity Foundation',
    description: 'Establish your TrustChain identity and cryptographic keys',
    icon: Shield, status: 'completed', progress: 100,
    requirements: [], rewards: ['Node certificate', 'Basic network access'],
    href: '/trustchain/identity', estimatedTime: '5 min', difficulty: 'beginner', category: 'identity'
  },
  {
    id: 'network-connection', title: 'Network Access',
    description: 'Connect to STOQ transport layer for high-speed communication',
    icon: Zap, status: 'current', progress: 75,
    requirements: ['TrustChain identity'], rewards: ['P2P tunneling', 'Enhanced bandwidth'],
    href: '/stoq/protocol', estimatedTime: '10 min', difficulty: 'beginner', category: 'network'
  },
  {
    id: 'resource-sharing', title: 'Resource Participation',
    description: 'Share computing resources and access HyperMesh network',
    icon: Network, status: 'available',
    requirements: ['STOQ connection', 'Resource commitment'], rewards: ['CAESAR tokens', 'Network voting rights'],
    href: '/hypermesh/resources', estimatedTime: '15 min', difficulty: 'intermediate', category: 'network'
  },
  {
    id: 'economic-participation', title: 'Economic Integration',
    description: 'Join the Caesar economy and start earning tokens',
    icon: Coins, status: 'available',
    requirements: ['HyperMesh access'], rewards: ['Token wallet', 'DEX trading'],
    href: '/caesar', estimatedTime: '10 min', difficulty: 'intermediate', category: 'economic'
  },
  {
    id: 'asset-creation', title: 'Asset Management',
    description: 'Create and manage digital assets in the Catalog',
    icon: Server, status: 'locked',
    requirements: ['Economic participation', 'Trust score >85%'], rewards: ['Asset registry access', 'Service deployment'],
    href: '/catalog/creation', estimatedTime: '20 min', difficulty: 'advanced', category: 'advanced'
  },
  {
    id: 'analytics-mastery', title: 'Analytics & Insights',
    description: 'Access Ngauge analytics and contribute to user onboarding',
    icon: Gauge, status: 'locked',
    requirements: ['Asset management', 'Community contribution'], rewards: ['Analytics dashboard', 'Ad network participation'],
    href: '/ngauge', estimatedTime: '15 min', difficulty: 'advanced', category: 'advanced'
  }
];

export const achievements: Achievement[] = [
  { id: 'first-steps', title: 'First Steps', description: 'Completed identity setup', icon: Star, earned: true, earnedDate: '2024-01-15', category: 'milestone' },
  { id: 'network-pioneer', title: 'Network Pioneer', description: 'Connected to 3 different network layers', icon: Network, earned: false, category: 'technical' },
  { id: 'token-holder', title: 'Token Holder', description: 'Earned your first 100 CAESAR tokens', icon: Coins, earned: false, category: 'economic' },
  { id: 'community-contributor', title: 'Community Contributor', description: 'Helped onboard 5 new users', icon: Trophy, earned: false, category: 'social' }
];
