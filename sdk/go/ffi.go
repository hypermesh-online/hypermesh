//go:build cgo
// +build cgo

package hypermesh

/*
#cgo LDFLAGS: -lhypermesh_ffi -L../../target/release -L../../target/debug
#cgo CFLAGS: -I../../hypermesh-ffi/include

#include <stdlib.h>
#include <stdint.h>

// Opaque client handle.
typedef struct hypermesh_client_t hypermesh_client_t;

// Connection lifecycle.
extern hypermesh_client_t *hypermesh_connect(const char *socket_path);
extern void hypermesh_disconnect(hypermesh_client_t *client);

// Raw RPC.
extern char *hypermesh_call(hypermesh_client_t *client,
                            const char *method,
                            const char *params_json);

// Node.
extern char *hypermesh_status(hypermesh_client_t *client);

// DNS.
extern char *hypermesh_dns_resolve(hypermesh_client_t *client, const char *name);
extern char *hypermesh_dns_list(hypermesh_client_t *client);
extern char *hypermesh_dns_register(hypermesh_client_t *client,
                                    const char *name,
                                    const char *addr);

// Network.
extern char *hypermesh_peers(hypermesh_client_t *client);

// Blockchain.
extern char *hypermesh_blockchain_height(hypermesh_client_t *client);
extern char *hypermesh_blockchain_block(hypermesh_client_t *client, uint64_t index);

// Topology.
extern char *hypermesh_topology_info(hypermesh_client_t *client);

// Assets.
extern char *hypermesh_asset_list(hypermesh_client_t *client);
extern char *hypermesh_asset_store(hypermesh_client_t *client, const char *file_path);
extern char *hypermesh_asset_fetch(hypermesh_client_t *client,
                                   const char *asset_id,
                                   const char *output_path);

// Domains.
extern char *hypermesh_domain_list(hypermesh_client_t *client);
extern char *hypermesh_domain_register(hypermesh_client_t *client,
                                       const char *name,
                                       const char *privacy);

// Dashboards.
extern char *hypermesh_dashboard_list(hypermesh_client_t *client);
extern char *hypermesh_dashboard_deploy(hypermesh_client_t *client, const char *path);

// Config.
extern char *hypermesh_config_show(hypermesh_client_t *client);
extern char *hypermesh_config_get(hypermesh_client_t *client, const char *key);

// Caesar EVP.
extern char *hypermesh_caesar_wallet(hypermesh_client_t *client);
extern char *hypermesh_caesar_balance(hypermesh_client_t *client);
extern char *hypermesh_caesar_transactions(hypermesh_client_t *client, uint32_t limit);
extern char *hypermesh_caesar_rewards(hypermesh_client_t *client);
extern char *hypermesh_caesar_route_packet(hypermesh_client_t *client,
                                           const char *destination,
                                           double amount_grams);
extern char *hypermesh_caesar_governor_params(hypermesh_client_t *client);

// TrustChain.
extern char *hypermesh_trustchain_certificates(hypermesh_client_t *client);
extern char *hypermesh_trustchain_issue(hypermesh_client_t *client,
                                        const char *subject,
                                        const char *scope);
extern char *hypermesh_trustchain_validate(hypermesh_client_t *client,
                                           const char *cert_pem);
extern char *hypermesh_trustchain_revoke(hypermesh_client_t *client,
                                         const char *cert_id);
extern char *hypermesh_trustchain_dns_zones(hypermesh_client_t *client);

// NGauge.
extern char *hypermesh_ngauge_capacity(hypermesh_client_t *client);
extern char *hypermesh_ngauge_traffic(hypermesh_client_t *client);
extern char *hypermesh_ngauge_marketplace(hypermesh_client_t *client);
extern char *hypermesh_ngauge_node_metrics(hypermesh_client_t *client);
extern char *hypermesh_ngauge_leases(hypermesh_client_t *client);

// Catalog.
extern char *hypermesh_catalog_browse(hypermesh_client_t *client,
                                      const char *query,
                                      uint32_t page);
extern char *hypermesh_catalog_search(hypermesh_client_t *client,
                                      const char *query);
extern char *hypermesh_catalog_package_info(hypermesh_client_t *client,
                                            const char *name);
extern char *hypermesh_catalog_registry_stats(hypermesh_client_t *client);

// Memory management.
extern void hypermesh_free_string(char *s);
extern const char *hypermesh_last_error(const hypermesh_client_t *client);
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"runtime"
	"unsafe"
)

// FFIError represents an error returned by the HyperMesh FFI layer.
type FFIError struct {
	Message string
}

func (e *FFIError) Error() string {
	return fmt.Sprintf("hypermesh-ffi: %s", e.Message)
}

// FFIClient provides native access to the HyperMesh daemon via libhypermesh_ffi.
// It communicates over a Unix domain socket using the shared library, bypassing
// HTTP entirely. The zero-copy path avoids serialisation overhead for local use.
type FFIClient struct {
	handle *C.hypermesh_client_t
}

// NewFFIClient connects to the HyperMesh daemon via the FFI shared library.
// If socketPath is empty, the library uses its default 3-tier fallback
// ($HYPERMESH_SOCK / $XDG_RUNTIME_DIR / ~/.hypermesh).
func NewFFIClient(socketPath string) (*FFIClient, error) {
	var cPath *C.char
	if socketPath != "" {
		cPath = C.CString(socketPath)
		defer C.free(unsafe.Pointer(cPath))
	}

	handle := C.hypermesh_connect(cPath)
	if handle == nil {
		return nil, ffiLastError(nil)
	}

	client := &FFIClient{handle: handle}
	runtime.SetFinalizer(client, func(c *FFIClient) {
		c.Close()
	})
	return client, nil
}

// Close disconnects from the daemon and releases all FFI resources.
// Safe to call multiple times.
func (c *FFIClient) Close() {
	if c.handle != nil {
		C.hypermesh_disconnect(c.handle)
		c.handle = nil
	}
}

// Call sends a raw JSON-RPC method call to the daemon and returns the
// result as raw JSON. This is the escape hatch for methods not yet
// covered by the typed API.
func (c *FFIClient) Call(method, paramsJSON string) (json.RawMessage, error) {
	cMethod := C.CString(method)
	defer C.free(unsafe.Pointer(cMethod))
	cParams := C.CString(paramsJSON)
	defer C.free(unsafe.Pointer(cParams))

	return c.consumeResult(C.hypermesh_call(c.handle, cMethod, cParams))
}

// ── Node ────────────────────────────────────────────────────────────────

// Status returns the current node status.
func (c *FFIClient) Status() (*NodeStatus, error) {
	return decodeFFI[NodeStatus](c, C.hypermesh_status(c.handle))
}

// ── DNS ─────────────────────────────────────────────────────────────────

// DnsList returns all registered DNS entries.
func (c *FFIClient) DnsList() (*DnsList, error) {
	return decodeFFI[DnsList](c, C.hypermesh_dns_list(c.handle))
}

// DnsResolve resolves a DNS name to its record.
func (c *FFIClient) DnsResolve(name string) (*DnsRecord, error) {
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	return decodeFFI[DnsRecord](c, C.hypermesh_dns_resolve(c.handle, cName))
}

// DnsRegister registers a DNS name pointing to the given address.
func (c *FFIClient) DnsRegister(name, addr string) (json.RawMessage, error) {
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	cAddr := C.CString(addr)
	defer C.free(unsafe.Pointer(cAddr))

	return c.consumeResult(C.hypermesh_dns_register(c.handle, cName, cAddr))
}

// ── Network ─────────────────────────────────────────────────────────────

// Peers returns the list of connected peers.
func (c *FFIClient) Peers() (*PeerList, error) {
	return decodeFFI[PeerList](c, C.hypermesh_peers(c.handle))
}

// ── Blockchain ──────────────────────────────────────────────────────────

// BlockchainHeight returns the current blockchain height.
func (c *FFIClient) BlockchainHeight() (*BlockchainHeight, error) {
	return decodeFFI[BlockchainHeight](c, C.hypermesh_blockchain_height(c.handle))
}

// BlockchainBlock returns the block at the given index.
func (c *FFIClient) BlockchainBlock(index uint64) (*Block, error) {
	return decodeFFI[Block](c, C.hypermesh_blockchain_block(c.handle, C.uint64_t(index)))
}

// ── Topology ────────────────────────────────────────────────────────────

// TopologyInfo returns the node's topology information.
func (c *FFIClient) TopologyInfo() (*TopologyInfo, error) {
	return decodeFFI[TopologyInfo](c, C.hypermesh_topology_info(c.handle))
}

// ── Assets ──────────────────────────────────────────────────────────────

// AssetList returns all stored assets.
func (c *FFIClient) AssetList() (*AssetList, error) {
	return decodeFFI[AssetList](c, C.hypermesh_asset_list(c.handle))
}

// AssetStore stores a file as a HyperMesh asset and returns the result
// (including the new asset_id).
func (c *FFIClient) AssetStore(filePath string) (json.RawMessage, error) {
	cPath := C.CString(filePath)
	defer C.free(unsafe.Pointer(cPath))
	return c.consumeResult(C.hypermesh_asset_store(c.handle, cPath))
}

// AssetFetch fetches an asset by ID and writes it to outputPath.
func (c *FFIClient) AssetFetch(assetID, outputPath string) error {
	cID := C.CString(assetID)
	defer C.free(unsafe.Pointer(cID))
	cOut := C.CString(outputPath)
	defer C.free(unsafe.Pointer(cOut))

	_, err := c.consumeResult(C.hypermesh_asset_fetch(c.handle, cID, cOut))
	return err
}

// ── Domains ─────────────────────────────────────────────────────────────

// DomainList returns all registered domains.
func (c *FFIClient) DomainList() (*DomainList, error) {
	return decodeFFI[DomainList](c, C.hypermesh_domain_list(c.handle))
}

// DomainRegister registers a domain with the given name and privacy mode.
func (c *FFIClient) DomainRegister(name, privacy string) (json.RawMessage, error) {
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	cPrivacy := C.CString(privacy)
	defer C.free(unsafe.Pointer(cPrivacy))

	return c.consumeResult(C.hypermesh_domain_register(c.handle, cName, cPrivacy))
}

// ── Dashboards ──────────────────────────────────────────────────────────

// DashboardList returns all deployed dashboards.
func (c *FFIClient) DashboardList() (*DashboardList, error) {
	return decodeFFI[DashboardList](c, C.hypermesh_dashboard_list(c.handle))
}

// DashboardDeploy deploys a dashboard from the given filesystem path.
func (c *FFIClient) DashboardDeploy(path string) (json.RawMessage, error) {
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))
	return c.consumeResult(C.hypermesh_dashboard_deploy(c.handle, cPath))
}

// ── Config ──────────────────────────────────────────────────────────────

// ConfigShow returns the full daemon configuration.
func (c *FFIClient) ConfigShow() (json.RawMessage, error) {
	return c.consumeResult(C.hypermesh_config_show(c.handle))
}

// ConfigGet returns a single configuration value by key.
func (c *FFIClient) ConfigGet(key string) (*ConfigValue, error) {
	cKey := C.CString(key)
	defer C.free(unsafe.Pointer(cKey))
	return decodeFFI[ConfigValue](c, C.hypermesh_config_get(c.handle, cKey))
}

// ── Caesar EVP ──────────────────────────────────────────────────────────

// CaesarWallet returns the caller's Caesar wallet info.
func (c *FFIClient) CaesarWallet() (*CaesarWalletInfo, error) {
	return decodeFFI[CaesarWalletInfo](c, C.hypermesh_caesar_wallet(c.handle))
}

// CaesarBalance returns the current Caesar balance.
func (c *FFIClient) CaesarBalance() (*CaesarBalance, error) {
	return decodeFFI[CaesarBalance](c, C.hypermesh_caesar_balance(c.handle))
}

// CaesarTransactions returns recent Caesar transactions. Pass 0 for default limit.
func (c *FFIClient) CaesarTransactions(limit uint32) (*CaesarTransactionList, error) {
	return decodeFFI[CaesarTransactionList](c, C.hypermesh_caesar_transactions(c.handle, C.uint32_t(limit)))
}

// CaesarRewards returns accumulated Caesar reward info.
func (c *FFIClient) CaesarRewards() (*CaesarRewardInfo, error) {
	return decodeFFI[CaesarRewardInfo](c, C.hypermesh_caesar_rewards(c.handle))
}

// CaesarRoutePacket routes an EVP packet to the given destination.
func (c *FFIClient) CaesarRoutePacket(destination string, amountGrams float64) (*CaesarRouteResult, error) {
	cDest := C.CString(destination)
	defer C.free(unsafe.Pointer(cDest))
	return decodeFFI[CaesarRouteResult](c, C.hypermesh_caesar_route_packet(c.handle, cDest, C.double(amountGrams)))
}

// CaesarGovernorParams returns the current Caesar Governor parameters.
func (c *FFIClient) CaesarGovernorParams() (*CaesarGovernorParams, error) {
	return decodeFFI[CaesarGovernorParams](c, C.hypermesh_caesar_governor_params(c.handle))
}

// ── TrustChain ──────────────────────────────────────────────────────────

// TrustChainCerts returns all TrustChain certificates.
func (c *FFIClient) TrustChainCerts() (*TrustChainCertificateList, error) {
	return decodeFFI[TrustChainCertificateList](c, C.hypermesh_trustchain_certificates(c.handle))
}

// TrustChainIssue issues a new certificate for the given subject and scope.
func (c *FFIClient) TrustChainIssue(subject, scope string) (*TrustChainCertificate, error) {
	cSubject := C.CString(subject)
	defer C.free(unsafe.Pointer(cSubject))
	cScope := C.CString(scope)
	defer C.free(unsafe.Pointer(cScope))

	return decodeFFI[TrustChainCertificate](c, C.hypermesh_trustchain_issue(c.handle, cSubject, cScope))
}

// TrustChainValidate validates a PEM-encoded certificate.
func (c *FFIClient) TrustChainValidate(certPem string) (*TrustChainValidationResult, error) {
	cPem := C.CString(certPem)
	defer C.free(unsafe.Pointer(cPem))
	return decodeFFI[TrustChainValidationResult](c, C.hypermesh_trustchain_validate(c.handle, cPem))
}

// TrustChainRevoke revokes the certificate with the given ID.
func (c *FFIClient) TrustChainRevoke(certID string) (*TrustChainRevokeResult, error) {
	cID := C.CString(certID)
	defer C.free(unsafe.Pointer(cID))
	return decodeFFI[TrustChainRevokeResult](c, C.hypermesh_trustchain_revoke(c.handle, cID))
}

// TrustChainDnsZones returns all TrustChain DNS zones.
func (c *FFIClient) TrustChainDnsZones() (*TrustChainDnsZoneList, error) {
	return decodeFFI[TrustChainDnsZoneList](c, C.hypermesh_trustchain_dns_zones(c.handle))
}

// ── NGauge ─────────────────────────────────────────────────────────────

// NGaugeCapacity returns current node capacity metrics.
func (c *FFIClient) NGaugeCapacity() (*NGaugeCapacityMetrics, error) {
	return decodeFFI[NGaugeCapacityMetrics](c, C.hypermesh_ngauge_capacity(c.handle))
}

// NGaugeTraffic returns current traffic statistics.
func (c *FFIClient) NGaugeTraffic() (*NGaugeTrafficMetrics, error) {
	return decodeFFI[NGaugeTrafficMetrics](c, C.hypermesh_ngauge_traffic(c.handle))
}

// NGaugeMarketplace returns marketplace resource pool info.
func (c *FFIClient) NGaugeMarketplace() (*NGaugeListingList, error) {
	return decodeFFI[NGaugeListingList](c, C.hypermesh_ngauge_marketplace(c.handle))
}

// NGaugeNodeMetrics returns detailed node-level metrics.
func (c *FFIClient) NGaugeNodeMetrics() (*NGaugeNodeMetrics, error) {
	return decodeFFI[NGaugeNodeMetrics](c, C.hypermesh_ngauge_node_metrics(c.handle))
}

// NGaugeLeases returns active resource leases.
func (c *FFIClient) NGaugeLeases() (*NGaugeLeaseList, error) {
	return decodeFFI[NGaugeLeaseList](c, C.hypermesh_ngauge_leases(c.handle))
}

// ── Catalog ─────────────────────────────────────────────────────────────

// CatalogBrowse browses catalog packages. Pass empty query to list all.
func (c *FFIClient) CatalogBrowse(query string, page uint32) (*CatalogPackageList, error) {
	var cQuery *C.char
	if query != "" {
		cQuery = C.CString(query)
		defer C.free(unsafe.Pointer(cQuery))
	}
	return decodeFFI[CatalogPackageList](c, C.hypermesh_catalog_browse(c.handle, cQuery, C.uint32_t(page)))
}

// CatalogSearch searches catalog packages by query string.
func (c *FFIClient) CatalogSearch(query string) (*CatalogSearchResults, error) {
	cQuery := C.CString(query)
	defer C.free(unsafe.Pointer(cQuery))
	return decodeFFI[CatalogSearchResults](c, C.hypermesh_catalog_search(c.handle, cQuery))
}

// CatalogPackageInfo returns detailed info about a specific catalog package.
func (c *FFIClient) CatalogPackageInfo(name string) (*CatalogPackageInfo, error) {
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	return decodeFFI[CatalogPackageInfo](c, C.hypermesh_catalog_package_info(c.handle, cName))
}

// CatalogRegistryStats returns aggregate catalog registry statistics.
func (c *FFIClient) CatalogRegistryStats() (*CatalogRegistryStats, error) {
	return decodeFFI[CatalogRegistryStats](c, C.hypermesh_catalog_registry_stats(c.handle))
}

// ── Internal helpers ────────────────────────────────────────────────────

// ffiLastError retrieves the thread-local error from the FFI library.
// The returned pointer must NOT be freed (it is owned by the library).
func ffiLastError(handle *C.hypermesh_client_t) error {
	cErr := C.hypermesh_last_error(handle)
	if cErr == nil {
		return &FFIError{Message: "unknown FFI error"}
	}
	return &FFIError{Message: C.GoString(cErr)}
}

// consumeResult takes ownership of a C string returned by the FFI library,
// converts it to a Go json.RawMessage, and frees the C memory. Returns an
// error sourced from hypermesh_last_error when the pointer is nil.
func (c *FFIClient) consumeResult(cStr *C.char) (json.RawMessage, error) {
	if cStr == nil {
		return nil, ffiLastError(c.handle)
	}
	goStr := C.GoString(cStr)
	C.hypermesh_free_string(cStr)
	return json.RawMessage(goStr), nil
}

// decodeFFI is a generic helper that consumes a C string, deserialises the
// JSON into the requested Go type, and frees the C memory.
func decodeFFI[T any](c *FFIClient, cStr *C.char) (*T, error) {
	raw, err := c.consumeResult(cStr)
	if err != nil {
		return nil, err
	}
	var result T
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, fmt.Errorf("hypermesh-ffi: failed to decode response: %w", err)
	}
	return &result, nil
}
