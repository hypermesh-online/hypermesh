// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { CheckCircle, Circle, Lock } from 'lucide-react';
import type { JourneyStep } from './types';

export function getStepIcon(step: JourneyStep): React.ReactElement {
  const Icon = step.icon;
  switch (step.status) {
    case 'completed':
      return <CheckCircle className="h-6 w-6 text-green-400" />;
    case 'current':
      return <Icon className="h-6 w-6 text-cyan-400" />;
    case 'available':
      return <Circle className="h-6 w-6 text-blue-400" />;
    case 'locked':
      return <Lock className="h-6 w-6 text-gray-600" />;
  }
}

export function getDifficultyColor(difficulty?: string): string {
  switch (difficulty) {
    case 'beginner': return 'text-green-400';
    case 'intermediate': return 'text-yellow-400';
    case 'advanced': return 'text-red-400';
    default: return 'text-gray-400';
  }
}
