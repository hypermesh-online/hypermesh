// Domain, dashboard, config, asset, and topology operations.
//
// Run: go run ./examples/domain/
package main

import (
	"context"
	"fmt"
	"log"

	hypermesh "github.com/hypermesh-online/sdk-go"
)

func main() {
	client := hypermesh.NewClient("")
	ctx := context.Background()

	// List registered domains
	domains, err := client.Domain.List(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Domains: %d\n", len(domains.Domains))
	for _, d := range domains.Domains {
		fmt.Printf("  %s (privacy: %s, owner: %s)\n", d.Name, d.Privacy, d.Owner)
	}

	// Register a new domain
	fmt.Printf("\nRegistering domain 'testapp' with Private privacy...\n")
	err = client.Domain.Register(ctx, "testapp", "Private")
	if err != nil {
		fmt.Printf("Register failed: %v\n", err)
	} else {
		fmt.Println("Registered successfully")
	}

	// Join a domain
	fmt.Printf("\nJoining domain 'testapp'...\n")
	err = client.Domain.Join(ctx, "testapp", "")
	if err != nil {
		fmt.Printf("Join failed: %v\n", err)
	} else {
		fmt.Println("Joined successfully")
	}

	// Dashboard info
	info, err := client.Dashboard.Info(ctx)
	if err != nil {
		fmt.Printf("Dashboard info error: %v\n", err)
	} else {
		fmt.Printf("\nDashboard: %s v%s\n", info.Name, info.Version)
	}

	// List dashboards
	dashboards, err := client.Dashboard.List(ctx)
	if err != nil {
		fmt.Printf("Dashboard list error: %v\n", err)
	} else {
		fmt.Printf("Dashboards: %d\n", len(dashboards.Dashboards))
		for _, d := range dashboards.Dashboards {
			fmt.Printf("  %s (path: %s)\n", d.Name, d.Path)
		}
	}

	// Config
	config, err := client.Config.Show(ctx)
	if err != nil {
		fmt.Printf("Config error: %v\n", err)
	} else {
		fmt.Printf("\nConfig keys: %d\n", len(config))
	}

	val, err := client.Config.Get(ctx, "privacy_mode")
	if err != nil {
		fmt.Printf("Config get error: %v\n", err)
	} else {
		fmt.Printf("privacy_mode: %v\n", val.Value)
	}

	// Assets
	assets, err := client.Asset.List(ctx)
	if err != nil {
		fmt.Printf("Asset list error: %v\n", err)
	} else {
		fmt.Printf("\nAssets: %d\n", len(assets.Assets))
		for _, a := range assets.Assets {
			fmt.Printf("  [%s] %s (state: %s)\n", a.Type, a.ID, a.State)
		}
	}

	// Topology
	topo, err := client.Topology.Info(ctx)
	if err != nil {
		fmt.Printf("Topology error: %v\n", err)
	} else {
		fmt.Printf("\nTopology:\n")
		fmt.Printf("  Node: %s\n", topo.NodeID)
		fmt.Printf("  Position: (%.1f, %.1f, %.1f)\n",
			topo.Coordinate.X, topo.Coordinate.Y, topo.Coordinate.Z)
		fmt.Printf("  Node count: %d\n", topo.NodeCount)
	}

	neighbors, err := client.Topology.Neighbors(ctx)
	if err != nil {
		fmt.Printf("Neighbors error: %v\n", err)
	} else {
		fmt.Printf("  Neighbors: %d\n", len(neighbors.Neighbors))
		for _, n := range neighbors.Neighbors {
			fmt.Printf("    %s distance=%.2f\n", n.NodeID, n.Distance)
		}
	}
}
