# HyperMesh Go SDK

Go client for the HyperMesh node HTTP REST API.

## Requirements

- Go 1.21+
- Zero external dependencies

## Install

```bash
go get github.com/hypermesh-online/sdk-go
```

## Usage

```go
package main

import (
    "context"
    "fmt"
    "log"

    hypermesh "github.com/hypermesh-online/sdk-go"
)

func main() {
    client := hypermesh.NewClient("http://localhost:9293")
    ctx := context.Background()

    // Node status
    status, err := client.Node.Status(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Node %s is %s\n", status.NodeID, status.Status)

    // Blockchain height
    height, err := client.Blockchain.Height(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Chain height: %d\n", height.Height)

    // DNS operations
    records, err := client.Dns.List(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("DNS records: %d\n", len(records.Records))

    err = client.Dns.Register(ctx, "my-service", "fd00::1")
    if err != nil {
        log.Fatal(err)
    }

    // Network peers
    peers, err := client.Network.Peers(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Connected peers: %d\n", len(peers.Peers))

    // Domain management
    err = client.Domain.Register(ctx, "my-domain", "Public")
    if err != nil {
        log.Fatal(err)
    }

    err = client.Domain.Join(ctx, "other-domain", "invite-token")
    if err != nil {
        log.Fatal(err)
    }
}
```

## API Groups

| Group | Methods |
|-------|---------|
| `client.Node` | `Status`, `Ping` |
| `client.Blockchain` | `Height`, `Block`, `Validate` |
| `client.Dns` | `List`, `Resolve`, `Register` |
| `client.Network` | `Peers` |
| `client.Topology` | `Info`, `Neighbors` |
| `client.Asset` | `List` |
| `client.Dashboard` | `List`, `Info` |
| `client.Config` | `Show`, `Get` |
| `client.Domain` | `List`, `Register`, `Join` |

## Error Handling

All methods return `*HyperMeshError` for non-2xx HTTP responses:

```go
status, err := client.Node.Status(ctx)
if err != nil {
    var hmErr *hypermesh.HyperMeshError
    if errors.As(err, &hmErr) {
        fmt.Printf("API error: %d %s\n", hmErr.StatusCode, hmErr.Message)
        if hmErr.IsNotFound() {
            // handle 404
        }
    }
}
```

## Options

```go
// Custom timeout
client := hypermesh.NewClient("http://localhost:9293",
    hypermesh.WithTimeout(10 * time.Second),
)

// Custom http.Client
client := hypermesh.NewClient("http://localhost:9293",
    hypermesh.WithHTTPClient(&http.Client{
        Timeout: 5 * time.Second,
        Transport: &http.Transport{
            MaxIdleConns: 10,
        },
    }),
)
```
