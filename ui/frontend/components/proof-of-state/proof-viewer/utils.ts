// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { HardDrive, Users, Zap, Clock, Shield } from 'lucide-react';

export function getProofIcon(type: string) {
  switch (type) {
    case 'PoSp': return HardDrive;
    case 'PoSt': return Users;
    case 'PoWk': return Zap;
    case 'PoTm': return Clock;
    default: return Shield;
  }
}

export function getProofDescription(type: string) {
  switch (type) {
    case 'PoSp': return 'WHERE - Storage location and physical/network location verification';
    case 'PoSt': return 'WHO - Ownership, access rights, and economic stake validation';
    case 'PoWk': return 'WHAT/HOW - Computational resources and processing validation';
    case 'PoTm': return 'WHEN - Temporal ordering and timestamp validation';
    default: return 'Unknown proof type';
  }
}

export function getProofColor(type: string) {
  switch (type) {
    case 'PoSp': return 'blue';
    case 'PoSt': return 'green';
    case 'PoWk': return 'yellow';
    case 'PoTm': return 'purple';
    default: return 'gray';
  }
}
