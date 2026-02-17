// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Router, createRouter } from '@roxi/routify'
import routes from './routes.default.js'

// remove previous routers to avoid bumping router names (/path => /1/path)
globalThis.__routify.reset()
export const router = createRouter({routes})
export { Router, routes }
