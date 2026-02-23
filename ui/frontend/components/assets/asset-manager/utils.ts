// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import {
  Cpu,
  HardDrive,
  MemoryStick,
  Network,
  Monitor,
  Package,
  Server,
  Shield,
  Lock,
  Users,
  Globe
} from 'lucide-react';

export function getAssetIcon(type: string) {
  switch (type) {
    case 'compute': case 'cpu': return Cpu;
    case 'storage': return HardDrive;
    case 'memory': return MemoryStick;
    case 'network': return Network;
    case 'vm': return Monitor;
    case 'application': return Package;
    default: return Server;
  }
}

export function getPrivacyIcon(level: string) {
  switch (level) {
    case 'private': return Lock;
    case 'federated': return Users;
    case 'public': return Globe;
    default: return Shield;
  }
}
