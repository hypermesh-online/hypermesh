package hypermesh

// NodeStatus represents the response from GET /api/v1/status.
type NodeStatus struct {
	NodeID     string          `json:"node_id"`
	Uptime     uint64          `json:"uptime"`
	Version    string          `json:"version"`
	Coordinate MatrixPosition  `json:"coordinate"`
	Networks   []string        `json:"networks"`
	Status     string          `json:"status"`
}

// MatrixPosition represents a node's position in the Block-MATRIX topology.
type MatrixPosition struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
	Z float64 `json:"z"`
}

// PingResponse represents the response from GET /api/v1/ping.
type PingResponse struct {
	Pong   bool   `json:"pong"`
	NodeID string `json:"node_id"`
}

// BlockchainHeight represents the response from GET /api/v1/blockchain/height.
type BlockchainHeight struct {
	Height uint64 `json:"height"`
}

// Block represents a single block in the blockchain.
type Block struct {
	Index        uint64            `json:"index"`
	Timestamp    uint64            `json:"timestamp"`
	PreviousHash string            `json:"previous_hash"`
	Hash         string            `json:"hash"`
	Data         map[string]any    `json:"data"`
	StateProof   *StateProof       `json:"state_proof,omitempty"`
}

// StateProof contains the four proofs required by HyperMesh protocol.
type StateProof struct {
	PoSpace *ProofEntry `json:"po_space,omitempty"`
	PoStake *ProofEntry `json:"po_stake,omitempty"`
	PoWork  *ProofEntry `json:"po_work,omitempty"`
	PoTime  *ProofEntry `json:"po_time,omitempty"`
}

// ProofEntry represents a single proof within a StateProof.
type ProofEntry struct {
	Valid     bool   `json:"valid"`
	Timestamp uint64 `json:"timestamp,omitempty"`
	Data      string `json:"data,omitempty"`
}

// ValidationResult represents the response from GET /api/v1/blockchain/validate.
type ValidationResult struct {
	Valid  bool   `json:"valid"`
	Errors []string `json:"errors,omitempty"`
}

// DnsList represents the response from GET /api/v1/dns/list.
type DnsList struct {
	Records []DnsRecord `json:"records"`
}

// DnsRecord represents a single DNS record.
type DnsRecord struct {
	Name    string `json:"name"`
	Address string `json:"address"`
	TTL     uint32 `json:"ttl,omitempty"`
	NodeID  string `json:"node_id,omitempty"`
}

// DnsRegisterRequest is the body for POST /api/v1/dns/register.
type DnsRegisterRequest struct {
	Name    string `json:"name"`
	Address string `json:"address"`
}

// PeerList represents the response from GET /api/v1/network/peers.
type PeerList struct {
	Peers []Peer `json:"peers"`
}

// Peer represents a connected peer node.
type Peer struct {
	NodeID     string         `json:"node_id"`
	Address    string         `json:"address"`
	Coordinate MatrixPosition `json:"coordinate"`
	Connected  bool           `json:"connected"`
	Latency    uint64         `json:"latency,omitempty"`
}

// TopologyInfo represents the response from GET /api/v1/topology/info.
type TopologyInfo struct {
	NodeID     string         `json:"node_id"`
	Coordinate MatrixPosition `json:"coordinate"`
	Dimensions []int          `json:"dimensions,omitempty"`
	NodeCount  uint64         `json:"node_count"`
}

// NeighborList represents the response from GET /api/v1/topology/neighbors.
type NeighborList struct {
	Neighbors []Neighbor `json:"neighbors"`
}

// Neighbor represents a neighbor node in the matrix topology.
type Neighbor struct {
	NodeID     string         `json:"node_id"`
	Coordinate MatrixPosition `json:"coordinate"`
	Distance   float64        `json:"distance"`
}

// AssetList represents the response from GET /api/v1/asset/list.
type AssetList struct {
	Assets []Asset `json:"assets"`
}

// Asset represents a registered asset in the Block-MATRIX.
type Asset struct {
	ID       string         `json:"id"`
	Type     string         `json:"type"`
	State    string         `json:"state"`
	Owner    string         `json:"owner,omitempty"`
	Metadata map[string]any `json:"metadata,omitempty"`
}

// DashboardList represents the response from GET /api/v1/dashboard/list.
type DashboardList struct {
	Dashboards []DashboardEntry `json:"dashboards"`
}

