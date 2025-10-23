// Package graph implements high-performance graph data structures
// for network topology representation and associative search algorithms
package graph

import (
	"fmt"
	"math"
	"sort"
	"sync"
	"time"

	"gonum.org/v1/gonum/graph"
	"gonum.org/v1/gonum/graph/path"
	"gonum.org/v1/gonum/graph/simple"
)

// NetworkNode represents a node in the network graph with performance metrics
type NetworkNode struct {
	ID         int64
	Address    string
	Region     string
	Latitude   float64
	Longitude  float64
	
	// Performance characteristics
	Latency       time.Duration
	Throughput    float64  // MB/s
	Reliability   float64  // 0.0-1.0
	LoadFactor    float64  // 0.0-1.0
	LastSeen      time.Time
	
	// Service information
	Services      map[string]ServiceInfo
	Capabilities  []string
	
	mutex sync.RWMutex
}

// ServiceInfo represents service details hosted on a node
type ServiceInfo struct {
	Name        string
	Version     string
	Port        int
	Protocol    string
	HealthScore float64
	Endpoints   []string
}

// NetworkEdge represents a connection between two nodes with weighted metrics
type NetworkEdge struct {
	From   int64
	To     int64
	Weight float64
	
	// Performance metrics
	Latency     time.Duration
	Bandwidth   float64  // MB/s
	PacketLoss  float64  // 0.0-1.0
	Jitter      time.Duration
	Cost        float64
	
	// Quality metrics
	Reliability float64
	Stability   float64
	LastUpdate  time.Time
}

// NetworkGraph implements a high-performance graph for network topology
type NetworkGraph struct {
	graph       *simple.WeightedDirectedGraph
	nodes       map[int64]*NetworkNode
	edges       map[int64]map[int64]*NetworkEdge
	
	// Spatial indexing for geographic queries
	spatialIndex *SpatialIndex
	
	// Performance optimization
	pathCache    *PathCache
	updateChan   chan GraphUpdate
	
	// Thread safety
	mutex        sync.RWMutex
	
	// Metrics
	totalNodes   int64
	totalEdges   int64
	lastUpdate   time.Time
}

// GraphUpdate represents a topology change
type GraphUpdate struct {
	Type     UpdateType
	NodeID   int64
	EdgeFrom int64
	EdgeTo   int64
	Node     *NetworkNode
	Edge     *NetworkEdge
}

type UpdateType int

const (
	NodeAdd UpdateType = iota
	NodeRemove
	NodeUpdate
	EdgeAdd
	EdgeRemove
	EdgeUpdate
)

// NewNetworkGraph creates a new high-performance network graph
func NewNetworkGraph(capacity int) *NetworkGraph {
	ng := &NetworkGraph{
		graph:        simple.NewWeightedDirectedGraph(0, math.Inf(1)),
		nodes:        make(map[int64]*NetworkNode, capacity),
		edges:        make(map[int64]map[int64]*NetworkEdge),
		spatialIndex: NewSpatialIndex(),
		pathCache:    NewPathCache(1000), // Cache 1000 paths
		updateChan:   make(chan GraphUpdate, 100),
	}
	
	// Start update processor
	go ng.processUpdates()
	
	return ng
}

// AddNode adds a new node to the network graph
func (ng *NetworkGraph) AddNode(node *NetworkNode) error {
	ng.mutex.Lock()
	defer ng.mutex.Unlock()
	
	if _, exists := ng.nodes[node.ID]; exists {
		return fmt.Errorf("node %d already exists", node.ID)
	}
	
	// Add to gonum graph
	ng.graph.AddNode(simple.Node(node.ID))
	
	// Store node
	ng.nodes[node.ID] = node
	ng.edges[node.ID] = make(map[int64]*NetworkEdge)
	
	// Add to spatial index
	ng.spatialIndex.AddNode(node.ID, node.Latitude, node.Longitude)
	
	ng.totalNodes++
	ng.lastUpdate = time.Now()
	
	// Send update notification
	select {
	case ng.updateChan <- GraphUpdate{Type: NodeAdd, NodeID: node.ID, Node: node}:
	default:
		// Channel full, update lost (non-critical)
	}
	
	return nil
}

