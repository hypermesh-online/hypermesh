// Package associative implements association matrices for learning network relationships
package associative

import (
	"fmt"
	"math"
	"sort"
	"time"
)

// NewAssociationMatrix creates a new association matrix
func NewAssociationMatrix(decayRate, learningRate float64) *AssociationMatrix {
	return &AssociationMatrix{
		weights:      make(map[AssociationKey]float64),
		lastUpdate:   make(map[AssociationKey]time.Time),
		decayRate:    decayRate,
		learningRate: learningRate,
	}
}

// GetAssociation retrieves the association strength between two entities
func (am *AssociationMatrix) GetAssociation(from, to int64, assocType AssociationType) *Association {
	am.mutex.RLock()
	defer am.mutex.RUnlock()
	
	key := AssociationKey{From: from, To: to, Type: assocType}
	
	if weight, exists := am.weights[key]; exists {
		// Apply temporal decay
		lastUpdate := am.lastUpdate[key]
		decayFactor := am.calculateDecay(lastUpdate)
		actualWeight := weight * decayFactor
		
		return &Association{
			From:       from,
			To:         to,
			Type:       assocType,
			Strength:   actualWeight,
			Confidence: am.calculateConfidence(actualWeight, lastUpdate),
		}
	}
	
	return nil
}

// UpdateAssociation updates the strength of an association using reinforcement learning
func (am *AssociationMatrix) UpdateAssociation(from, to int64, assocType AssociationType, reward float64) {
	am.mutex.Lock()
	defer am.mutex.Unlock()
	
	key := AssociationKey{From: from, To: to, Type: assocType}
	now := time.Now()
	
	// Get current weight with decay applied
	currentWeight := 0.0
	if weight, exists := am.weights[key]; exists {
		lastUpdate := am.lastUpdate[key]
		decayFactor := am.calculateDecay(lastUpdate)
		currentWeight = weight * decayFactor
	}
	
	// Apply reinforcement learning update
	// Q(s,a) = Q(s,a) + α * [reward + γ * max(Q(s',a')) - Q(s,a)]
	newWeight := currentWeight + am.learningRate*(reward-currentWeight)
	
	// Clamp weight to [0, 1] range
	if newWeight < 0 {
		newWeight = 0
	} else if newWeight > 1 {
		newWeight = 1
	}
	
	am.weights[key] = newWeight
	am.lastUpdate[key] = now
}

// GetStrongestAssociations returns the strongest associations from a node
func (am *AssociationMatrix) GetStrongestAssociations(from int64, limit int) []Association {
	am.mutex.RLock()
	defer am.mutex.RUnlock()
	
	var associations []Association
	
	// Collect all associations from this node
	for key, weight := range am.weights {
		if key.From == from {
			lastUpdate := am.lastUpdate[key]
			decayFactor := am.calculateDecay(lastUpdate)
			actualWeight := weight * decayFactor
			
			if actualWeight > 0.01 { // Threshold to filter weak associations
				associations = append(associations, Association{
					From:       key.From,
					To:         key.To,
					Type:       key.Type,
					Strength:   actualWeight,
					Confidence: am.calculateConfidence(actualWeight, lastUpdate),
				})
			}
		}
	}
	
	// Sort by strength descending
	sort.Slice(associations, func(i, j int) bool {
		return associations[i].Strength > associations[j].Strength
	})
	
	// Limit results
	if len(associations) > limit {
		associations = associations[:limit]
	}
	
	return associations
}

// GetServiceAffinity returns the affinity between a node and a service type
func (am *AssociationMatrix) GetServiceAffinity(nodeID int64, serviceType string) float64 {
	am.mutex.RLock()
	defer am.mutex.RUnlock()
	
	// For service affinity, we'll use a hash of the service type as the "to" ID
	serviceHash := am.hashServiceType(serviceType)
	key := AssociationKey{From: nodeID, To: serviceHash, Type: NodeToService}
	
	if weight, exists := am.weights[key]; exists {
		lastUpdate := am.lastUpdate[key]
		decayFactor := am.calculateDecay(lastUpdate)
		return weight * decayFactor
	}
	
	return 0.0
}

// UpdateServiceAffinity updates the affinity between a node and service type
func (am *AssociationMatrix) UpdateServiceAffinity(nodeID int64, serviceType string, reward float64) {
	serviceHash := am.hashServiceType(serviceType)
	am.UpdateAssociation(nodeID, serviceHash, NodeToService, reward)
}

