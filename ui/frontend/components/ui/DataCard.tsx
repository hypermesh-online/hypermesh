// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';

interface DataCardProps {
  title: string;
  description?: string;
  icon?: LucideIcon;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  actions?: Array<{
    label: string;
    onClick: () => void;
    variant?: 'default' | 'outline' | 'destructive';
    icon?: LucideIcon;
  }>;
  badge?: {
    text: string;
    variant?: 'default' | 'secondary' | 'destructive' | 'outline';
  };
  className?: string;
  children: React.ReactNode;
}

export function DataCard({
  title,
  description,
  icon: Icon,
  theme = 'cyan',
  actions,
  badge,
  className,
  children
}: DataCardProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        border: 'border-cyan-500/30',
        bg: 'bg-black/40',
        icon: 'text-cyan-400'
      },
      green: {
        border: 'border-green-500/30',
        bg: 'bg-black/40',
        icon: 'text-green-400'
      },
      purple: {
        border: 'border-purple-500/30',
        bg: 'bg-black/40',
        icon: 'text-purple-400'
      },
      red: {
        border: 'border-red-500/30',
        bg: 'bg-black/40',
        icon: 'text-red-400'
      },
      yellow: {
        border: 'border-yellow-500/30',
        bg: 'bg-black/40',
        icon: 'text-yellow-400'
      }
    };
    return themes[theme];
  };

  const colors = getThemeColors();

  return (
    <Card className={cn(
      'backdrop-blur-lg transition-all duration-300',
      colors.border,
      colors.bg,
      className
    )}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            {Icon && <Icon className={cn('h-5 w-5', colors.icon)} />}
            <div>
              <CardTitle className="text-white flex items-center gap-2">
                {title}
                {badge && (
                  <Badge variant={badge.variant} className="ml-auto">
                    {badge.text}
                  </Badge>
                )}
              </CardTitle>
              {description && (
                <p className="text-gray-400 text-sm mt-1">{description}</p>
              )}
            </div>
          </div>
          {actions && (
            <div className="flex gap-2">
              {actions.map((action, index) => {
                const ActionIcon = action.icon;
                return (
                  <Button
                    key={index}
                    variant={action.variant || 'outline'}
                    size="sm"
                    onClick={action.onClick}
                    className={cn(
                      action.variant === 'outline' && `border-${theme}-500/30 text-${theme}-400 hover:bg-${theme}-500/20`
                    )}
                  >
                    {ActionIcon && <ActionIcon className="h-4 w-4 mr-2" />}
                    {action.label}
                  </Button>
                );
              })}
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {children}
      </CardContent>
    </Card>
  );
}
