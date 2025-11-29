// Package service implements enhanced service discovery with associative intelligence
package service

import (
	"context"
	"fmt"
	"sort"
	"sync"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/graph"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/associative"
	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/routing"
)

// EnhancedServiceRegistry implements intelligent service discovery
type EnhancedServiceRegistry struct {
	// Core service storage
	services    map[string]*ServiceInstance
	servicesByNode map[int64][]*ServiceInstance
	
	// Graph integration
	networkGraph *graph.NetworkGraph
	
	// Associative learning for service affinity
	serviceAffinity *associative.AssociationMatrix
	
	// Routing integration
	routingTable *routing.RoutingTable
	
	// Performance optimization
	discoveryCache *DiscoveryCache
	healthMonitor  *HealthMonitor
	
	// Configuration
	config *RegistryConfig
	
	// Metrics
	metrics *DiscoveryMetrics
	
	// Thread safety
	mutex sync.RWMutex
}

// ServiceInstance represents a service instance with enhanced metadata
type ServiceInstance struct {
	ID          string
	Name        string
	Version     string
	NodeID      int64
	Address     string
	Port        int
	Protocol    string
	
	// Service characteristics
	ServiceType    string
	Capabilities   []string
	Dependencies   []string
	Tags          map[string]string
	Metadata      map[string]interface{}
	
	// Health and performance
	HealthStatus   HealthStatus
	HealthScore    float64
	ResponseTime   time.Duration
	ThroughputRPS  float64
	ErrorRate      float64
	
	// Discovery metadata
	RegisteredAt   time.Time
	LastHealthCheck time.Time
	LastAccessed   time.Time
	AccessCount    int64
	
	// Associative data
	AffinityScore  float64
	RelatedServices []string
}

// HealthStatus represents service health state
type HealthStatus int

const (
	HealthUnknown HealthStatus = iota
	HealthHealthy
	HealthDegraded
	HealthUnhealthy
	HealthCritical
)

// ServiceQuery defines parameters for service discovery
type ServiceQuery struct {
	ServiceName    string
	ServiceType    string
	Version        string
	RequiredTags   map[string]string
	Capabilities   []string
	
	// Location preferences
	PreferredRegions []string
	SourceNodeID     int64
	MaxDistance      float64
	
	// Quality requirements
	MinHealthScore   float64
	MaxResponseTime  time.Duration
	MinThroughput    float64
	
	// Discovery options
	IncludeDegraded  bool
	MaxResults       int
	SortBy          SortCriteria
	
	Context         context.Context
}

type SortCriteria int

const (
	SortByProximity SortCriteria = iota
	SortByHealth
	SortByPerformance
	SortByAffinity
	SortByLoad
)

// DiscoveryResult contains discovered services with ranking
type DiscoveryResult struct {
	Services      []*RankedService
	TotalFound    int
	QueryTime     time.Duration
	CacheHit      bool
	
	// Quality metrics
	AverageHealth    float64
	AverageLatency   time.Duration
	GeographicSpread float64
}

// RankedService represents a discovered service with ranking information
type RankedService struct {
	Service       *ServiceInstance
	Rank          int
	Score         float64
	Distance      float64
	RouteLatency  time.Duration
	
	// Scoring breakdown
	HealthScore    float64
	ProximityScore float64
	AffinityScore  float64
	PerformanceScore float64
	LoadScore      float64
	
	ReasonForRank string
}

// RegistryConfig configures the enhanced service registry
type RegistryConfig struct {
	// Cache settings
	CacheSize       int
	CacheTTL        time.Duration
	
	// Health monitoring
	HealthCheckInterval    time.Duration
	UnhealthyThreshold     float64
	DegradedThreshold      float64
	
	// Discovery optimization
	MaxDiscoveryTime       time.Duration
	AffinityLearningRate   float64
	ProximityWeight        float64
	HealthWeight           float64
	AffinityWeight         float64
	PerformanceWeight      float64
	
	// Cleanup
	StaleServiceTimeout    time.Duration
	CleanupInterval        time.Duration
}