// DashboardEntry represents a dashboard manifest entry.
type DashboardEntry struct {
	ID   string `json:"id"`
	Name string `json:"name"`
	Path string `json:"path"`
}

// DashboardInfo represents the response from GET /api/v1/dashboard/info.
type DashboardInfo struct {
	ID      string `json:"id"`
	Name    string `json:"name"`
	Version string `json:"version"`
}

// ConfigValue represents the response from GET /api/v1/config/get/:key.
type ConfigValue struct {
	Key   string `json:"key"`
	Value any    `json:"value"`
}

// DomainList represents the response from GET /api/v1/domain/list.
type DomainList struct {
	Domains []Domain `json:"domains"`
}

// Domain represents a registered domain (domain-as-network).
type Domain struct {
	Name    string `json:"name"`
	Privacy string `json:"privacy"`
	Owner   string `json:"owner,omitempty"`
}

// DomainRegisterRequest is the body for POST /api/v1/domain/register.
type DomainRegisterRequest struct {
	Name    string `json:"name"`
	Privacy string `json:"privacy"`
}

// DomainJoinRequest is the body for POST /api/v1/domain/join.
type DomainJoinRequest struct {
	Name  string `json:"name"`
	Token string `json:"token,omitempty"`
}

// ── Caesar ──

// CaesarWalletInfo represents the response from GET /api/v1/caesar/wallet.
type CaesarWalletInfo struct {
	BalanceGrams float64 `json:"balance_grams"`
	BalanceUsd   float64 `json:"balance_usd"`
	Tier         string  `json:"tier"`
	NodeID       string  `json:"node_id"`
}

// CaesarBalance represents the response from GET /api/v1/caesar/balance.
type CaesarBalance struct {
	GoldGrams     float64 `json:"gold_grams"`
	UsdEquivalent float64 `json:"usd_equivalent"`
	Tier          string  `json:"tier"`
}

// CaesarTransaction represents a single transaction.
type CaesarTransaction struct {
	ID          string  `json:"id"`
	From        string  `json:"from"`
	To          string  `json:"to"`
	AmountGrams float64 `json:"amount_grams"`
	Fee         float64 `json:"fee"`
	Status      string  `json:"status"`
	Timestamp   uint64  `json:"timestamp"`
}

// CaesarTransactionList represents the response from GET /api/v1/caesar/transactions.
type CaesarTransactionList struct {
	Count        int                  `json:"count"`
	Transactions []CaesarTransaction  `json:"transactions"`
}

// CaesarRewardInfo represents the response from GET /api/v1/caesar/rewards.
type CaesarRewardInfo struct {
	TotalEarned    float64 `json:"total_earned"`
	Pending        float64 `json:"pending"`
	TierMultiplier float64 `json:"tier_multiplier"`
}

// CaesarRouteRequest is the body for POST /api/v1/caesar/route.
type CaesarRouteRequest struct {
	Destination string  `json:"destination"`
	AmountGrams float64 `json:"amount_grams"`
}

// CaesarRouteResult represents the response from POST /api/v1/caesar/route.
type CaesarRouteResult struct {
	PacketID string  `json:"packet_id"`
	Status   string  `json:"status"`
	Fee      float64 `json:"fee"`
}

// CaesarGovernorParams represents the response from GET /api/v1/caesar/governor/params.
type CaesarGovernorParams struct {
	Velocity      float64 `json:"velocity"`
	FeeRate       float64 `json:"fee_rate"`
	DemurrageRate float64 `json:"demurrage_rate"`
}

// ── TrustChain ──

// TrustChainCertificate represents a TrustChain certificate.
type TrustChainCertificate struct {
	ID        string `json:"id"`
	Subject   string `json:"subject"`
	Scope     string `json:"scope"`
	ValidFrom string `json:"valid_from"`
	ValidTo   string `json:"valid_to"`
	Pem       string `json:"pem"`
}

// TrustChainCertificateList represents the response from GET /api/v1/trustchain/certificates.
type TrustChainCertificateList struct {
	Count        int                     `json:"count"`
	Certificates []TrustChainCertificate `json:"certificates"`
}

// TrustChainIssueRequest is the body for POST /api/v1/trustchain/issue.
type TrustChainIssueRequest struct {
	Subject string `json:"subject"`
	Scope   string `json:"scope"`
}

// TrustChainValidateRequest is the body for POST /api/v1/trustchain/validate.
type TrustChainValidateRequest struct {
	CertPem string `json:"cert_pem"`
}

