# HyperMesh Go SDK Guide

## Installation

```bash
go get github.com/hypermesh-online/sdk-go
```

Zero external dependencies -- uses only the Go standard library (`net/http`, `encoding/json`).

Requires Go 1.21+.

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    hypermesh "github.com/hypermesh-online/sdk-go"
)

func main() {
    client := hypermesh.NewClient("") // defaults to http://localhost:9293

    status, err := client.Node.Status(context.Background())
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println(status.NodeID, status.Status)
}
```

Custom endpoint:

```go
client := hypermesh.NewClient("http://192.168.1.50:9293")
```

With options:

```go
client := hypermesh.NewClient("",
    hypermesh.WithTimeout(60 * time.Second),
)
```

With a custom `http.Client`:

```go
httpClient := &http.Client{Timeout: 60 * time.Second}
client := hypermesh.NewClient("",
    hypermesh.WithHTTPClient(httpClient),
)
```

## API Reference

All methods accept a `context.Context` as the first argument for cancellation
and timeouts.

### Node

```go
// Get full node status
status, err := client.Node.Status(ctx)
// Fields: NodeID, Uptime, Version, Coordinate, Networks, Status

// Ping the node
pong, err := client.Node.Ping(ctx)
// Fields: Pong (bool), NodeID
```

### Blockchain

```go
// Get current chain height
h, err := client.Blockchain.Height(ctx)
// Fields: Height (uint64)

// Get a specific block by index
block, err := client.Blockchain.Block(ctx, 0)
// Fields: Index, Timestamp, PreviousHash, Hash, Data, StateProof

// Validate blockchain integrity
result, err := client.Blockchain.Validate(ctx)
// Fields: Valid (bool), Errors ([]string)
```

### DNS

```go
// List all DNS records
dns, err := client.Dns.List(ctx)
// Fields: Records ([]DnsRecord)
// DnsRecord: Name, Address, TTL, NodeID

// Resolve a name
record, err := client.Dns.Resolve(ctx, "trust.hypermesh")
// Fields: Name, Address

// Register a new DNS record
err = client.Dns.Register(ctx, "mynode.hypermesh", "::1")
```

### Assets

```go
// List all registered assets
assets, err := client.Asset.List(ctx)
// Fields: Assets ([]Asset)
// Asset: ID, Type, State, Owner, Metadata
```

### Domain

```go
// List registered domains
domains, err := client.Domain.List(ctx)
// Fields: Domains ([]Domain)
// Domain: Name, Privacy, Owner

// Register a new domain
err = client.Domain.Register(ctx, "myapp", "Private")

// Join an existing domain with invitation token
err = client.Domain.Join(ctx, "myapp", "invitation-token")
// Without token:
err = client.Domain.Join(ctx, "myapp", "")
```

### Dashboard

```go
// List available dashboards
dashboards, err := client.Dashboard.List(ctx)
// Fields: Dashboards ([]DashboardEntry)
// DashboardEntry: ID, Name, Path

// Get dashboard info
info, err := client.Dashboard.Info(ctx)
// Fields: ID, Name, Version
```

### Config

```go
// Show full node configuration
config, err := client.Config.Show(ctx)
// Returns: map[string]any

// Get a specific config value
val, err := client.Config.Get(ctx, "privacy_mode")
// Fields: Key, Value (any)
```

### Network

```go
// List connected peers
peers, err := client.Network.Peers(ctx)
// Fields: Peers ([]Peer)
// Peer: NodeID, Address, Coordinate, Connected, Latency
```

### Topology

```go
// Get this node's position in the Block-MATRIX
topo, err := client.Topology.Info(ctx)
// Fields: NodeID, Coordinate (MatrixPosition), Dimensions, NodeCount

// List matrix neighbors
neighbors, err := client.Topology.Neighbors(ctx)
// Fields: Neighbors ([]Neighbor)
// Neighbor: NodeID, Coordinate, Distance
```

## Error Handling

API errors are returned as `*HyperMeshError`:

```go
import "errors"

status, err := client.Node.Status(ctx)
if err != nil {
    var apiErr *hypermesh.HyperMeshError
    if errors.As(err, &apiErr) {
        fmt.Printf("HTTP %d from %s: %s\n",
            apiErr.StatusCode, apiErr.Endpoint, apiErr.Message)

        if apiErr.IsNotFound() {
            fmt.Println("Resource not found")
        }
    } else {
        // Connection error or other failure
        fmt.Println("Error:", err)
    }
}
```

`HyperMeshError` fields:
- `StatusCode` (`int`) -- HTTP status code
- `Message` (`string`) -- error description
- `Endpoint` (`string`) -- the API path that failed

Helper methods:
- `IsNotFound()` -- returns true if status is 404
- `IsUnauthorized()` -- returns true if status is 401

## Context and Cancellation

All methods accept `context.Context` for timeout and cancellation:

```go
// With timeout
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

status, err := client.Node.Status(ctx)
```

## Types

Key types defined in `types.go`:

| Type | Fields |
|------|--------|
| `MatrixPosition` | `X`, `Y`, `Z` (float64) |
| `NodeStatus` | `NodeID`, `Uptime`, `Version`, `Coordinate`, `Networks`, `Status` |
| `Block` | `Index`, `Timestamp`, `PreviousHash`, `Hash`, `Data`, `StateProof` |
| `StateProof` | `PoSpace`, `PoStake`, `PoWork`, `PoTime` (each `*ProofEntry`) |
| `DnsRecord` | `Name`, `Address`, `TTL`, `NodeID` |
| `Asset` | `ID`, `Type`, `State`, `Owner`, `Metadata` |
| `Domain` | `Name`, `Privacy`, `Owner` |
| `Peer` | `NodeID`, `Address`, `Coordinate`, `Connected`, `Latency` |