// NewEnhancedServiceRegistry creates a new enhanced service registry
func NewEnhancedServiceRegistry(
	networkGraph *graph.NetworkGraph,
	routingTable *routing.RoutingTable,
	config *RegistryConfig,
) *EnhancedServiceRegistry {
	if config == nil {
		config = DefaultRegistryConfig()
	}
	
	registry := &EnhancedServiceRegistry{
		services:        make(map[string]*ServiceInstance),
		servicesByNode:  make(map[int64][]*ServiceInstance),
		networkGraph:    networkGraph,
		serviceAffinity: associative.NewAssociationMatrix(0.95, config.AffinityLearningRate),
		routingTable:    routingTable,
		discoveryCache:  NewDiscoveryCache(config.CacheSize, config.CacheTTL),
		healthMonitor:   NewHealthMonitor(config.HealthCheckInterval),
		config:         config,
		metrics:        NewDiscoveryMetrics(),
	}
	
	// Start background processes
	go registry.startHealthMonitoring()
	go registry.startCleanupProcess()
	
	return registry
}

// RegisterService registers a new service instance
func (esr *EnhancedServiceRegistry) RegisterService(service *ServiceInstance) error {
	esr.mutex.Lock()
	defer esr.mutex.Unlock()
	
	// Validate service
	if err := esr.validateService(service); err != nil {
		return fmt.Errorf("invalid service: %w", err)
	}
	
	// Set registration metadata
	service.RegisteredAt = time.Now()
	service.LastHealthCheck = time.Now()
	service.HealthStatus = HealthHealthy
	service.HealthScore = 1.0
	
	// Store service
	esr.services[service.ID] = service
	
	// Index by node
	if esr.servicesByNode[service.NodeID] == nil {
		esr.servicesByNode[service.NodeID] = make([]*ServiceInstance, 0)
	}
	esr.servicesByNode[service.NodeID] = append(esr.servicesByNode[service.NodeID], service)
	
	// Update service affinities
	esr.updateServiceAffinities(service)
	
	// Invalidate discovery cache
	esr.discoveryCache.InvalidateByServiceType(service.ServiceType)
	
	// Start health monitoring for this service
	esr.healthMonitor.AddService(service)
	
	esr.metrics.RecordRegistration(service)
	
	return nil
}

// DiscoverServices finds services matching the query criteria
func (esr *EnhancedServiceRegistry) DiscoverServices(query ServiceQuery) (*DiscoveryResult, error) {
	startTime := time.Now()
	
	// Check cache first
	cacheKey := esr.createCacheKey(query)
	if cached := esr.discoveryCache.Get(cacheKey); cached != nil {
		esr.metrics.RecordCacheHit()
		cached.CacheHit = true
		return cached, nil
	}
	
	esr.metrics.RecordCacheMiss()
	
	// Find candidate services
	candidates := esr.findCandidateServices(query)
	
	if len(candidates) == 0 {
		return &DiscoveryResult{
			Services:   []*RankedService{},
			TotalFound: 0,
			QueryTime:  time.Since(startTime),
			CacheHit:   false,
		}, nil
	}
	
	// Rank services using multi-criteria scoring
	rankedServices := esr.rankServices(candidates, query)
	
	// Apply sorting and limits
	esr.sortServices(rankedServices, query.SortBy)
	if query.MaxResults > 0 && len(rankedServices) > query.MaxResults {
		rankedServices = rankedServices[:query.MaxResults]
	}
	
	// Calculate result metrics
	result := &DiscoveryResult{
		Services:         rankedServices,
		TotalFound:       len(candidates),
		QueryTime:        time.Since(startTime),
		CacheHit:         false,
		AverageHealth:    esr.calculateAverageHealth(rankedServices),
		AverageLatency:   esr.calculateAverageLatency(rankedServices),
		GeographicSpread: esr.calculateGeographicSpread(rankedServices),
	}
	
	// Cache the result
	esr.discoveryCache.Put(cacheKey, result)
	
	// Update affinity learning based on query patterns
	esr.updateAffinityLearning(query, rankedServices)
	
	esr.metrics.RecordSuccessfulDiscovery(result)
	
	return result, nil
}

// findCandidateServices finds all services that match basic query criteria
func (esr *EnhancedServiceRegistry) findCandidateServices(query ServiceQuery) []*ServiceInstance {
	esr.mutex.RLock()
	defer esr.mutex.RUnlock()
	
	var candidates []*ServiceInstance
	
	for _, service := range esr.services {
		if esr.matchesBasicCriteria(service, query) {
			candidates = append(candidates, service)
		}
	}
	
	return candidates
}

