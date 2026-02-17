// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CheckCircle, Circle, ArrowRight, Lock } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Link } from 'react-router-dom';

interface FlowStep {
  id: string;
  title: string;
  description: string;
  href: string;
  status: 'completed' | 'current' | 'upcoming' | 'locked';
  requirement?: string;
}

interface FlowIndicatorProps {
  steps: FlowStep[];
  title?: string;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  orientation?: 'horizontal' | 'vertical';
  compact?: boolean;
  className?: string;
}

export function FlowIndicator({
  steps,
  title = 'User Journey',
  theme = 'cyan',
  orientation = 'horizontal',
  compact = false,
  className
}: FlowIndicatorProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        border: 'border-cyan-500/30',
        bg: 'bg-cyan-500/5',
        completed: 'text-cyan-400 border-cyan-400',
        current: 'text-cyan-400 border-cyan-400 bg-cyan-500/10',
        upcoming: 'text-gray-400 border-gray-600',
        locked: 'text-gray-600 border-gray-700',
        connector: 'bg-cyan-400'
      },
      green: {
        border: 'border-green-500/30',
        bg: 'bg-green-500/5',
        completed: 'text-green-400 border-green-400',
        current: 'text-green-400 border-green-400 bg-green-500/10',
        upcoming: 'text-gray-400 border-gray-600',
        locked: 'text-gray-600 border-gray-700',
        connector: 'bg-green-400'
      },
      purple: {
        border: 'border-purple-500/30',
        bg: 'bg-purple-500/5',
        completed: 'text-purple-400 border-purple-400',
        current: 'text-purple-400 border-purple-400 bg-purple-500/10',
        upcoming: 'text-gray-400 border-gray-600',
        locked: 'text-gray-600 border-gray-700',
        connector: 'bg-purple-400'
      },
      red: {
        border: 'border-red-500/30',
        bg: 'bg-red-500/5',
        completed: 'text-red-400 border-red-400',
        current: 'text-red-400 border-red-400 bg-red-500/10',
        upcoming: 'text-gray-400 border-gray-600',
        locked: 'text-gray-600 border-gray-700',
        connector: 'bg-red-400'
      },
      yellow: {
        border: 'border-yellow-500/30',
        bg: 'bg-yellow-500/5',
        completed: 'text-yellow-400 border-yellow-400',
        current: 'text-yellow-400 border-yellow-400 bg-yellow-500/10',
        upcoming: 'text-gray-400 border-gray-600',
        locked: 'text-gray-600 border-gray-700',
        connector: 'bg-yellow-400'
      }
    };
    return themes[theme];
  };

  const getStepIcon = (status: FlowStep['status']) => {
    switch (status) {
      case 'completed':
        return CheckCircle;
      case 'current':
        return Circle;
      case 'upcoming':
        return Circle;
      case 'locked':
        return Lock;
      default:
        return Circle;
    }
  };

  const getStepClasses = (status: FlowStep['status']) => {
    const colors = getThemeColors();
    switch (status) {
      case 'completed':
        return colors.completed;
      case 'current':
        return colors.current;
      case 'upcoming':
        return colors.upcoming;
      case 'locked':
        return colors.locked;
      default:
        return colors.upcoming;
    }
  };

  const colors = getThemeColors();

  return (
    <Card className={cn(
      'backdrop-blur-lg',
      colors.border,
      colors.bg,
      className
    )}>
      <CardContent className={cn('p-4', compact && 'p-3')}>
        <h3 className={cn(
          'font-medium text-white mb-4',
          compact && 'text-sm mb-3'
        )}>
          {title}
        </h3>

        <div className={cn(
          'flex',
          orientation === 'vertical' ? 'flex-col space-y-4' : 'items-center space-x-2',
          compact && (orientation === 'vertical' ? 'space-y-2' : 'space-x-1')
        )}>
          {steps.map((step, index) => {
            const Icon = getStepIcon(step.status);
            const isClickable = step.status === 'completed' || step.status === 'current';
            const showConnector = index < steps.length - 1;

            const StepContent = () => (
              <div className={cn(
                'flex items-center',
                orientation === 'vertical' ? 'space-x-3' : 'flex-col space-y-2'
              )}>
                <div className={cn(
                  'flex items-center justify-center w-8 h-8 rounded-full border-2 transition-all duration-200',
                  getStepClasses(step.status),
                  compact && 'w-6 h-6'
                )}>
                  <Icon className={cn(
                    'h-4 w-4',
                    compact && 'h-3 w-3'
                  )} />
                </div>

                {orientation === 'vertical' && (
                  <div className="flex-1">
                    <h4 className={cn(
                      'font-medium text-white',
                      compact && 'text-sm'
                    )}>
                      {step.title}
                    </h4>
                    <p className={cn(
                      'text-gray-400',
                      compact ? 'text-xs' : 'text-sm'
                    )}>
                      {step.description}
                    </p>
                    {step.requirement && step.status === 'locked' && (
                      <Badge variant="outline" className="mt-1 text-xs bg-gray-500/20 text-gray-400 border-gray-500/30">
                        Requires: {step.requirement}
                      </Badge>
                    )}
                  </div>
                )}
              </div>
            );

            return (
              <React.Fragment key={step.id}>
                {isClickable ? (
                  <Link
                    to={step.href}
                    className={cn(
                      'transition-transform duration-200 hover:scale-105',
                      orientation === 'horizontal' && 'text-center'
                    )}
                  >
                    <StepContent />
                    {orientation === 'horizontal' && (
                      <div className="mt-2">
                        <p className={cn(
                          'font-medium text-white',
                          compact ? 'text-xs' : 'text-sm'
                        )}>
                          {step.title}
                        </p>
                        <p className={cn(
                          'text-gray-400',
                          compact ? 'text-xs' : 'text-sm'
                        )}>
                          {step.description}
                        </p>
                      </div>
                    )}
                  </Link>
                ) : (
                  <div className={cn(
                    orientation === 'horizontal' && 'text-center'
                  )}>
                    <StepContent />
                    {orientation === 'horizontal' && (
                      <div className="mt-2">
                        <p className={cn(
                          'font-medium text-white',
                          compact ? 'text-xs' : 'text-sm'
                        )}>
                          {step.title}
                        </p>
                        <p className={cn(
                          'text-gray-400',
                          compact ? 'text-xs' : 'text-sm'
                        )}>
                          {step.description}
                        </p>
                        {step.requirement && step.status === 'locked' && (
                          <Badge variant="outline" className="mt-1 text-xs bg-gray-500/20 text-gray-400 border-gray-500/30">
                            Requires: {step.requirement}
                          </Badge>
                        )}
                      </div>
                    )}
                  </div>
                )}

                {showConnector && (
                  <div className={cn(
                    orientation === 'vertical' 
                      ? 'ml-4 w-0.5 h-4'
                      : 'flex-1 h-0.5 min-w-8',
                    step.status === 'completed' ? colors.connector : 'bg-gray-600'
                  )} />
                )}
              </React.Fragment>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