// AddEdge adds a weighted edge between two nodes
func (ng *NetworkGraph) AddEdge(edge *NetworkEdge) error {
	ng.mutex.Lock()
	defer ng.mutex.Unlock()
	
	// Verify nodes exist
	if _, exists := ng.nodes[edge.From]; !exists {
		return fmt.Errorf("source node %d does not exist", edge.From)
	}
	if _, exists := ng.nodes[edge.To]; !exists {
		return fmt.Errorf("destination node %d does not exist", edge.To)
	}
	
	// Add to gonum graph
	gnEdge := ng.graph.NewWeightedEdge(simple.Node(edge.From), simple.Node(edge.To), edge.Weight)
	ng.graph.SetWeightedEdge(gnEdge)
	
	// Store edge
	ng.edges[edge.From][edge.To] = edge
	
	ng.totalEdges++
	ng.lastUpdate = time.Now()
	
	// Invalidate affected cached paths
	ng.pathCache.InvalidateNode(edge.From)
	ng.pathCache.InvalidateNode(edge.To)
	
	// Send update notification
	select {
	case ng.updateChan <- GraphUpdate{Type: EdgeAdd, EdgeFrom: edge.From, EdgeTo: edge.To, Edge: edge}:
	default:
	}
	
	return nil
}

// GetNode retrieves a node by ID
func (ng *NetworkGraph) GetNode(id int64) (*NetworkNode, bool) {
	ng.mutex.RLock()
	defer ng.mutex.RUnlock()
	
	node, exists := ng.nodes[id]
	return node, exists
}

// GetEdge retrieves an edge between two nodes
func (ng *NetworkGraph) GetEdge(from, to int64) (*NetworkEdge, bool) {
	ng.mutex.RLock()
	defer ng.mutex.RUnlock()
	
	if edges, exists := ng.edges[from]; exists {
		edge, exists := edges[to]
		return edge, exists
	}
	return nil, false
}

// FindNearestNodes returns nodes within a geographic radius
func (ng *NetworkGraph) FindNearestNodes(lat, lng, radiusKm float64, maxNodes int) []*NetworkNode {
	ng.mutex.RLock()
	defer ng.mutex.RUnlock()
	
	nodeIDs := ng.spatialIndex.FindNearest(lat, lng, radiusKm, maxNodes)
	
	nodes := make([]*NetworkNode, 0, len(nodeIDs))
	for _, id := range nodeIDs {
		if node, exists := ng.nodes[id]; exists {
			nodes = append(nodes, node)
		}
	}
	
	return nodes
}

// FindOptimalPath uses multi-objective optimization to find the best path
// FindShortestPath finds the shortest path between two nodes using default preferences
func (ng *NetworkGraph) FindShortestPath(from, to int64) (*OptimalPath, error) {
	preferences := PathPreferences{
		LatencyWeight:    1.0,
		ThroughputWeight: 0.0,
		ReliabilityWeight: 0.0,
		CostWeight:       0.0,
	}
	return ng.FindOptimalPath(from, to, preferences)
}

func (ng *NetworkGraph) FindOptimalPath(from, to int64, preferences PathPreferences) (*OptimalPath, error) {
	ng.mutex.RLock()
	defer ng.mutex.RUnlock()
	
	// Check cache first
	if path := ng.pathCache.Get(from, to, preferences); path != nil {
		return path, nil
	}
	
	// Use weighted shortest path with custom weight function
	shortest := path.DijkstraFrom(simple.Node(from), ng.graph)
	
	pathNodes, _ := shortest.To(to)
	if len(pathNodes) == 0 {
		return nil, fmt.Errorf("no path found from %d to %d", from, to)
	}
	
	// Calculate detailed path metrics
	optimized := ng.calculatePathMetrics(pathNodes, preferences)
	
	// Cache the result
	ng.pathCache.Put(from, to, preferences, optimized)
	
	return optimized, nil
}

// FindMultiPath returns multiple alternative paths with different optimization criteria
func (ng *NetworkGraph) FindMultiPath(from, to int64, maxPaths int) ([]*OptimalPath, error) {
	ng.mutex.RLock()
	defer ng.mutex.RUnlock()
	
	paths := make([]*OptimalPath, 0, maxPaths)
	
	// Find paths optimized for different criteria
	preferences := []PathPreferences{
		{LatencyWeight: 1.0, ThroughputWeight: 0.0, ReliabilityWeight: 0.0},
		{LatencyWeight: 0.0, ThroughputWeight: 1.0, ReliabilityWeight: 0.0},
		{LatencyWeight: 0.0, ThroughputWeight: 0.0, ReliabilityWeight: 1.0},
		{LatencyWeight: 0.4, ThroughputWeight: 0.4, ReliabilityWeight: 0.2},
		{LatencyWeight: 0.2, ThroughputWeight: 0.2, ReliabilityWeight: 0.6},
	}
	
	for i, pref := range preferences {
		if i >= maxPaths {
			break
		}
		
		if path, err := ng.FindOptimalPath(from, to, pref); err == nil {
			paths = append(paths, path)
		}
	}
	
	// Sort by composite score
	sort.Slice(paths, func(i, j int) bool {
		return paths[i].CompositeScore > paths[j].CompositeScore
	})
	
	return paths, nil
}