// matchesBasicCriteria checks if a service matches basic query criteria
func (esr *EnhancedServiceRegistry) matchesBasicCriteria(service *ServiceInstance, query ServiceQuery) bool {
	// Service name match
	if query.ServiceName != "" && service.Name != query.ServiceName {
		return false
	}
	
	// Service type match
	if query.ServiceType != "" && service.ServiceType != query.ServiceType {
		return false
	}
	
	// Version match (semantic version matching could be added)
	if query.Version != "" && service.Version != query.Version {
		return false
	}
	
	// Health requirements
	if service.HealthScore < query.MinHealthScore {
		return false
	}
	
	if !query.IncludeDegraded && service.HealthStatus != HealthHealthy {
		return false
	}
	
	// Performance requirements
	if query.MaxResponseTime > 0 && service.ResponseTime > query.MaxResponseTime {
		return false
	}
	
	if query.MinThroughput > 0 && service.ThroughputRPS < query.MinThroughput {
		return false
	}
	
	// Required tags
	for key, value := range query.RequiredTags {
		if serviceValue, exists := service.Tags[key]; !exists || serviceValue != value {
			return false
		}
	}
	
	// Required capabilities
	for _, capability := range query.Capabilities {
		if !esr.hasCapability(service, capability) {
			return false
		}
	}
	
	return true
}

// rankServices applies multi-criteria ranking to candidate services
func (esr *EnhancedServiceRegistry) rankServices(candidates []*ServiceInstance, query ServiceQuery) []*RankedService {
	ranked := make([]*RankedService, 0, len(candidates))
	
	for _, service := range candidates {
		rankedService := &RankedService{
			Service: service,
		}
		
		// Calculate individual scores
		rankedService.HealthScore = esr.calculateHealthScore(service)
		rankedService.ProximityScore = esr.calculateProximityScore(service, query)
		rankedService.AffinityScore = esr.calculateAffinityScore(service, query)
		rankedService.PerformanceScore = esr.calculatePerformanceScore(service)
		rankedService.LoadScore = esr.calculateLoadScore(service)
		
		// Calculate distance and routing metrics
		if query.SourceNodeID > 0 {
			rankedService.Distance = esr.calculateDistance(service.NodeID, query.SourceNodeID)
			rankedService.RouteLatency = esr.calculateRouteLatency(service.NodeID, query.SourceNodeID)
		}
		
		// Calculate composite score
		rankedService.Score = esr.calculateCompositeScore(rankedService, query)
		
		// Generate ranking reason
		rankedService.ReasonForRank = esr.generateRankingReason(rankedService)
		
		ranked = append(ranked, rankedService)
	}
	
	return ranked
}

// calculateCompositeScore combines all scoring factors
func (esr *EnhancedServiceRegistry) calculateCompositeScore(rankedService *RankedService, query ServiceQuery) float64 {
	config := esr.config
	
	score := config.HealthWeight*rankedService.HealthScore +
		config.ProximityWeight*rankedService.ProximityScore +
		config.AffinityWeight*rankedService.AffinityScore +
		config.PerformanceWeight*rankedService.PerformanceScore +
		0.1*rankedService.LoadScore // Lower weight for load
	
	// Normalize to 0-1 range
	if score > 1.0 {
		score = 1.0
	}
	
	return score
}

// calculateProximityScore calculates geographic/network proximity score
func (esr *EnhancedServiceRegistry) calculateProximityScore(service *ServiceInstance, query ServiceQuery) float64 {
	if query.SourceNodeID <= 0 {
		return 0.5 // Neutral score when no source specified
	}
	
	// Get nodes from network graph
	sourceNode, sourceExists := esr.networkGraph.GetNode(query.SourceNodeID)
	targetNode, targetExists := esr.networkGraph.GetNode(service.NodeID)
	
	if !sourceExists || !targetExists {
		return 0.0
	}
	
	// Calculate geographic distance
	distance := graph.HaversineDistance(
		sourceNode.Latitude, sourceNode.Longitude,
		targetNode.Latitude, targetNode.Longitude,
	)
	
	// Convert distance to proximity score (closer = higher score)
	// Using exponential decay: score = e^(-distance/scale)
	const distanceScale = 1000.0 // 1000 km scale
	proximityScore := math.Exp(-distance / distanceScale)
	
	return proximityScore
}

