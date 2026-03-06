// DNS operations with the HyperMesh Go SDK.
//
// Run: go run ./examples/dns/
package main

import (
	"context"
	"errors"
	"fmt"
	"log"

	hypermesh "github.com/hypermesh-online/sdk-go"
)

func main() {
	client := hypermesh.NewClient("")
	ctx := context.Background()

	// List existing DNS records
	dns, err := client.Dns.List(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("DNS records: %d\n", len(dns.Records))
	for _, r := range dns.Records {
		fmt.Printf("  %s -> %s\n", r.Name, r.Address)
	}

	// Register a new record
	fmt.Printf("\nRegistering example.hypermesh -> ::1\n")
	err = client.Dns.Register(ctx, "example.hypermesh", "::1")
	if err != nil {
		var apiErr *hypermesh.HyperMeshError
		if errors.As(err, &apiErr) {
			fmt.Printf("Register failed (HTTP %d): %s\n", apiErr.StatusCode, apiErr.Message)
		} else {
			log.Fatal(err)
		}
	} else {
		fmt.Println("Registered successfully")
	}

	// Resolve the record
	record, err := client.Dns.Resolve(ctx, "example.hypermesh")
	if err != nil {
		fmt.Printf("Resolve failed: %v\n", err)
	} else {
		fmt.Printf("Resolved: %s -> %s\n", record.Name, record.Address)
	}

	// List records after registration
	updated, err := client.Dns.List(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("\nDNS records after registration: %d\n", len(updated.Records))
	for _, r := range updated.Records {
		fmt.Printf("  %s -> %s\n", r.Name, r.Address)
	}
}
