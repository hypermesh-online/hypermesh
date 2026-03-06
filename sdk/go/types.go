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
