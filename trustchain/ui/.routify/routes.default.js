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
      "id": "_default_caesar_svelte",
      "name": "caesar",
      "file": {
        "path": "src/routes/caesar.svelte",
        "dir": "src/routes",
        "base": "caesar.svelte",
        "ext": ".svelte",
        "name": "caesar"
      },
      "asyncModule": () => import('../src/routes/caesar.svelte'),
      "children": []
    },
    {
      "meta": {},
      "id": "_default_consensus_svelte",
      "name": "consensus",
      "file": {
        "path": "src/routes/consensus.svelte",
        "dir": "src/routes",
        "base": "consensus.svelte",
        "ext": ".svelte",
        "name": "consensus"
      },
      "asyncModule": () => import('../src/routes/consensus.svelte'),
      "children": []
    },
    {
      "meta": {},
      "id": "_default_hypermesh_svelte",
      "name": "hypermesh",
      "file": {
        "path": "src/routes/hypermesh.svelte",
        "dir": "src/routes",
        "base": "hypermesh.svelte",
        "ext": ".svelte",
        "name": "hypermesh"
      },
      "asyncModule": () => import('../src/routes/hypermesh.svelte'),
      "children": []
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
      "id": "_default_settings_svelte",
      "name": "settings",
      "file": {
        "path": "src/routes/settings.svelte",
        "dir": "src/routes",
        "base": "settings.svelte",
        "ext": ".svelte",
        "name": "settings"
      },
      "asyncModule": () => import('../src/routes/settings.svelte'),
      "children": []
    },
    {
      "meta": {},
      "id": "_default_stoq_svelte",
      "name": "stoq",
      "file": {
        "path": "src/routes/stoq.svelte",
        "dir": "src/routes",
        "base": "stoq.svelte",
        "ext": ".svelte",
        "name": "stoq"
      },
      "asyncModule": () => import('../src/routes/stoq.svelte'),
      "children": []
    },
    {
      "meta": {},
      "id": "_default_trustchain_svelte",
      "name": "trustchain",
      "file": {
        "path": "src/routes/trustchain.svelte",
        "dir": "src/routes",
        "base": "trustchain.svelte",
        "ext": ".svelte",
        "name": "trustchain"
      },
      "asyncModule": () => import('../src/routes/trustchain.svelte'),
      "children": []
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