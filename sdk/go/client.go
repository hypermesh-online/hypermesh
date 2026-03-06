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

	// DefaultCaesarURL is the default Caesar EVP service address.
	DefaultCaesarURL = "http://localhost:9294"

	// DefaultTrustChainURL is the default TrustChain service address.
	DefaultTrustChainURL = "http://localhost:8444"

	// DefaultCatalogURL is the default Catalog service address.
	DefaultCatalogURL = "http://localhost:9295"

	// DefaultEngaugeURL is the default Engauge service address.
	DefaultEngaugeURL = "http://localhost:9296"

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
	Caesar     *CaesarApi
	TrustChain *TrustChainApi
	Engauge    *EngaugeApi
	Catalog    *CatalogApi
}

// Option configures the Client.
type Option func(*clientConfig)

type clientConfig struct {
	baseURL       string
	caesarURL     string
	trustChainURL string
	catalogURL    string
	engaugeURL    string
	httpClient    *http.Client
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

// WithCaesarURL sets a custom Caesar EVP service URL.
func WithCaesarURL(url string) Option {
	return func(cfg *clientConfig) {
		cfg.caesarURL = strings.TrimRight(url, "/")
	}
}

// WithTrustChainURL sets a custom TrustChain service URL.
func WithTrustChainURL(url string) Option {
	return func(cfg *clientConfig) {
		cfg.trustChainURL = strings.TrimRight(url, "/")
	}
}

// WithCatalogURL sets a custom Catalog service URL.
func WithCatalogURL(url string) Option {
	return func(cfg *clientConfig) {
		cfg.catalogURL = strings.TrimRight(url, "/")
	}
}

// WithEngaugeURL sets a custom Engauge service URL.
func WithEngaugeURL(url string) Option {
	return func(cfg *clientConfig) {
		cfg.engaugeURL = strings.TrimRight(url, "/")
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

	cfg := &clientConfig{
		baseURL:       baseURL,
		caesarURL:     DefaultCaesarURL,
		trustChainURL: DefaultTrustChainURL,
		catalogURL:    DefaultCatalogURL,
		engaugeURL:    DefaultEngaugeURL,
	}
	for _, opt := range opts {
		opt(cfg)
	}

	if cfg.httpClient == nil {
		cfg.httpClient = &http.Client{Timeout: DefaultTimeout}
	}

	h := newHTTPClient(cfg.baseURL, cfg.httpClient)
	caesarH := newHTTPClient(cfg.caesarURL, cfg.httpClient)
	trustChainH := newHTTPClient(cfg.trustChainURL, cfg.httpClient)
	catalogH := newHTTPClient(cfg.catalogURL, cfg.httpClient)
	engaugeH := newHTTPClient(cfg.engaugeURL, cfg.httpClient)

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
		Caesar:     &CaesarApi{http: caesarH},
		TrustChain: &TrustChainApi{http: trustChainH},
		Engauge:    &EngaugeApi{http: engaugeH},
		Catalog:    &CatalogApi{http: catalogH},
	}
}
