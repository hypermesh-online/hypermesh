// Package graph implements spatial indexing for geographic network queries
package graph

import (
	"math"
	"sort"
	"sync"
)

const (
	EarthRadiusKm = 6371.0
)

// SpatialIndex implements a spatial index for fast geographic queries
type SpatialIndex struct {
	// QuadTree for efficient spatial partitioning
	root *QuadNode
	
	// Node cache for direct access
	nodes map[int64]*SpatialNode
	
	mutex sync.RWMutex
}

// SpatialNode represents a node in the spatial index
type SpatialNode struct {
	ID        int64
	Latitude  float64
	Longitude float64
}

// QuadNode represents a node in the quadtree
type QuadNode struct {
	// Boundaries
	MinLat, MaxLat float64
	MinLng, MaxLng float64
	
	// Nodes in this quad (if leaf)
	Nodes []*SpatialNode
	
	// Children (if not leaf)
	NW, NE, SW, SE *QuadNode
	
	// Configuration
	MaxNodes int
	MaxDepth int
	Depth    int
}

// NewSpatialIndex creates a new spatial index
func NewSpatialIndex() *SpatialIndex {
	return &SpatialIndex{
		root: &QuadNode{
			MinLat:   -90.0,
			MaxLat:   90.0,
			MinLng:   -180.0,
			MaxLng:   180.0,
			Nodes:    make([]*SpatialNode, 0),
			MaxNodes: 10,
			MaxDepth: 8,
			Depth:    0,
		},
		nodes: make(map[int64]*SpatialNode),
	}
}

// AddNode adds a node to the spatial index
func (si *SpatialIndex) AddNode(id int64, lat, lng float64) {
	si.mutex.Lock()
	defer si.mutex.Unlock()
	
	node := &SpatialNode{
		ID:        id,
		Latitude:  lat,
		Longitude: lng,
	}
	
	si.nodes[id] = node
	si.root.Insert(node)
}

// RemoveNode removes a node from the spatial index
func (si *SpatialIndex) RemoveNode(id int64) bool {
	si.mutex.Lock()
	defer si.mutex.Unlock()
	
	node, exists := si.nodes[id]
	if !exists {
		return false
	}
	
	delete(si.nodes, id)
	return si.root.Remove(node)
}

// FindNearest finds nodes within a radius, sorted by distance
func (si *SpatialIndex) FindNearest(lat, lng, radiusKm float64, maxResults int) []int64 {
	si.mutex.RLock()
	defer si.mutex.RUnlock()
	
	candidates := si.root.Query(lat, lng, radiusKm)
	
	// Calculate distances and filter
	type NodeDistance struct {
		ID       int64
		Distance float64
	}
	
	distances := make([]NodeDistance, 0, len(candidates))
	
	for _, node := range candidates {
		distance := HaversineDistance(lat, lng, node.Latitude, node.Longitude)
		if distance <= radiusKm {
			distances = append(distances, NodeDistance{
				ID:       node.ID,
				Distance: distance,
			})
		}
	}
	
	// Sort by distance
	sort.Slice(distances, func(i, j int) bool {
		return distances[i].Distance < distances[j].Distance
	})
	
	// Limit results
	if len(distances) > maxResults {
		distances = distances[:maxResults]
	}
	
	// Extract IDs
	results := make([]int64, len(distances))
	for i, d := range distances {
		results[i] = d.ID
	}
	
	return results
}

// Insert adds a node to the quadtree
func (qn *QuadNode) Insert(node *SpatialNode) bool {
	// Check if node is within bounds
	if !qn.Contains(node.Latitude, node.Longitude) {
		return false
	}
	
	// If we have children, delegate to appropriate child
	if qn.HasChildren() {
		return qn.insertIntoChild(node)
	}
	
	// Add to this node
	qn.Nodes = append(qn.Nodes, node)
	
	// Check if we need to subdivide
	if len(qn.Nodes) > qn.MaxNodes && qn.Depth < qn.MaxDepth {
		qn.Subdivide()
		
		// Redistribute nodes to children
		nodes := qn.Nodes
		qn.Nodes = nil
		
		for _, n := range nodes {
			qn.insertIntoChild(n)
		}
	}
	
	return true
}

// Remove removes a node from the quadtree
func (qn *QuadNode) Remove(node *SpatialNode) bool {
	if !qn.Contains(node.Latitude, node.Longitude) {
		return false
	}
	
	if qn.HasChildren() {
		return qn.removeFromChild(node)
	}
	
	// Remove from this node
	for i, n := range qn.Nodes {
		if n.ID == node.ID {
			qn.Nodes = append(qn.Nodes[:i], qn.Nodes[i+1:]...)
			return true
		}
	}
	
	return false
}

