package hypermesh

import (
	"context"
	"fmt"
)

// DnsApi provides access to DNS endpoints.
type DnsApi struct {
	http *httpClient
}

// List returns all registered DNS records.
func (a *DnsApi) List(ctx context.Context) (*DnsList, error) {
	var result DnsList
	if err := a.http.get(ctx, "/api/v1/dns/list", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Resolve resolves a DNS name to its record.
func (a *DnsApi) Resolve(ctx context.Context, name string) (*DnsRecord, error) {
	var result DnsRecord
	path := fmt.Sprintf("/api/v1/dns/resolve/%s", name)
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Register registers a new DNS name with the given address.
func (a *DnsApi) Register(ctx context.Context, name, address string) error {
	body := DnsRegisterRequest{
		Name:    name,
		Address: address,
	}
	return a.http.post(ctx, "/api/v1/dns/register", body, nil)
}
