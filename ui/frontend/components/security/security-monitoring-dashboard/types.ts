// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface SecurityAlert {
  id: string;
  type: 'certificate' | 'byzantine' | 'network' | 'audit';
  severity: 'low' | 'medium' | 'high' | 'critical';
  title: string;
  description: string;
  timestamp: string;
  resolved: boolean;
}

export interface SecurityMetrics {
  certificateHealth: number;
  networkSecurity: number;
  threatLevel: 'low' | 'medium' | 'high' | 'critical';
  activeThreats: number;
  lastAudit: string;
  auditScore: number;
}
