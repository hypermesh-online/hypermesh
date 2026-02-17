// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route } from 'react-router-dom';
import { Gauge } from 'lucide-react';
import { 
  NgaugeOverview, 
  OnboardingManagement, 
  AdNetworkManagement, 
  AnalyticsDashboard,
  SubNavigation 
} from './ngauge';

export function NgaugeModule() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
          <Gauge className="h-8 w-8" />
          Ngauge
        </h1>
        <p className="text-muted-foreground">
          User Interface & Onboarding System - Privacy-first analytics and user engagement
        </p>
      </div>

      <SubNavigation />

      <Routes>
        <Route path="/" element={<NgaugeOverview />} />
        <Route path="/onboarding" element={<OnboardingManagement />} />
        <Route path="/ads" element={<AdNetworkManagement />} />
        <Route path="/analytics" element={<AnalyticsDashboard />} />
      </Routes>
    </div>
  );
}
