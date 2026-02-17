// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { LucideIcon, Download, Maximize2, RefreshCw } from 'lucide-react';

interface ChartContainerProps {
  title: string;
  description?: string;
  icon?: LucideIcon;
  badge?: {
    text: string;
    variant?: 'default' | 'secondary' | 'destructive' | 'outline';
  };
  actions?: Array<{
    label: string;
    icon?: LucideIcon;
    onClick: () => void;
    variant?: 'default' | 'outline' | 'ghost';
  }>;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  loading?: boolean;
  error?: string;
  lastUpdated?: string;
  allowFullscreen?: boolean;
  allowExport?: boolean;
  allowRefresh?: boolean;
  onRefresh?: () => void;
  className?: string;
  children: React.ReactNode;
}

export function ChartContainer({
  title,
  description,
  icon: Icon,
  badge,
  actions = [],
  theme = 'cyan',
  loading = false,
  error,
  lastUpdated,
  allowFullscreen = false,
  allowExport = false,
  allowRefresh = false,
  onRefresh,
  className,
  children
}: ChartContainerProps) {
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

  const defaultActions = [];
  
  if (allowRefresh && onRefresh) {
    defaultActions.push({
      label: 'Refresh',
      icon: RefreshCw,
      onClick: onRefresh,
      variant: 'ghost' as const
    });
  }
  
  if (allowExport) {
    defaultActions.push({
      label: 'Export',
      icon: Download,
      onClick: () => console.log('Export chart'),
      variant: 'ghost' as const
    });
  }
  
  if (allowFullscreen) {
    defaultActions.push({
      label: 'Fullscreen',
      icon: Maximize2,
      onClick: () => console.log('Open fullscreen'),
      variant: 'ghost' as const
    });
  }

  const allActions = [...actions, ...defaultActions];

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
                  <Badge variant={badge.variant} className="ml-2">
                    {badge.text}
                  </Badge>
                )}
              </CardTitle>
              {description && (
                <CardDescription className="text-gray-400 mt-1">
                  {description}
                </CardDescription>
              )}
            </div>
          </div>
          
          {allActions.length > 0 && (
            <div className="flex gap-1">
              {allActions.map((action, index) => {
                const ActionIcon = action.icon;
                return (
                  <Button
                    key={index}
                    variant={action.variant || 'ghost'}
                    size="sm"
                    onClick={action.onClick}
                    className={cn(
                      'h-8 w-8 p-0',
                      action.variant === 'ghost' && 'hover:bg-gray-800'
                    )}
                    title={action.label}
                  >
                    {ActionIcon ? (
                      <ActionIcon className="h-4 w-4" />
                    ) : (
                      <span className="text-xs">{action.label}</span>
                    )}
                  </Button>
                );
              })}
            </div>
          )}
        </div>
        
        {lastUpdated && (
          <p className="text-xs text-gray-500 mt-2">
            Last updated: {lastUpdated}
          </p>
        )}
      </CardHeader>
      
      <CardContent>
        {loading && (
          <div className="flex items-center justify-center h-48">
            <div className="flex items-center gap-2 text-gray-400">
              <RefreshCw className="h-4 w-4 animate-spin" />
              <span className="text-sm">Loading chart data...</span>
            </div>
          </div>
        )}
        
        {error && (
          <div className="flex items-center justify-center h-48">
            <div className="text-center">
              <p className="text-red-400 text-sm mb-2">Error loading chart</p>
              <p className="text-gray-500 text-xs">{error}</p>
              {onRefresh && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onRefresh}
                  className="mt-3"
                >
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Retry
                </Button>
              )}
            </div>
          </div>
        )}
        
        {!loading && !error && children}
      </CardContent>
    </Card>
  );
}
