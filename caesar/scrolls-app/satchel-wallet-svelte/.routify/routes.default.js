// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// @ts-nocheck


export const routes = {
  "meta": {},
  "id": "_default",
  "name": "",
  "file": {
    "path": "src/routes",
    "dir": "src",
    "base": "routes",
    "ext": "",
    "name": "routes"
  },
  "rootName": "default",
  "routifyDir": import.meta.url,
  "children": [
    {
      "meta": {},
      "id": "_default_assets",
      "name": "assets",
      "module": false,
      "file": {
        "path": "src/routes/assets",
        "dir": "src/routes",
        "base": "assets",
        "ext": "",
        "name": "assets"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_assets_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/assets/index.svelte",
            "dir": "src/routes/assets",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/assets/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_contracts",
      "name": "contracts",
      "module": false,
      "file": {
        "path": "src/routes/contracts",
        "dir": "src/routes",
        "base": "contracts",
        "ext": "",
        "name": "contracts"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_contracts_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/contracts/index.svelte",
            "dir": "src/routes/contracts",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/contracts/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_history",
      "name": "history",
      "module": false,
      "file": {
        "path": "src/routes/history",
        "dir": "src/routes",
        "base": "history",
        "ext": "",
        "name": "history"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_history_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/history/index.svelte",
            "dir": "src/routes/history",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/history/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_hypermesh",
      "name": "hypermesh",
      "module": false,
      "file": {
        "path": "src/routes/hypermesh",
        "dir": "src/routes",
        "base": "hypermesh",
        "ext": "",
        "name": "hypermesh"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_hypermesh_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/hypermesh/index.svelte",
            "dir": "src/routes/hypermesh",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/hypermesh/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {
        "isDefault": true
      },
      "id": "_default_index_svelte",
      "name": "index",
      "file": {
        "path": "src/routes/index.svelte",
        "dir": "src/routes",
        "base": "index.svelte",
        "ext": ".svelte",
        "name": "index"
      },
      "asyncModule": () => import('../src/routes/index.svelte'),
      "children": []
    },
    {
      "meta": {},
      "id": "_default_receive",
      "name": "receive",
      "module": false,
      "file": {
        "path": "src/routes/receive",
        "dir": "src/routes",
        "base": "receive",
        "ext": "",
        "name": "receive"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_receive_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/receive/index.svelte",
            "dir": "src/routes/receive",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/receive/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_send",
      "name": "send",
      "module": false,
      "file": {
        "path": "src/routes/send",
        "dir": "src/routes",
        "base": "send",
        "ext": "",
        "name": "send"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_send_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/send/index.svelte",
            "dir": "src/routes/send",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/send/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_services",
      "name": "services",
      "module": false,
      "file": {
        "path": "src/routes/services",
        "dir": "src/routes",
        "base": "services",
        "ext": "",
        "name": "services"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_services_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/services/index.svelte",
            "dir": "src/routes/services",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/services/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_settings",
      "name": "settings",
      "module": false,
      "file": {
        "path": "src/routes/settings",
        "dir": "src/routes",
        "base": "settings",
        "ext": "",
        "name": "settings"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_settings_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/settings/index.svelte",
            "dir": "src/routes/settings",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/settings/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_tokens",
      "name": "tokens",
      "module": false,
      "file": {
        "path": "src/routes/tokens",
        "dir": "src/routes",
        "base": "tokens",
        "ext": "",
        "name": "tokens"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_tokens_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/tokens/index.svelte",
            "dir": "src/routes/tokens",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/tokens/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {},
      "id": "_default_trade",
      "name": "trade",
      "module": false,
      "file": {
        "path": "src/routes/trade",
        "dir": "src/routes",
        "base": "trade",
        "ext": "",
        "name": "trade"
      },
      "children": [
        {
          "meta": {
            "isDefault": true
          },
          "id": "_default_trade_index_svelte",
          "name": "index",
          "file": {
            "path": "src/routes/trade/index.svelte",
            "dir": "src/routes/trade",
            "base": "index.svelte",
            "ext": ".svelte",
            "name": "index"
          },
          "asyncModule": () => import('../src/routes/trade/index.svelte'),
          "children": []
        }
      ]
    },
    {
      "meta": {
        "dynamic": true,
        "dynamicSpread": true,
        "order": false,
        "inline": false
      },
      "name": "[...404]",
      "file": {
        "path": ".routify/components/[...404].svelte",
        "dir": ".routify/components",
        "base": "[...404].svelte",
        "ext": ".svelte",
        "name": "[...404]"
      },
      "asyncModule": () => import('./components/[...404].svelte'),
      "children": []
    }
  ]
}
export default routes