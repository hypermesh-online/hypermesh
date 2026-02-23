// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Trophy, CheckCircle } from 'lucide-react';
import { NavigationElement } from '../NavigationElement';
import type { Achievement } from './types';

interface AchievementsSectionProps {
  achievements: Achievement[];
}

export function AchievementsSection({ achievements }: AchievementsSectionProps) {
  return (
    <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Trophy className="h-6 w-6 text-yellow-400" />
          Achievements
          <Badge className="ml-auto bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
            {achievements.filter(a => a.earned).length}/{achievements.length}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid gap-3 md:grid-cols-2">
          {achievements.map((achievement, index) => (
            <NavigationElement
              key={achievement.id}
              id={`achievement-${achievement.id}`}
              order={index + 100}
              ariaLabel={`Achievement: ${achievement.title}. ${achievement.earned ? 'Earned' : 'Not earned'}. ${achievement.description}`}
            >
              <div className={cn(
                'flex items-center gap-3 p-3 rounded-lg border transition-all duration-300',
                achievement.earned
                  ? 'bg-yellow-500/10 border-yellow-500/30'
                  : 'bg-gray-500/10 border-gray-600/30'
              )}>
                <div className={cn(
                  'p-2 rounded-full',
                  achievement.earned ? 'bg-yellow-500/20' : 'bg-gray-500/20'
                )}>
                  <achievement.icon className={cn(
                    'h-4 w-4',
                    achievement.earned ? 'text-yellow-400' : 'text-gray-500'
                  )} />
                </div>
                <div className="flex-1">
                  <h4 className={cn(
                    'font-medium',
                    achievement.earned ? 'text-white' : 'text-gray-400'
                  )}>
                    {achievement.title}
                  </h4>
                  <p className="text-sm text-gray-500">{achievement.description}</p>
                  {achievement.earned && achievement.earnedDate && (
                    <p className="text-xs text-yellow-400 mt-1">
                      Earned on {new Date(achievement.earnedDate).toLocaleDateString()}
                    </p>
                  )}
                </div>
                {achievement.earned && (
                  <CheckCircle className="h-5 w-5 text-yellow-400" />
                )}
              </div>
            </NavigationElement>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
