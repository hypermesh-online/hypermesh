// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — Wizard barrel.

export { SetupWizard, useSetupWizardGate } from './SetupWizard';
export {
  isTauri,
  invokeOrFallback,
  listenOrNoop,
  wizard,
  daemon,
  type DaemonStatus,
  type DaemonStartArgs,
  type WizardState,
} from './tauriBridge';
