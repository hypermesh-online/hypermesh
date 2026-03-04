// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import {
  HardDrive,
  Users,
  Zap,
  Clock,
  Shield,
  Database,
  Network,
  Activity
} from 'lucide-react';

export function getOperationIcon(type: string) {
  switch (type) {
    case 'asset_creation': return Database;
    case 'asset_allocation': return Users;
    case 'state_proof_validation': return Shield;
    case 'proxy_setup': return Network;
    default: return Activity;
  }
}

export function getProofIcon(type: string) {
  switch (type) {
    case 'PoSp': return HardDrive;
    case 'PoSt': return Users;
    case 'PoWk': return Zap;
    case 'PoTm': return Clock;
    default: return Shield;
  }
}
