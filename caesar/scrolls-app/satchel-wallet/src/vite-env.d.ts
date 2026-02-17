// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PLAID_CLIENT_ID: string
  readonly VITE_PLAID_SECRET: string
  readonly VITE_PLAID_ENVIRONMENT: 'sandbox' | 'development' | 'production'
  readonly VITE_ALCHEMY_API_KEY: string
  readonly VITE_INFURA_PROJECT_ID: string
  readonly VITE_ETHERSCAN_API_KEY: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}