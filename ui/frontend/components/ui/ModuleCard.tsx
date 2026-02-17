// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';

interface ModuleCardProps {
  title: string;
  description?: string;
  value: string | number;
  subtitle?: string;
  icon?: LucideIcon;
  iconColor?: string;
  gradient?: string;
  status?: 'active' | 'inactive' | 'warning' | 'error';
  progress?: number;
  badge?: {
    text: string;
    variant?: 'default' | 'secondary' | 'destructive' | 'outline';
    className?: string;
  };
  className?: string;
  children?: React.ReactNode;
}

export function ModuleCard({
  title,
  description,
  value,
  subtitle,
  icon: Icon,
  iconColor = 'text-cyan-400',
  gradient,
  status,
  progress,
  badge,
  className,
  children
}: ModuleCardProps) {
  const getStatusColor = () => {
    switch (status) {
      case 'active': return 'border-green-500/30 bg-green-500/5';
      case 'warning': return 'border-yellow-500/30 bg-yellow-500/5';
      case 'error': return 'border-red-500/30 bg-red-500/5';
      default: return 'border-cyan-500/30 bg-black/40';
    }
  };

  return (
    <Card className={cn(
      'backdrop-blur-lg transition-all duration-300 hover:shadow-lg hover:shadow-cyan-500/10',
      getStatusColor(),
      className
    )}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-white">{title}</CardTitle>
        {Icon && (
          <div className={cn(
            "p-1 rounded",
            gradient && `bg-gradient-to-r ${gradient}`
          )}>
            <Icon className={cn("h-4 w-4", gradient ? "text-black" : iconColor)} />
          </div>
        )}
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <div className="text-2xl font-bold text-cyan-400">{value}</div>
          {subtitle && <p className="text-xs text-gray-400">{subtitle}</p>}
          {description && <CardDescription className="text-gray-400">{description}</CardDescription>}
          {progress !== undefined && (
            <Progress value={progress} className="h-2" />
          )}
          {badge && (
            <Badge variant={badge.variant} className={badge.className}>
              {badge.text}
            </Badge>
          )}
          {children}
        </div>
      </CardContent>
    </Card>
  );
}