// TrustChainValidationResult represents the response from POST /api/v1/trustchain/validate.
type TrustChainValidationResult struct {
	Valid      bool     `json:"valid"`
	Errors     []string `json:"errors,omitempty"`
	ChainValid bool     `json:"chain_valid"`
}

// TrustChainRevokeRequest is the body for POST /api/v1/trustchain/revoke.
type TrustChainRevokeRequest struct {
	CertID string `json:"cert_id"`
}

// TrustChainRevokeResult represents the response from POST /api/v1/trustchain/revoke.
type TrustChainRevokeResult struct {
	Revoked bool   `json:"revoked"`
	CertID  string `json:"cert_id"`
}

// TrustChainDnsZone represents a DNS zone.
type TrustChainDnsZone struct {
	Name    string `json:"name"`
	Records int    `json:"records"`
}

// TrustChainDnsZoneList represents the response from GET /api/v1/trustchain/dns/zones.
type TrustChainDnsZoneList struct {
	Count int                 `json:"count"`
	Zones []TrustChainDnsZone `json:"zones"`
}

// ── Engauge ──

// EngaugeCapacityMetrics represents the response from GET /api/v1/engauge/capacity.
type EngaugeCapacityMetrics struct {
	BytesServed      uint64  `json:"bytes_served"`
	ComputeDelivered float64 `json:"compute_delivered"`
	Storage          uint64  `json:"storage"`
	Bandwidth        float64 `json:"bandwidth"`
	Uptime           float64 `json:"uptime"`
}

// EngaugeTrafficMetrics represents the response from GET /api/v1/engauge/traffic.
type EngaugeTrafficMetrics struct {
	OrganicRatio     float64 `json:"organic_ratio"`
	SpeculativeRatio float64 `json:"speculative_ratio"`
	TotalRequests    uint64  `json:"total_requests"`
}

// EngaugeListing represents a marketplace listing.
type EngaugeListing struct {
	ID           string  `json:"id"`
	ResourceType string  `json:"resource_type"`
	Price        float64 `json:"price"`
}

// EngaugeListingList represents the response from GET /api/v1/engauge/marketplace/listings.
type EngaugeListingList struct {
	Count    int              `json:"count"`
	Listings []EngaugeListing `json:"listings"`
}

// EngaugeNodeMetrics represents the response from GET /api/v1/engauge/node/metrics.
type EngaugeNodeMetrics struct {
	ActivityScore float64 `json:"activity_score"`
	Receipts      uint64  `json:"receipts"`
	Bandwidth     float64 `json:"bandwidth"`
}

// EngaugeLease represents a resource lease.
type EngaugeLease struct {
	ID           string `json:"id"`
	ResourceType string `json:"resource_type"`
	Status       string `json:"status"`
}

// EngaugeLeaseList represents the response from GET /api/v1/engauge/leases.
type EngaugeLeaseList struct {
	Count  int            `json:"count"`
	Leases []EngaugeLease `json:"leases"`
}

// ── Catalog ──

// CatalogPackage represents a catalog package.
type CatalogPackage struct {
	Name        string `json:"name"`
	Version     string `json:"version"`
	Description string `json:"description"`
	Author      string `json:"author"`
	Downloads   uint64 `json:"downloads"`
}

// CatalogPackageList represents the response from GET /api/v1/catalog/browse.
type CatalogPackageList struct {
	Count    int              `json:"count"`
	Packages []CatalogPackage `json:"packages"`
}

// CatalogSearchResult represents a single search result.
type CatalogSearchResult struct {
	Name        string  `json:"name"`
	Version     string  `json:"version"`
	Description string  `json:"description"`
	Relevance   float64 `json:"relevance"`
}

// CatalogSearchResults represents the response from GET /api/v1/catalog/search.
type CatalogSearchResults struct {
	Count   int                   `json:"count"`
	Results []CatalogSearchResult `json:"results"`
}

// CatalogPackageInfo represents the response from GET /api/v1/catalog/package/:name.
type CatalogPackageInfo struct {
	Name        string `json:"name"`
	Version     string `json:"version"`
	Description string `json:"description"`
	Author      string `json:"author"`
	Downloads   uint64 `json:"downloads"`
}

// CatalogRegistryStats represents the response from GET /api/v1/catalog/registry/stats.
type CatalogRegistryStats struct {
	PackageCount   int    `json:"package_count"`
	PublisherCount int    `json:"publisher_count"`
	TotalDownloads uint64 `json:"total_downloads"`
}
