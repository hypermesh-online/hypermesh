// Re-export generated crate status data
// Source: scripts/sync-status.ts generates these from crate-status.toml files
export { crateStatuses, type CrateStatus } from '@core/scripts/output/status';
export { crateStats, type CrateStats, totalStats } from '@core/scripts/output/stats';

// Lookup helpers
import { crateStatuses, type CrateStatus } from '@core/scripts/output/status';
import { crateStats, type CrateStats } from '@core/scripts/output/stats';

export function getCrateStatus(id: string): CrateStatus | undefined {
  return crateStatuses.find(c => c.id === id);
}

export function getCrateStats(id: string): CrateStats | undefined {
  return crateStats.find(c => c.id === id);
}

// Map crate IDs to dashboard module names
const crateToModule: Record<string, string> = {
  'block-matrix': 'hypermesh',
  'caesar': 'caesar',
  'catalog': 'catalog',
  'stoq': 'stoq',
  'trustchain': 'trustchain',
  'hypermesh-ebpf': 'ebpf',
  'hypermesh-lib': 'lib',
  'engauge': 'engauge',
  'gateway': 'gateway',
  'ui': 'ui',
};

export function getModuleName(crateId: string): string {
  return crateToModule[crateId] || crateId;
}