// Query finds all nodes within a radius
func (qn *QuadNode) Query(lat, lng, radiusKm float64) []*SpatialNode {
	// Check if query circle intersects with quad bounds
	if !qn.IntersectsCircle(lat, lng, radiusKm) {
		return nil
	}
	
	var results []*SpatialNode
	
	if qn.HasChildren() {
		// Query children
		for _, child := range []*QuadNode{qn.NW, qn.NE, qn.SW, qn.SE} {
			if child != nil {
				childResults := child.Query(lat, lng, radiusKm)
				results = append(results, childResults...)
			}
		}
	} else {
		// Return nodes from this quad
		results = append(results, qn.Nodes...)
	}
	
	return results
}

// Contains checks if a point is within the quad bounds
func (qn *QuadNode) Contains(lat, lng float64) bool {
	return lat >= qn.MinLat && lat <= qn.MaxLat &&
		lng >= qn.MinLng && lng <= qn.MaxLng
}

// HasChildren returns true if the node has been subdivided
func (qn *QuadNode) HasChildren() bool {
	return qn.NW != nil
}

// Subdivide splits the quad into four children
func (qn *QuadNode) Subdivide() {
	midLat := (qn.MinLat + qn.MaxLat) / 2
	midLng := (qn.MinLng + qn.MaxLng) / 2
	
	// Northwest
	qn.NW = &QuadNode{
		MinLat:   midLat,
		MaxLat:   qn.MaxLat,
		MinLng:   qn.MinLng,
		MaxLng:   midLng,
		Nodes:    make([]*SpatialNode, 0),
		MaxNodes: qn.MaxNodes,
		MaxDepth: qn.MaxDepth,
		Depth:    qn.Depth + 1,
	}
	
	// Northeast
	qn.NE = &QuadNode{
		MinLat:   midLat,
		MaxLat:   qn.MaxLat,
		MinLng:   midLng,
		MaxLng:   qn.MaxLng,
		Nodes:    make([]*SpatialNode, 0),
		MaxNodes: qn.MaxNodes,
		MaxDepth: qn.MaxDepth,
		Depth:    qn.Depth + 1,
	}
	
	// Southwest
	qn.SW = &QuadNode{
		MinLat:   qn.MinLat,
		MaxLat:   midLat,
		MinLng:   qn.MinLng,
		MaxLng:   midLng,
		Nodes:    make([]*SpatialNode, 0),
		MaxNodes: qn.MaxNodes,
		MaxDepth: qn.MaxDepth,
		Depth:    qn.Depth + 1,
	}
	
	// Southeast
	qn.SE = &QuadNode{
		MinLat:   qn.MinLat,
		MaxLat:   midLat,
		MinLng:   midLng,
		MaxLng:   qn.MaxLng,
		Nodes:    make([]*SpatialNode, 0),
		MaxNodes: qn.MaxNodes,
		MaxDepth: qn.MaxDepth,
		Depth:    qn.Depth + 1,
	}
}

// insertIntoChild delegates insertion to appropriate child
func (qn *QuadNode) insertIntoChild(node *SpatialNode) bool {
	for _, child := range []*QuadNode{qn.NW, qn.NE, qn.SW, qn.SE} {
		if child != nil && child.Insert(node) {
			return true
		}
	}
	return false
}

// removeFromChild delegates removal to appropriate child
func (qn *QuadNode) removeFromChild(node *SpatialNode) bool {
	for _, child := range []*QuadNode{qn.NW, qn.NE, qn.SW, qn.SE} {
		if child != nil && child.Remove(node) {
			return true
		}
	}
	return false
}

// IntersectsCircle checks if a circle intersects with the quad bounds
func (qn *QuadNode) IntersectsCircle(centerLat, centerLng, radiusKm float64) bool {
	// Convert radius to degrees (approximate)
	radiusDegLat := radiusKm / 111.0 // 1 degree lat ≈ 111 km
	radiusDegLng := radiusKm / (111.0 * math.Cos(centerLat*math.Pi/180.0))
	
	// Check if circle intersects quad
	return !(centerLat - radiusDegLat > qn.MaxLat ||
		centerLat + radiusDegLat < qn.MinLat ||
		centerLng - radiusDegLng > qn.MaxLng ||
		centerLng + radiusDegLng < qn.MinLng)
}

// HaversineDistance calculates the great circle distance between two points
func HaversineDistance(lat1, lng1, lat2, lng2 float64) float64 {
	// Convert to radians
	lat1Rad := lat1 * math.Pi / 180.0
	lng1Rad := lng1 * math.Pi / 180.0
	lat2Rad := lat2 * math.Pi / 180.0
	lng2Rad := lng2 * math.Pi / 180.0
	
	// Differences
	dlat := lat2Rad - lat1Rad
	dlng := lng2Rad - lng1Rad
	
	// Haversine formula
	a := math.Sin(dlat/2)*math.Sin(dlat/2) +
		math.Cos(lat1Rad)*math.Cos(lat2Rad)*
			math.Sin(dlng/2)*math.Sin(dlng/2)
	
	c := 2 * math.Asin(math.Sqrt(a))
	
	return EarthRadiusKm * c
}