// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.


        import * as module from '../src/App.svelte'
        import { renderModule } from '@roxi/routify/tools/ssr5.js'
        import { map } from './route-map.js'

        export const render = url => renderModule(module, { url, routesMap: map })