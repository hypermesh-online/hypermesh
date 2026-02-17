// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { 
  Users, 
  DollarSign,
  TrendingUp,
  Shield,
  Star,
  Target
} from 'lucide-react';

export function NgaugeOverview() {
  const platformStats = {
    totalUsers: '2,145',
    newUsersToday: 47,
    adRevenue: '$1,234',
    engagementRate: 78
  };

  const recentActivity = [
    { type: 'user', message: 'New user completed onboarding', time: '2 minutes ago' },
    { type: 'ad', message: 'Ad campaign reached 10K impressions', time: '15 minutes ago' },
    { type: 'achievement', message: 'User earned "Resource Sharer" badge', time: '23 minutes ago' },
    { type: 'revenue', message: 'Daily ad revenue goal achieved', time: '1 hour ago' },
  ];

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Users</CardTitle>
            <Users className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{platformStats.totalUsers}</div>
            <p className="text-xs text-muted-foreground">+{platformStats.newUsersToday} today</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Ad Revenue</CardTitle>
            <DollarSign className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{platformStats.adRevenue}</div>
            <p className="text-xs text-muted-foreground">+12% from yesterday</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Engagement Rate</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{platformStats.engagementRate}%</div>
            <Progress value={platformStats.engagementRate} className="mt-2" />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Privacy Score</CardTitle>
            <Shield className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">9.2/10</div>
            <p className="text-xs text-muted-foreground">Privacy-first approach</p>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>User Onboarding Progress</CardTitle>
            <CardDescription>Weekly onboarding completion rates</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex justify-between items-center p-3 rounded-lg bg-muted/50">
                <div>
                  <p className="font-medium">Welcome & Setup</p>
                  <p className="text-sm text-muted-foreground">Basic account creation</p>
                </div>
                <div className="text-right">
                  <p className="text-2xl font-bold">94%</p>
                  <Badge variant="default">Excellent</Badge>
                </div>
              </div>

              <div className="flex justify-between items-center p-3 rounded-lg bg-muted/50">
                <div>
                  <p className="font-medium">Resource Discovery</p>
                  <p className="text-sm text-muted-foreground">Finding shareable resources</p>
                </div>
                <div className="text-right">
                  <p className="text-2xl font-bold">67%</p>
                  <Badge variant="secondary">Good</Badge>
                </div>
              </div>

              <div className="flex justify-between items-center p-3 rounded-lg bg-muted/50">
                <div>
                  <p className="font-medium">First Interaction</p>
                  <p className="text-sm text-muted-foreground">First meaningful action</p>
                </div>
                <div className="text-right">
                  <p className="text-2xl font-bold">45%</p>
                  <Badge variant="destructive">Needs Work</Badge>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent Platform Activity</CardTitle>
            <CardDescription>Latest user and system events</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentActivity.map((activity, index) => (
                <div key={index} className="flex items-center gap-4 p-3 rounded-lg bg-muted/50">
                  <div className={cn(
                    "w-2 h-2 rounded-full",
                    activity.type === 'user' ? 'bg-blue-500' :
                    activity.type === 'ad' ? 'bg-green-500' :
                    activity.type === 'achievement' ? 'bg-yellow-500' : 'bg-purple-500'
                  )} />
                  <div className="flex-1">
                    <p className="text-sm">{activity.message}</p>
                    <p className="text-xs text-muted-foreground">{activity.time}</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>User Engagement & Achievements</CardTitle>
          <CardDescription>Gamification metrics and user progression</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="p-4 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <Star className="h-4 w-4 text-yellow-500" />
                <h4 className="font-medium">Achievement System</h4>
              </div>
              <div className="text-2xl font-bold mb-1">156</div>
              <p className="text-xs text-muted-foreground">Badges earned this week</p>
              <Progress value={78} className="mt-2" />
            </div>

            <div className="p-4 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <Users className="h-4 w-4 text-blue-500" />
                <h4 className="font-medium">Community Growth</h4>
              </div>
              <div className="text-2xl font-bold mb-1">89%</div>
              <p className="text-xs text-muted-foreground">User retention rate</p>
              <Progress value={89} className="mt-2" />
            </div>

            <div className="p-4 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <Target className="h-4 w-4 text-green-500" />
                <h4 className="font-medium">Goal Completion</h4>
              </div>
              <div className="text-2xl font-bold mb-1">67%</div>
              <p className="text-xs text-muted-foreground">Monthly objectives met</p>
              <Progress value={67} className="mt-2" />
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}