// Package hypermesh provides a Go SDK for the HyperMesh node HTTP REST API.
//
// All API methods accept a context.Context for cancellation and timeouts.
// The client uses only the standard library (net/http, encoding/json) with
// zero external dependencies.
package hypermesh

import (
	"net/http"
	"strings"
	"time"
)

const (
	// DefaultBaseURL is the default HyperMesh node API address.
	DefaultBaseURL = "http://localhost:9293"

	// DefaultTimeout is the default HTTP client timeout.
	DefaultTimeout = 30 * time.Second
)

// Client is the top-level HyperMesh SDK client. Access API groups through
// the exported fields (e.g. client.Node.Status, client.Blockchain.Height).
type Client struct {
	Node       *NodeApi
	Blockchain *BlockchainApi
	Dns        *DnsApi
	Network    *NetworkApi
	Topology   *TopologyApi
	Asset      *AssetApi
	Dashboard  *DashboardApi
	Config     *ConfigApi
	Domain     *DomainApi
}

// Option configures the Client.
type Option func(*clientConfig)

type clientConfig struct {
	baseURL    string
	httpClient *http.Client
}

// WithHTTPClient sets a custom http.Client for all requests.
func WithHTTPClient(c *http.Client) Option {
	return func(cfg *clientConfig) {
		cfg.httpClient = c
	}
}

// WithTimeout sets the HTTP client timeout. Ignored if WithHTTPClient is used.
func WithTimeout(d time.Duration) Option {
	return func(cfg *clientConfig) {
		if cfg.httpClient == nil {
			cfg.httpClient = &http.Client{Timeout: d}
		}
	}
}

// NewClient creates a new HyperMesh SDK client.
//
// The baseURL should be the scheme + host + port of the node API
// (e.g. "http://localhost:9293"). If empty, DefaultBaseURL is used.
func NewClient(baseURL string, opts ...Option) *Client {
	if baseURL == "" {
		baseURL = DefaultBaseURL
	}
	baseURL = strings.TrimRight(baseURL, "/")

	cfg := &clientConfig{baseURL: baseURL}
	for _, opt := range opts {
		opt(cfg)
	}

	if cfg.httpClient == nil {
		cfg.httpClient = &http.Client{Timeout: DefaultTimeout}
	}

	h := newHTTPClient(cfg.baseURL, cfg.httpClient)

	return &Client{
		Node:       &NodeApi{http: h},
		Blockchain: &BlockchainApi{http: h},
		Dns:        &DnsApi{http: h},
		Network:    &NetworkApi{http: h},
		Topology:   &TopologyApi{http: h},
		Asset:      &AssetApi{http: h},
		Dashboard:  &DashboardApi{http: h},
		Config:     &ConfigApi{http: h},
		Domain:     &DomainApi{http: h},
	}
}
