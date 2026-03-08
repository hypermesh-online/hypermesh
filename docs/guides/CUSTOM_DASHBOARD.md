# Custom Dashboard Guide

Dashboards in HyperMesh are **blockchain assets**. When you deploy a dashboard, it gets:

1. Bundled into a binary blob
2. BLAKE3 content-hashed
3. Registered as a `Dashboard` asset on your node's blockchain
4. Stored in the asset store keyed by content hash
5. Served by the HTTP API from the blockchain (not from a filesystem directory)

## Architecture

```
deploy → bundle files → BLAKE3 hash → register on blockchain → store in asset store
                                                                       ↓
HTTP request → query blockchain for active Dashboard → load bundle by hash → unbundle → serve
```

The blockchain is the **source of truth**. The `dashboard.list` and `dashboard.info` IPC commands query the chain, not the filesystem.

## Dashboard Scopes

Every dashboard has two scopes:

| Scope | Purpose | Default URL |
|-------|---------|-------------|
| `public` | Public-facing landing page | `/public/` |
| `private` | Full node management UI (metrics, controls, admin) | `/` (root) |

The **private** scope is served at the root path — it's the full UI for the node operator, including all management controls (DNS registration, peer connect, domain management, config viewer, node shutdown).

The **public** scope is an access scope — it defines content that's visible without authentication. It does NOT imply clearnet HTTP access. Whether a node exposes a clearnet HTTP gateway is the operator's choice (e.g., via the gateway crate). The node serves both scopes via the same HTTP API regardless of transport.

## Project Structure

```
my-dashboard/
├── dashboard.toml          # Manifest (required)
├── dist/
│   ├── public/             # Public scope files
│   │   └── index.html
│   └── private/            # Private scope files (your main UI)
│       ├── index.html
│       └── assets/
│           ├── app.js
│           └── style.css
```

## Manifest Format

```toml
[dashboard]
name = "my-dashboard"
version = "1.0.0"
description = "My custom HyperMesh dashboard"
domain = "my-dashboard.hypermesh"

[access]
public = "dist/public/"
private = "dist/private/"

[dependencies]
# Optional: catalog packages your dashboard depends on
```

## Commands

### Scaffold a new project

```bash
hypermesh dashboard init my-dashboard
```

Creates a project skeleton with `dashboard.toml` and placeholder HTML for both scopes.

### Deploy

```bash
hypermesh dashboard deploy ./my-dashboard/
```

This:
1. Reads `dashboard.toml`
2. Validates the manifest and referenced directories
3. Collects all files from the scope directories
4. Bundles them into a single blob
5. Registers the bundle as a `Dashboard` asset on the blockchain
6. Stores the bundle in the asset store

If the node daemon is running, deploy goes through IPC. Otherwise it does a direct blockchain write.

### List deployed dashboards

```bash
hypermesh dashboard list
```

Queries the blockchain for all `Dashboard`-type assets.

### Get dashboard info

```bash
hypermesh dashboard info --name my-dashboard
```

## Using React / Vite / Next.js

Build your frontend as usual, then point the manifest at the build output:

```toml
[dashboard]
name = "my-react-dashboard"
version = "1.0.0"
description = "React-based HyperMesh dashboard"
domain = "my-dashboard.hypermesh"

[access]
private = "dist/"
public = "dist-public/"
```

Build and deploy:

```bash
cd my-react-dashboard
npm run build          # or: bun run build
hypermesh dashboard deploy .
```

The HTTP API serves SPA-style: any path not matching a file falls back to the scope's `index.html`, so client-side routing (React Router, etc.) works out of the box.

## API Endpoints

Your dashboard JS can call these REST endpoints via the Gateway (`[::1]:8443`):

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/status` | GET | Node status |
| `/api/v1/blockchain/height` | GET | Chain height |
| `/api/v1/blockchain/block/:index` | GET | Block by index |
| `/api/v1/blockchain/validate` | GET | Validate chain |
| `/api/v1/dns/list` | GET | DNS records |
| `/api/v1/dns/resolve/:name` | GET | Resolve DNS name |
| `/api/v1/dns/register` | POST | Register DNS record |
| `/api/v1/network/peers` | GET | Connected peers |
| `/api/v1/topology/info` | GET | Matrix topology |
| `/api/v1/topology/neighbors` | GET | Neighbor nodes |
| `/api/v1/asset/list` | GET | Registered assets |
| `/api/v1/dashboard/list` | GET | Deployed dashboards |
| `/api/v1/dashboard/info` | GET | Dashboard details |
| `/api/v1/config/show` | GET | Node config |
| `/api/v1/config/get/:key` | GET | Config value |
| `/api/v1/domain/list` | GET | Domain registrations |
| `/api/v1/domain/register` | POST | Register domain |
| `/api/v1/domain/join` | POST | Join domain network |

## Default System Dashboard

On first boot, each node auto-registers a default `Dashboard` asset with:
- **Public**: "Hello HyperMesh" onboarding page
- **Private**: Full management dashboard (blockchain, peers, DNS, assets, domains, config, controls)

Deploy a custom dashboard to replace the default. The most recent `Dashboard` asset on the blockchain is the one that gets served.

## How It Works Internally

1. The HTTP API receives a non-API GET request (e.g., `/` or `/assets/app.js`)
2. It queries the node's blockchain for the most recently registered `Dashboard` asset
3. It loads the bundle from the asset store using the asset's `content_hash`
4. It unbundles the files into a map
5. It matches the request path to a file in the appropriate scope
6. If no exact match, SPA fallback serves the scope's `index.html`

Every dashboard access is a blockchain query — the chain is the source of truth.
