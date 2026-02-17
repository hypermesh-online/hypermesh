// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ArrowRight, Info, MapPin, Route } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Link } from 'react-router-dom';

interface NavigationHint {
  title: string;
  description: string;
  href: string;
  badge?: string;
  action?: string;
  priority?: 'high' | 'medium' | 'low';
}

interface NavigationHintsProps {
  hints: NavigationHint[];
  title?: string;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  compact?: boolean;
  className?: string;
}

export function NavigationHints({
  hints,
  title = 'Next Steps',
  theme = 'cyan',
  compact = false,
  className
}: NavigationHintsProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        border: 'border-cyan-500/30',
        bg: 'bg-cyan-500/5',
        accent: 'bg-cyan-500/10 border-cyan-500/20',
        text: 'text-cyan-400',
        button: 'bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black'
      },
      green: {
        border: 'border-green-500/30',
        bg: 'bg-green-500/5',
        accent: 'bg-green-500/10 border-green-500/20',
        text: 'text-green-400',
        button: 'bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-400 hover:to-emerald-500 text-black'
      },
      purple: {
        border: 'border-purple-500/30',
        bg: 'bg-purple-500/5',
        accent: 'bg-purple-500/10 border-purple-500/20',
        text: 'text-purple-400',
        button: 'bg-gradient-to-r from-purple-500 to-indigo-600 hover:from-purple-400 hover:to-indigo-500 text-black'
      },
      red: {
        border: 'border-red-500/30',
        bg: 'bg-red-500/5',
        accent: 'bg-red-500/10 border-red-500/20',
        text: 'text-red-400',
        button: 'bg-gradient-to-r from-red-500 to-pink-600 hover:from-red-400 hover:to-pink-500 text-black'
      },
      yellow: {
        border: 'border-yellow-500/30',
        bg: 'bg-yellow-500/5',
        accent: 'bg-yellow-500/10 border-yellow-500/20',
        text: 'text-yellow-400',
        button: 'bg-gradient-to-r from-yellow-500 to-orange-600 hover:from-yellow-400 hover:to-orange-500 text-black'
      }
    };
    return themes[theme];
  };

  const getPriorityColor = (priority?: string) => {
    switch (priority) {
      case 'high': return 'bg-red-500/20 text-red-400 border-red-500/30';
      case 'medium': return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      case 'low': return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
      default: return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
    }
  };

  const colors = getThemeColors();

  if (hints.length === 0) {
    return null;
  }

  return (
    <Card className={cn(
      'backdrop-blur-lg',
      colors.border,
      colors.bg,
      className
    )}>
      <CardContent className={cn('p-4', compact && 'p-3')}>
        <div className="flex items-center gap-2 mb-3">
          <Route className={cn('h-4 w-4', colors.text)} />
          <h3 className={cn('font-medium text-white', compact && 'text-sm')}>
            {title}
          </h3>
        </div>
        
        <div className={cn('space-y-3', compact && 'space-y-2')}>
          {hints.map((hint, index) => (
            <div key={index} className={cn(
              'flex items-center justify-between p-3 rounded-lg border',
              colors.accent,
              compact && 'p-2'
            )}>
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-1">
                  <h4 className={cn(
                    'font-medium text-white',
                    compact && 'text-sm'
                  )}>
                    {hint.title}
                  </h4>
                  {hint.badge && (
                    <Badge variant="outline" className={getPriorityColor(hint.priority)}>
                      {hint.badge}
                    </Badge>
                  )}
                </div>
                <p className={cn(
                  'text-gray-400',
                  compact ? 'text-xs' : 'text-sm'
                )}>
                  {hint.description}
                </p>
              </div>
              
              <Link to={hint.href}>
                <Button
                  size={compact ? 'sm' : 'default'}
                  className={colors.button}
                >
                  {hint.action || 'Continue'}
                  <ArrowRight className="h-4 w-4 ml-2" />
                </Button>
              </Link>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
