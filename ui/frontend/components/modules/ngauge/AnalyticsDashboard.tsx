// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { BarChart3 } from 'lucide-react';

export function AnalyticsDashboard() {
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">Privacy-Preserving Analytics</h2>
        <Button>
          <BarChart3 className="h-4 w-4 mr-2" />
          Export Report
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Page Views</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">45.6K</div>
            <p className="text-xs text-muted-foreground">+15% this week</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Unique Visitors</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">2.1K</div>
            <p className="text-xs text-muted-foreground">+8% from last week</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Avg Session Time</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">8m 23s</div>
            <p className="text-xs text-muted-foreground">+45s improvement</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Bounce Rate</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">34%</div>
            <p className="text-xs text-muted-foreground">-5% improvement</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="behavior" className="space-y-4">
        <TabsList>
          <TabsTrigger value="behavior">User Behavior</TabsTrigger>
          <TabsTrigger value="engagement">Engagement</TabsTrigger>
          <TabsTrigger value="privacy">Privacy Metrics</TabsTrigger>
        </TabsList>

        <TabsContent value="behavior">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Popular Pages</CardTitle>
                <CardDescription>Most visited pages (anonymized data)</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[
                    { page: '/dashboard', views: '12.3K', time: '5m 45s' },
                    { page: '/onboarding', views: '8.9K', time: '12m 15s' },
                    { page: '/resources', views: '6.7K', time: '3m 30s' },
                    { page: '/earnings', views: '5.4K', time: '4m 20s' },
                    { page: '/settings', views: '3.2K', time: '2m 10s' },
                  ].map((page, i) => (
                    <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                      <div>
                        <p className="font-medium font-mono">{page.page}</p>
                        <p className="text-sm text-muted-foreground">Avg time: {page.time}</p>
                      </div>
                      <span className="font-medium">{page.views}</span>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>User Flow</CardTitle>
                <CardDescription>Common navigation patterns</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[
                    { flow: 'Landing → Onboarding → Dashboard', users: '45%', conversion: '78%' },
                    { flow: 'Dashboard → Resources → Configuration', users: '32%', conversion: '65%' },
                    { flow: 'Settings → Privacy → Preferences', users: '23%', conversion: '89%' },
                    { flow: 'Earnings → Exchange → Wallet', users: '18%', conversion: '56%' },
                  ].map((flow, i) => (
                    <div key={i} className="p-3 rounded-lg bg-muted/50">
                      <p className="font-medium text-sm">{flow.flow}</p>
                      <div className="flex justify-between mt-1">
                        <span className="text-sm text-muted-foreground">{flow.users} of users</span>
                        <span className="text-sm font-medium">{flow.conversion} conversion</span>
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="engagement">
          <Card>
            <CardHeader>
              <CardTitle>User Engagement Metrics</CardTitle>
              <CardDescription>How users interact with the platform</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid gap-6 lg:grid-cols-2">
                <div className="space-y-4">
                  <h4 className="font-medium">Feature Usage</h4>
                  {[
                    { feature: 'Resource Sharing', usage: 87, trend: '+12%' },
                    { feature: 'Earnings Dashboard', usage: 78, trend: '+8%' },
                    { feature: 'Privacy Controls', usage: 94, trend: '+5%' },
                    { feature: 'Community Features', usage: 45, trend: '+23%' },
                  ].map((feature) => (
                    <div key={feature.feature} className="space-y-2">
                      <div className="flex justify-between">
                        <span className="text-sm font-medium">{feature.feature}</span>
                        <span className="text-sm">{feature.usage}% ({feature.trend})</span>
                      </div>
                      <Progress value={feature.usage} />
                    </div>
                  ))}
                </div>

                <div className="space-y-4">
                  <h4 className="font-medium">Retention Cohorts</h4>
                  {[
                    { period: 'Day 1', retention: 89 },
                    { period: 'Day 7', retention: 67 },
                    { period: 'Day 30', retention: 45 },
                    { period: 'Day 90', retention: 34 },
                  ].map((cohort) => (
                    <div key={cohort.period} className="space-y-2">
                      <div className="flex justify-between">
                        <span className="text-sm font-medium">{cohort.period}</span>
                        <span className="text-sm">{cohort.retention}%</span>
                      </div>
                      <Progress value={cohort.retention} />
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="privacy">
          <Card>
            <CardHeader>
              <CardTitle>Privacy Protection Metrics</CardTitle>
              <CardDescription>How well user privacy is preserved</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-6">
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="p-4 rounded-lg bg-muted/50">
                    <h4 className="font-medium mb-2">Data Anonymization</h4>
                    <div className="text-2xl font-bold text-green-600 mb-1">100%</div>
                    <p className="text-xs text-muted-foreground">All data anonymized</p>
                  </div>

                  <div className="p-4 rounded-lg bg-muted/50">
                    <h4 className="font-medium mb-2">Consent Rate</h4>
                    <div className="text-2xl font-bold text-green-600 mb-1">94%</div>
                    <p className="text-xs text-muted-foreground">Explicit consent given</p>
                  </div>

                  <div className="p-4 rounded-lg bg-muted/50">
                    <h4 className="font-medium mb-2">Data Minimization</h4>
                    <div className="text-2xl font-bold text-green-600 mb-1">87%</div>
                    <p className="text-xs text-muted-foreground">Unnecessary data avoided</p>
                  </div>
                </div>

                <div>
                  <h4 className="font-medium mb-4">Privacy Controls Usage</h4>
                  <div className="space-y-3">
                    {[
                      { control: 'Data Sharing Opt-out', usage: '78%', description: 'Users who disabled data sharing' },
                      { control: 'Analytics Tracking', usage: '65%', description: 'Users who allow anonymous analytics' },
                      { control: 'Targeted Ads', usage: '23%', description: 'Users who enable personalized ads' },
                      { control: 'Data Export', usage: '12%', description: 'Users who exported their data' },
                    ].map((control, i) => (
                      <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                        <div>
                          <p className="font-medium">{control.control}</p>
                          <p className="text-sm text-muted-foreground">{control.description}</p>
                        </div>
                        <span className="font-medium">{control.usage}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}