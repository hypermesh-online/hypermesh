// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Asset Management Hooks - Barrel re-export
 *
 * All hooks have been split into domain-specific files for maintainability.
 * This file re-exports everything to preserve existing import paths.
 */

// Core asset CRUD operations
export {
  useAssets,
  useAsset,
  useCreateAsset,
  useUpdateAsset,
  useDeleteAsset
} from './useAssetCrud';

// Allocation management
export {
  useRequestAllocation,
  useAllocations,
  useReleaseAllocation
} from './useAllocations';

// State proof validation (NOT consensus -- HyperMesh uses bilateral Proof of State)
export {
  useValidateStateProof,
  useStateProofHistory,
  useSubmitProof
} from './useStateProof';

// Byzantine fault detection
export {
  useByzantineDetections,
  useReportByzantineBehavior
} from './useByzantine';

// Remote proxy management
export {
  useRemoteProxies,
  useCreateRemoteProxy,
  useUpdateRemoteProxy,
  useValidateProxyTrust
} from './useProxies';

// Network, node health, and system operations
export {
  useNodeHealth,
  useNetworkTopology,
  useExecuteRemoteOperation,
  useHyperMeshSystemStatus
} from './useNetwork';

// VM asset integration with Catalog
export {
  useCatalogApplications,
  useCreateVMAsset,
  useInstallCatalogApplication,
  useExecuteVMAsset,
  useVMExecutions,
  useVMExecution,
  useCancelVMExecution,
  useVMAssets,
  useUpdateVMAsset
} from './useVMAssets';