// UpdateNodeMetrics updates performance metrics for a node
func (ng *NetworkGraph) UpdateNodeMetrics(nodeID int64, metrics NodeMetrics) error {
	ng.mutex.Lock()
	defer ng.mutex.Unlock()
	
	node, exists := ng.nodes[nodeID]
	if !exists {
		return fmt.Errorf("node %d not found", nodeID)
	}
	
	node.mutex.Lock()
	node.Latency = metrics.Latency
	node.Throughput = metrics.Throughput
	node.Reliability = metrics.Reliability
	node.LoadFactor = metrics.LoadFactor
	node.LastSeen = time.Now()
	node.mutex.Unlock()
	
	// Invalidate cached paths involving this node
	ng.pathCache.InvalidateNode(nodeID)
	
	return nil
}

// GetTopologyStats returns current graph statistics
func (ng *NetworkGraph) GetTopologyStats() TopologyStats {
	ng.mutex.RLock()
	defer ng.mutex.RUnlock()
	
	return TopologyStats{
		TotalNodes:   ng.totalNodes,
		TotalEdges:   ng.totalEdges,
		LastUpdate:   ng.lastUpdate,
		CacheHitRate: ng.pathCache.GetHitRate(),
	}
}

// processUpdates handles graph update notifications in background
func (ng *NetworkGraph) processUpdates() {
	for update := range ng.updateChan {
		// Process topology change notifications
		// This can trigger recomputation of cached paths,
		// load balancing decisions, etc.
		
		switch update.Type {
		case NodeAdd, NodeRemove:
			// Trigger topology adaptation
		case EdgeAdd, EdgeRemove, EdgeUpdate:
			// Trigger routing table updates
		}
	}
}

// calculatePathMetrics computes detailed metrics for a path
func (ng *NetworkGraph) calculatePathMetrics(pathNodes []graph.Node, preferences PathPreferences) *OptimalPath {
	if len(pathNodes) < 2 {
		return nil
	}
	
	var totalLatency time.Duration
	var minThroughput float64 = math.Inf(1)
	var avgReliability float64
	var totalCost float64
	hopCount := len(pathNodes) - 1
	
	nodeIDs := make([]int64, len(pathNodes))
	for i, node := range pathNodes {
		nodeIDs[i] = node.ID()
	}
	
	// Calculate path metrics
	for i := 0; i < len(pathNodes)-1; i++ {
		fromID := pathNodes[i].ID()
		toID := pathNodes[i+1].ID()
		
		if edge, exists := ng.edges[fromID][toID]; exists {
			totalLatency += edge.Latency
			if edge.Bandwidth < minThroughput {
				minThroughput = edge.Bandwidth
			}
			avgReliability += edge.Reliability
			totalCost += edge.Cost
		}
	}
	
	avgReliability /= float64(hopCount)
	
	// Calculate composite score based on preferences
	latencyScore := 1.0 / (float64(totalLatency.Microseconds()) + 1.0)
	throughputScore := minThroughput / 1000.0 // Normalize to Gbps
	reliabilityScore := avgReliability
	
	compositeScore := preferences.LatencyWeight*latencyScore +
		preferences.ThroughputWeight*throughputScore +
		preferences.ReliabilityWeight*reliabilityScore
	
	return &OptimalPath{
		NodeIDs:        nodeIDs,
		TotalLatency:   totalLatency,
		MinThroughput:  minThroughput,
		AvgReliability: avgReliability,
		TotalCost:      totalCost,
		HopCount:       hopCount,
		CompositeScore: compositeScore,
		CreatedAt:      time.Now(),
	}
}

// PathPreferences defines optimization criteria for path finding
type PathPreferences struct {
	LatencyWeight    float64
	ThroughputWeight float64
	ReliabilityWeight float64
	CostWeight       float64
}

// OptimalPath represents an optimized path through the network
type OptimalPath struct {
	NodeIDs        []int64
	TotalLatency   time.Duration
	MinThroughput  float64
	AvgReliability float64
	TotalCost      float64
	HopCount       int
	CompositeScore float64
	CreatedAt      time.Time
}

// NodeMetrics contains performance metrics for a node
type NodeMetrics struct {
	Latency     time.Duration
	Throughput  float64
	Reliability float64
	LoadFactor  float64
}

// TopologyStats provides graph statistics
type TopologyStats struct {
	TotalNodes   int64
	TotalEdges   int64
	LastUpdate   time.Time
	CacheHitRate float64
}