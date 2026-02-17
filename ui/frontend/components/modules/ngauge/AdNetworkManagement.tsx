// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { Target } from 'lucide-react';

export function AdNetworkManagement() {
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">Decentralized Ad Network</h2>
        <Button>
          <Target className="h-4 w-4 mr-2" />
          Create Campaign
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Daily Revenue</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">$1,234</div>
            <p className="text-xs text-muted-foreground">+18% from yesterday</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Active Campaigns</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">23</div>
            <p className="text-xs text-muted-foreground">8 high-performing</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Impressions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">2.4M</div>
            <p className="text-xs text-muted-foreground">+12% today</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Privacy Score</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">9.8/10</div>
            <p className="text-xs text-muted-foreground">Privacy-preserving</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="campaigns" className="space-y-4">
        <TabsList>
          <TabsTrigger value="campaigns">Ad Campaigns</TabsTrigger>
          <TabsTrigger value="targeting">Privacy Targeting</TabsTrigger>
          <TabsTrigger value="revenue">Revenue Analytics</TabsTrigger>
        </TabsList>

        <TabsContent value="campaigns">
          <Card>
            <CardHeader>
              <CardTitle>Active Campaigns</CardTitle>
              <CardDescription>Current advertising campaigns and performance</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {[
                  { name: 'Web3 Infrastructure Tools', budget: '$500', spent: '$234', impressions: '145K', ctr: '2.3%', status: 'active' },
                  { name: 'Decentralized Computing', budget: '$750', spent: '$489', impressions: '234K', ctr: '1.8%', status: 'active' },
                  { name: 'Blockchain Development', budget: '$300', spent: '$156', impressions: '89K', ctr: '3.1%', status: 'paused' },
                  { name: 'Privacy-First Analytics', budget: '$600', spent: '$445', impressions: '178K', ctr: '2.7%', status: 'active' },
                ].map((campaign, i) => (
                  <div key={i} className="flex items-center justify-between p-4 border rounded-lg">
                    <div>
                      <h4 className="font-medium">{campaign.name}</h4>
                      <p className="text-sm text-muted-foreground">
                        {campaign.impressions} impressions • CTR: {campaign.ctr}
                      </p>
                      <div className="flex gap-2 mt-1">
                        <Badge variant="outline">Budget: {campaign.budget}</Badge>
                        <Badge variant="outline">Spent: {campaign.spent}</Badge>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant={campaign.status === 'active' ? 'default' : 'secondary'}>
                        {campaign.status}
                      </Badge>
                      <Button variant="outline" size="sm">Edit</Button>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="targeting">
          <Card>
            <CardHeader>
              <CardTitle>Privacy-Preserving Targeting</CardTitle>
              <CardDescription>Audience targeting without compromising user privacy</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-6">
                <div>
                  <h4 className="font-medium mb-4">Targeting Methods</h4>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="p-4 border rounded-lg">
                      <h5 className="font-medium mb-2">Contextual Targeting</h5>
                      <p className="text-sm text-muted-foreground mb-3">
                        Target based on content context, not user data
                      </p>
                      <div className="flex justify-between text-sm">
                        <span>Privacy Score:</span>
                        <span className="font-medium text-green-600">10/10</span>
                      </div>
                    </div>

                    <div className="p-4 border rounded-lg">
                      <h5 className="font-medium mb-2">Federated Learning</h5>
                      <p className="text-sm text-muted-foreground mb-3">
                        Learn patterns without accessing individual data
                      </p>
                      <div className="flex justify-between text-sm">
                        <span>Privacy Score:</span>
                        <span className="font-medium text-green-600">9/10</span>
                      </div>
                    </div>

                    <div className="p-4 border rounded-lg">
                      <h5 className="font-medium mb-2">Cohort Analysis</h5>
                      <p className="text-sm text-muted-foreground mb-3">
                        Target user groups without individual tracking
                      </p>
                      <div className="flex justify-between text-sm">
                        <span>Privacy Score:</span>
                        <span className="font-medium text-green-600">8/10</span>
                      </div>
                    </div>

                    <div className="p-4 border rounded-lg">
                      <h5 className="font-medium mb-2">Interest Signals</h5>
                      <p className="text-sm text-muted-foreground mb-3">
                        Use declared interests and preferences
                      </p>
                      <div className="flex justify-between text-sm">
                        <span>Privacy Score:</span>
                        <span className="font-medium text-green-600">9/10</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div>
                  <h4 className="font-medium mb-4">Audience Insights</h4>
                  <div className="space-y-3">
                    {[
                      { segment: 'Web3 Developers', size: '1.2K', engagement: '3.4%', privacy: 'Full anonymity' },
                      { segment: 'Crypto Enthusiasts', size: '856', engagement: '2.8%', privacy: 'Full anonymity' },
                      { segment: 'Privacy Advocates', size: '634', engagement: '4.1%', privacy: 'Full anonymity' },
                      { segment: 'Tech Professionals', size: '1.8K', engagement: '2.5%', privacy: 'Full anonymity' },
                    ].map((segment, i) => (
                      <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                        <div>
                          <p className="font-medium">{segment.segment}</p>
                          <p className="text-sm text-muted-foreground">{segment.privacy}</p>
                        </div>
                        <div className="text-right">
                          <p className="font-medium">{segment.size} users</p>
                          <p className="text-sm text-muted-foreground">{segment.engagement} engagement</p>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="revenue">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Revenue Breakdown</CardTitle>
                <CardDescription>Ad revenue distribution by category</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[
                    { category: 'Display Ads', revenue: '$567', percentage: 46 },
                    { category: 'Sponsored Content', revenue: '$345', percentage: 28 },
                    { category: 'Native Ads', revenue: '$234', percentage: 19 },
                    { category: 'Video Ads', revenue: '$88', percentage: 7 },
                  ].map((item) => (
                    <div key={item.category} className="space-y-2">
                      <div className="flex justify-between">
                        <span className="text-sm font-medium">{item.category}</span>
                        <span className="text-sm">{item.revenue}</span>
                      </div>
                      <Progress value={item.percentage} />
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Performance Metrics</CardTitle>
                <CardDescription>Key advertising performance indicators</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[
                    { metric: 'Click-through Rate', value: '2.3%', trend: '+0.2%' },
                    { metric: 'Cost per Click', value: '$0.45', trend: '-$0.05' },
                    { metric: 'Conversion Rate', value: '1.8%', trend: '+0.3%' },
                    { metric: 'Revenue per User', value: '$0.58', trend: '+$0.08' },
                  ].map((metric, i) => (
                    <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                      <span className="font-medium">{metric.metric}</span>
                      <div className="text-right">
                        <p className="font-medium">{metric.value}</p>
                        <p className={cn(
                          "text-sm",
                          metric.trend.startsWith('+') ? 'text-green-600' : 'text-red-600'
                        )}>
                          {metric.trend}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}