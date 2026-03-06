// Blockchain operations with the HyperMesh Go SDK.
//
// Run: go run ./examples/blockchain/
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

	// Get chain height
	h, err := client.Blockchain.Height(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Blockchain height: %d\n", h.Height)

	// Fetch the genesis block
	genesis, err := client.Blockchain.Block(ctx, 0)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("\nGenesis block:\n")
	fmt.Printf("  Index: %d\n", genesis.Index)
	fmt.Printf("  Hash: %s\n", genesis.Hash)
	fmt.Printf("  Previous hash: %s\n", genesis.PreviousHash)
	fmt.Printf("  Timestamp: %d\n", genesis.Timestamp)

	// Show state proof if present
	if genesis.StateProof != nil {
		fmt.Printf("  State proof present: true\n")
		if genesis.StateProof.PoSpace != nil {
			fmt.Printf("    PoSpace valid: %v\n", genesis.StateProof.PoSpace.Valid)
		}
		if genesis.StateProof.PoStake != nil {
			fmt.Printf("    PoStake valid: %v\n", genesis.StateProof.PoStake.Valid)
		}
	}

	// Fetch the latest block
	if h.Height > 0 {
		latest, err := client.Blockchain.Block(ctx, h.Height-1)
		if err != nil {
			log.Fatal(err)
		}
		fmt.Printf("\nLatest block (index %d):\n", latest.Index)
		fmt.Printf("  Hash: %s\n", latest.Hash)
	}

	// Validate the chain
	result, err := client.Blockchain.Validate(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("\nBlockchain valid: %v\n", result.Valid)
	if len(result.Errors) > 0 {
		fmt.Printf("Errors:\n")
		for _, e := range result.Errors {
			fmt.Printf("  - %s\n", e)
		}
	}
}
