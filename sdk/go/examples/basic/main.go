// Basic usage of the HyperMesh Go SDK.
//
// Run: go run ./examples/basic/
package main

import (
	"context"
	"errors"
	"fmt"
	"log"

	hypermesh "github.com/hypermesh-online/sdk-go"
)

func main() {
	client := hypermesh.NewClient("") // http://localhost:9293
	ctx := context.Background()

	// Ping the node
	pong, err := client.Node.Ping(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Ping: %v (node: %s)\n", pong.Pong, pong.NodeID)

	// Get node status
	status, err := client.Node.Status(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Node ID: %s\n", status.NodeID)
	fmt.Printf("Status: %s\n", status.Status)
	fmt.Printf("Version: %s\n", status.Version)
	fmt.Printf("Uptime: %d seconds\n", status.Uptime)
	fmt.Printf("Coordinate: (%.1f, %.1f, %.1f)\n",
		status.Coordinate.X, status.Coordinate.Y, status.Coordinate.Z)
	fmt.Printf("Networks: %v\n", status.Networks)

	// List connected peers
	peers, err := client.Network.Peers(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("\nConnected peers: %d\n", len(peers.Peers))
	for _, p := range peers.Peers {
		fmt.Printf("  %s @ %s (connected: %v)\n", p.NodeID, p.Address, p.Connected)
	}

	// Error handling
	_, err = client.Blockchain.Block(ctx, 999999)
	if err != nil {
		var apiErr *hypermesh.HyperMeshError
		if errors.As(err, &apiErr) {
			fmt.Printf("\nExpected error for block 999999:\n")
			fmt.Printf("  Status: %d\n", apiErr.StatusCode)
			fmt.Printf("  Message: %s\n", apiErr.Message)
			fmt.Printf("  Not found: %v\n", apiErr.IsNotFound())
		}
	}
}
