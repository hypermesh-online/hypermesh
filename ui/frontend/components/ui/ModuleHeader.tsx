// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';

interface ModuleHeaderProps {
  title: string;
  description?: string;
  icon?: LucideIcon;
  gradient?: string;
  actions?: Array<{
    label: string;
    onClick: () => void;
    variant?: 'default' | 'outline' | 'destructive';
    icon?: LucideIcon;
  }>;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  centered?: boolean;
  className?: string;
}

export function ModuleHeader({
  title,
  description,
  icon: Icon,
  gradient,
  actions,
  theme = 'cyan',
  centered = false,
  className
}: ModuleHeaderProps) {
  const getGradient = () => {
    if (gradient) return gradient;
    
    const gradients = {
      cyan: 'from-cyan-400 to-blue-600',
      green: 'from-green-400 to-emerald-600',
      purple: 'from-purple-400 to-indigo-600',
      red: 'from-red-400 to-pink-600',
      yellow: 'from-yellow-400 to-orange-600'
    };
    return gradients[theme];
  };

  const HeaderContent = () => (
    <>
      <div className={cn('flex items-center gap-3', centered && 'justify-center')}>
        {Icon && (
          <div className={cn(
            'p-2 rounded-lg bg-gradient-to-r',
            getGradient()
          )}>
            <Icon className="h-8 w-8 text-black" />
          </div>
        )}
        <div className={cn(centered && 'text-center')}>
          <h1 className={cn(
            'text-3xl font-bold bg-gradient-to-r bg-clip-text text-transparent',
            getGradient()
          )}>
            {title}
          </h1>
          {description && (
            <p className="text-gray-400 mt-2 max-w-2xl">
              {description}
            </p>
          )}
        </div>
      </div>
      {actions && !centered && (
        <div className="flex gap-2">
          {actions.map((action, index) => {
            const ActionIcon = action.icon;
            return (
              <Button
                key={index}
                variant={action.variant || 'default'}
                onClick={action.onClick}
                className={cn(
                  action.variant === 'default' && `bg-gradient-to-r ${getGradient()} hover:opacity-90 text-black font-medium`
                )}
              >
                {ActionIcon && <ActionIcon className="h-4 w-4 mr-2" />}
                {action.label}
              </Button>
            );
          })}
        </div>
      )}
    </>
  );

  if (centered) {
    return (
      <div className={cn('text-center py-6', className)}>
        <HeaderContent />
        {actions && (
          <div className="flex justify-center gap-2 mt-4">
            {actions.map((action, index) => {
              const ActionIcon = action.icon;
              return (
                <Button
                  key={index}
                  variant={action.variant || 'default'}
                  onClick={action.onClick}
                  className={cn(
                    action.variant === 'default' && `bg-gradient-to-r ${getGradient()} hover:opacity-90 text-black font-medium`
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
    );
  }

  return (
    <div className={cn('flex justify-between items-center', className)}>
      <HeaderContent />
    </div>
  );
}
