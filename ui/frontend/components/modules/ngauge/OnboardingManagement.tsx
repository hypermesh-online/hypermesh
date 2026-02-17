// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Users, Star } from 'lucide-react';

export function OnboardingManagement() {
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">User Onboarding</h2>
        <Button>
          <Users className="h-4 w-4 mr-2" />
          Customize Flow
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">New Users Today</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">47</div>
            <p className="text-xs text-muted-foreground">+8 from yesterday</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Completion Rate</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">78%</div>
            <Progress value={78} className="mt-2" />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Avg Time to Complete</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">12.3m</div>
            <p className="text-xs text-muted-foreground">-2.1m improvement</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Drop-off Rate</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">22%</div>
            <p className="text-xs text-muted-foreground">Needs attention</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="flow" className="space-y-4">
        <TabsList>
          <TabsTrigger value="flow">Onboarding Flow</TabsTrigger>
          <TabsTrigger value="analytics">Analytics</TabsTrigger>
          <TabsTrigger value="customization">Customization</TabsTrigger>
        </TabsList>

        <TabsContent value="flow">
          <Card>
            <CardHeader>
              <CardTitle>Onboarding Steps</CardTitle>
              <CardDescription>Current user onboarding journey</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {[
                  { step: 1, title: 'Welcome & Introduction', completion: 94, dropoff: 6, description: 'Platform overview and value proposition' },
                  { step: 2, title: 'Account Setup', completion: 89, dropoff: 11, description: 'Basic profile and security configuration' },
                  { step: 3, title: 'Resource Discovery', completion: 67, dropoff: 33, description: 'Identify shareable computing resources' },
                  { step: 4, title: 'Privacy Configuration', completion: 78, dropoff: 22, description: 'Set privacy preferences and data sharing' },
                  { step: 5, title: 'First Action', completion: 45, dropoff: 55, description: 'Complete first meaningful platform interaction' },
                ].map((step) => (
                  <div key={step.step} className="flex items-center justify-between p-4 border rounded-lg">
                    <div className="flex items-center gap-4">
                      <div className="w-8 h-8 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm font-medium">
                        {step.step}
                      </div>
                      <div>
                        <h4 className="font-medium">{step.title}</h4>
                        <p className="text-sm text-muted-foreground">{step.description}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="font-medium">{step.completion}% complete</p>
                      <p className="text-sm text-muted-foreground">{step.dropoff}% drop-off</p>
                      <Progress value={step.completion} className="mt-1 w-24" />
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="analytics">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Completion Funnel</CardTitle>
                <CardDescription>User progression through onboarding steps</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[
                    { stage: 'Started', users: 100, percentage: 100 },
                    { stage: 'Account Created', users: 94, percentage: 94 },
                    { stage: 'Profile Setup', users: 89, percentage: 89 },
                    { stage: 'Resources Added', users: 67, percentage: 67 },
                    { stage: 'First Action', users: 45, percentage: 45 },
                  ].map((stage, i) => (
                    <div key={i} className="space-y-2">
                      <div className="flex justify-between">
                        <span className="text-sm font-medium">{stage.stage}</span>
                        <span className="text-sm">{stage.users}% ({stage.percentage} users)</span>
                      </div>
                      <Progress value={stage.percentage} />
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>User Feedback</CardTitle>
                <CardDescription>Onboarding experience ratings</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {[
                    { aspect: 'Ease of Use', rating: 4.2, responses: 123 },
                    { aspect: 'Clarity of Instructions', rating: 3.8, responses: 145 },
                    { aspect: 'Time to Complete', rating: 4.5, responses: 98 },
                    { aspect: 'Privacy Transparency', rating: 4.7, responses: 156 },
                  ].map((feedback, i) => (
                    <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                      <div>
                        <p className="font-medium">{feedback.aspect}</p>
                        <p className="text-sm text-muted-foreground">{feedback.responses} responses</p>
                      </div>
                      <div className="text-right">
                        <div className="flex items-center gap-1">
                          <Star className="h-4 w-4 text-yellow-500" />
                          <span className="font-medium">{feedback.rating}</span>
                        </div>
                        <Progress value={feedback.rating * 20} className="mt-1 w-16" />
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="customization">
          <Card>
            <CardHeader>
              <CardTitle>Onboarding Customization</CardTitle>
              <CardDescription>Tailor the onboarding experience for different user types</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-6">
                <div>
                  <h4 className="font-medium mb-4">User Type Paths</h4>
                  <div className="grid gap-4 md:grid-cols-2">
                    {[
                      { type: 'Technical Users', path: 'Advanced setup with detailed configuration options', users: 156 },
                      { type: 'Casual Users', path: 'Simplified flow with guided assistance', users: 234 },
                      { type: 'Enterprise Users', path: 'Business-focused with compliance features', users: 89 },
                      { type: 'Power Users', path: 'Skip basics, focus on advanced features', users: 67 },
                    ].map((userType, i) => (
                      <div key={i} className="p-4 border rounded-lg">
                        <div className="flex items-center justify-between mb-2">
                          <h5 className="font-medium">{userType.type}</h5>
                          <Badge variant="outline">{userType.users} users</Badge>
                        </div>
                        <p className="text-sm text-muted-foreground mb-3">{userType.path}</p>
                        <Button variant="outline" size="sm">Customize</Button>
                      </div>
                    ))}
                  </div>
                </div>

                <div>
                  <h4 className="font-medium mb-4">A/B Testing</h4>
                  <div className="space-y-3">
                    {[
                      { test: 'Welcome Video vs Text', winner: 'Video', improvement: '+12% completion' },
                      { test: 'Single Page vs Multi-Step', winner: 'Multi-Step', improvement: '+8% engagement' },
                      { test: 'Gamification Elements', winner: 'With Badges', improvement: '+15% retention' },
                    ].map((test, i) => (
                      <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
                        <div>
                          <p className="font-medium">{test.test}</p>
                          <p className="text-sm text-muted-foreground">Winner: {test.winner}</p>
                        </div>
                        <Badge variant="default">{test.improvement}</Badge>
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