// PruneWeakAssociations removes associations below a threshold
func (am *AssociationMatrix) PruneWeakAssociations(threshold float64) int {
	am.mutex.Lock()
	defer am.mutex.Unlock()
	
	var toRemove []AssociationKey
	
	for key, weight := range am.weights {
		lastUpdate := am.lastUpdate[key]
		decayFactor := am.calculateDecay(lastUpdate)
		actualWeight := weight * decayFactor
		
		if actualWeight < threshold {
			toRemove = append(toRemove, key)
		}
	}
	
	// Remove weak associations
	for _, key := range toRemove {
		delete(am.weights, key)
		delete(am.lastUpdate, key)
	}
	
	return len(toRemove)
}

// GetMatrixStats returns statistics about the association matrix
func (am *AssociationMatrix) GetMatrixStats() AssociationMatrixStats {
	am.mutex.RLock()
	defer am.mutex.RUnlock()
	
	totalAssociations := len(am.weights)
	strongAssociations := 0
	weakAssociations := 0
	averageStrength := 0.0
	maxStrength := 0.0
	
	now := time.Now()
	
	for key, weight := range am.weights {
		lastUpdate := am.lastUpdate[key]
		decayFactor := am.calculateDecay(lastUpdate)
		actualWeight := weight * decayFactor
		
		averageStrength += actualWeight
		if actualWeight > maxStrength {
			maxStrength = actualWeight
		}
		
		if actualWeight > 0.5 {
			strongAssociations++
		} else {
			weakAssociations++
		}
	}
	
	if totalAssociations > 0 {
		averageStrength /= float64(totalAssociations)
	}
	
	return AssociationMatrixStats{
		TotalAssociations:  totalAssociations,
		StrongAssociations: strongAssociations,
		WeakAssociations:   weakAssociations,
		AverageStrength:    averageStrength,
		MaxStrength:        maxStrength,
		LastPruned:         now,
	}
}

// calculateDecay computes the temporal decay factor
func (am *AssociationMatrix) calculateDecay(lastUpdate time.Time) float64 {
	if lastUpdate.IsZero() {
		return 1.0
	}
	
	timeDiff := time.Since(lastUpdate)
	hours := timeDiff.Hours()
	
	// Exponential decay: decay_factor = decay_rate ^ hours
	// With decay_rate = 0.95, associations lose 5% strength per hour
	return math.Pow(am.decayRate, hours)
}

// calculateConfidence computes confidence in an association
func (am *AssociationMatrix) calculateConfidence(strength float64, lastUpdate time.Time) float64 {
	// Confidence based on strength and recency
	strengthConfidence := strength
	
	recencyConfidence := 1.0
	if !lastUpdate.IsZero() {
		hours := time.Since(lastUpdate).Hours()
		// Confidence decreases with age
		recencyConfidence = math.Exp(-hours / 24.0) // Half confidence after 24 hours
	}
	
	return strengthConfidence * recencyConfidence
}

// hashServiceType creates a consistent hash for service types
func (am *AssociationMatrix) hashServiceType(serviceType string) int64 {
	// Simple string hash function
	hash := int64(0)
	for i, char := range serviceType {
		hash = hash*31 + int64(char) + int64(i)
	}
	
	// Ensure positive hash
	if hash < 0 {
		hash = -hash
	}
	
	return hash
}

// AssociationMatrixStats provides statistics about the matrix
type AssociationMatrixStats struct {
	TotalAssociations  int
	StrongAssociations int
	WeakAssociations   int
	AverageStrength    float64
	MaxStrength        float64
	LastPruned         time.Time
}

// Export/Import functionality for persistence

// ExportAssociations exports all associations to a serializable format
func (am *AssociationMatrix) ExportAssociations() map[string]AssociationExport {
	am.mutex.RLock()
	defer am.mutex.RUnlock()
	
	exports := make(map[string]AssociationExport)
	
	for key, weight := range am.weights {
		keyStr := fmt.Sprintf("%d-%d-%d", key.From, key.To, int(key.Type))
		exports[keyStr] = AssociationExport{
			From:       key.From,
			To:         key.To,
			Type:       key.Type,
			Weight:     weight,
			LastUpdate: am.lastUpdate[key],
		}
	}
	
	return exports
}

// ImportAssociations imports associations from a serialized format
func (am *AssociationMatrix) ImportAssociations(imports map[string]AssociationExport) {
	am.mutex.Lock()
	defer am.mutex.Unlock()
	
	for _, export := range imports {
		key := AssociationKey{
			From: export.From,
			To:   export.To,
			Type: export.Type,
		}
		
		am.weights[key] = export.Weight
		am.lastUpdate[key] = export.LastUpdate
	}
}

// AssociationExport represents an exportable association
type AssociationExport struct {
	From       int64           `json:"from"`
	To         int64           `json:"to"`
	Type       AssociationType `json:"type"`
	Weight     float64         `json:"weight"`
	LastUpdate time.Time       `json:"last_update"`
}

