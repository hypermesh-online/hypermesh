// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type React from 'react';

export interface JourneyStep {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  status: 'completed' | 'current' | 'available' | 'locked';
  progress?: number;
  requirements?: string[];
  rewards?: string[];
  href: string;
  estimatedTime?: string;
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
  category: 'identity' | 'network' | 'economic' | 'advanced';
}

export interface Achievement {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  earned: boolean;
  earnedDate?: string;
  category: 'milestone' | 'social' | 'technical' | 'economic';
}

export interface UserJourneyProps {
  className?: string;
  compact?: boolean;
  showAchievements?: boolean;
  onStepSelect?: (step: JourneyStep) => void;
}
