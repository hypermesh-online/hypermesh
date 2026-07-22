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
	DefaultBaseURL = "https://localhost:8443"

	// DefaultCaesarURL is the default Caesar EVP service address (via Gateway).
	DefaultCaesarURL = "https://localhost:8443"

	// DefaultTrustChainURL is the default TrustChain service address (via Gateway).
	DefaultTrustChainURL = "https://localhost:8443"

	// DefaultCatalogURL is the default Catalog service address (via Gateway).
	DefaultCatalogURL = "https://localhost:8443"

	// DefaultNGaugeURL is the default NGauge service address (via Gateway).
	DefaultNGaugeURL = "https://localhost:8443"

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
	NGauge    *NGaugeApi
	Catalog    *CatalogApi

	// httpClients tracks every transport so SetCapabilityToken can
	// rotate the K.2 token across the whole client in one call.
	httpClients []*httpClient
}

// SetCapabilityToken installs (or rotates) the Phase K.2 capability
// token on every underlying transport. Pass an empty string to clear it.
func (c *Client) SetCapabilityToken(token string) {
	for _, h := range c.httpClients {
		h.SetCapabilityToken(token)
	}
}

// Option configures the Client.
type Option func(*clientConfig)

type clientConfig struct {
	baseURL       string
	caesarURL     string
	trustChainURL string
	catalogURL    string
	ngaugeURL    string
	httpClient    *http.Client
	// sessionToken — Phase K.2 capability token (base64 of serialized
	// CapabilityToken). When set, every request carries the
	// X-HyperMesh-Capability header.
	sessionToken string
}

// WithSessionToken — Phase K.2 — install a capability token at client
// construction time. Required when connecting to a daemon configured
// for token enforcement; ignored by alpha-default inert daemons.
func WithSessionToken(token string) Option {
	return func(cfg *clientConfig) {
		cfg.sessionToken = token
	}
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

// WithNGaugeURL sets a custom NGauge service URL.
func WithNGaugeURL(url string) Option {
	return func(cfg *clientConfig) {
		cfg.ngaugeURL = strings.TrimRight(url, "/")
	}
}

// NewClient creates a new HyperMesh SDK client.
//
// The baseURL should be the scheme + host + port of the node API
// (e.g. "https://localhost:8443"). If empty, DefaultBaseURL is used.
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
		ngaugeURL:    DefaultNGaugeURL,
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
	ngaugeH := newHTTPClient(cfg.ngaugeURL, cfg.httpClient)

	allTransports := []*httpClient{h, caesarH, trustChainH, catalogH, ngaugeH}
	// Phase K.2 — install the session token on every transport.
	if cfg.sessionToken != "" {
		for _, hc := range allTransports {
			hc.SetCapabilityToken(cfg.sessionToken)
		}
	}

	return &Client{
		Node:        &NodeApi{http: h},
		Blockchain:  &BlockchainApi{http: h},
		Dns:         &DnsApi{http: h},
		Network:     &NetworkApi{http: h},
		Topology:    &TopologyApi{http: h},
		Asset:       &AssetApi{http: h},
		Dashboard:   &DashboardApi{http: h},
		Config:      &ConfigApi{http: h},
		Domain:      &DomainApi{http: h},
		Caesar:      &CaesarApi{http: caesarH},
		TrustChain:  &TrustChainApi{http: trustChainH},
		NGauge:     &NGaugeApi{http: ngaugeH},
		Catalog:     &CatalogApi{http: catalogH},
		httpClients: allTransports,
	}
}