// calculateAffinityScore calculates learned service affinity score
func (esr *EnhancedServiceRegistry) calculateAffinityScore(service *ServiceInstance, query ServiceQuery) float64 {
	if query.ServiceType == "" {
		return 0.5 // Neutral score when no service type context
	}
	
	// Get learned affinity between this service and the query context
	affinity := esr.serviceAffinity.GetServiceAffinity(service.NodeID, query.ServiceType)
	
	// Factor in service relationships
	relationshipScore := 0.0
	for _, relatedService := range service.RelatedServices {
		if relatedService == query.ServiceName || relatedService == query.ServiceType {
			relationshipScore = 0.8
			break
		}
	}
	
	// Combine affinity and relationship scores
	return math.Max(affinity, relationshipScore)
}

// sortServices sorts services based on the specified criteria
func (esr *EnhancedServiceRegistry) sortServices(services []*RankedService, sortBy SortCriteria) {
	switch sortBy {
	case SortByProximity:
		sort.Slice(services, func(i, j int) bool {
			return services[i].ProximityScore > services[j].ProximityScore
		})
	case SortByHealth:
		sort.Slice(services, func(i, j int) bool {
			return services[i].HealthScore > services[j].HealthScore
		})
	case SortByPerformance:
		sort.Slice(services, func(i, j int) bool {
			return services[i].PerformanceScore > services[j].PerformanceScore
		})
	case SortByAffinity:
		sort.Slice(services, func(i, j int) bool {
			return services[i].AffinityScore > services[j].AffinityScore
		})
	case SortByLoad:
		sort.Slice(services, func(i, j int) bool {
			return services[i].LoadScore > services[j].LoadScore
		})
	default: // SortByCompositeScore
		sort.Slice(services, func(i, j int) bool {
			return services[i].Score > services[j].Score
		})
	}
	
	// Assign ranks
	for i, service := range services {
		service.Rank = i + 1
	}
}

// UpdateServiceHealth updates health status for a service
func (esr *EnhancedServiceRegistry) UpdateServiceHealth(serviceID string, health HealthMetrics) error {
	esr.mutex.Lock()
	defer esr.mutex.Unlock()
	
	service, exists := esr.services[serviceID]
	if !exists {
		return fmt.Errorf("service %s not found", serviceID)
	}
	
	// Update health metrics
	service.HealthScore = health.Score
	service.ResponseTime = health.ResponseTime
	service.ThroughputRPS = health.ThroughputRPS
	service.ErrorRate = health.ErrorRate
	service.LastHealthCheck = time.Now()
	
	// Update health status based on thresholds
	if health.Score >= esr.config.DegradedThreshold {
		service.HealthStatus = HealthHealthy
	} else if health.Score >= esr.config.UnhealthyThreshold {
		service.HealthStatus = HealthDegraded
	} else {
		service.HealthStatus = HealthUnhealthy
	}
	
	// Invalidate discovery cache for this service type
	esr.discoveryCache.InvalidateByServiceType(service.ServiceType)
	
	return nil
}

// Helper methods and supporting types...

// HealthMetrics contains health check results
type HealthMetrics struct {
	Score         float64
	ResponseTime  time.Duration
	ThroughputRPS float64
	ErrorRate     float64
	Timestamp     time.Time
}

// DefaultRegistryConfig returns default configuration
func DefaultRegistryConfig() *RegistryConfig {
	return &RegistryConfig{
		CacheSize:              1000,
		CacheTTL:              2 * time.Minute,
		HealthCheckInterval:   30 * time.Second,
		UnhealthyThreshold:    0.3,
		DegradedThreshold:     0.7,
		MaxDiscoveryTime:      5 * time.Second,
		AffinityLearningRate:  0.1,
		ProximityWeight:       0.3,
		HealthWeight:         0.3,
		AffinityWeight:       0.2,
		PerformanceWeight:    0.2,
		StaleServiceTimeout:  10 * time.Minute,
		CleanupInterval:      5 * time.Minute,
	}
}

// Add missing import for math
import (
	"math"
)