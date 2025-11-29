// Package optimization implements multi-objective optimization for network routing
package optimization

import (
	"context"
	"fmt"
	"math"
	"sort"
	"sync"
	"time"

	"github.com/NeoTecDigital/hypermesh/layer3-alm/pkg/graph"
)

// MultiObjectiveOptimizer implements advanced multi-objective optimization algorithms
type MultiObjectiveOptimizer struct {
	// Configuration
	config *OptimizerConfig
	
	// Pareto frontier management
	paretoFront *ParetoFrontier
	
	// Objective functions
	objectives []ObjectiveFunction
	
	// Performance tracking
	optimizationMetrics *OptimizationMetrics
	
	// Thread safety
	mutex sync.RWMutex
}

// OptimizerConfig configures the multi-objective optimizer
type OptimizerConfig struct {
	// Algorithm parameters
	PopulationSize      int
	MaxGenerations      int
	CrossoverRate       float64
	MutationRate        float64
	
	// Objective weights (for TOPSIS when single solution needed)
	LatencyWeight       float64
	ThroughputWeight    float64
	ReliabilityWeight   float64
	CostWeight          float64
	
	// Performance tuning
	MaxConcurrentOpts   int
	OptimizationTimeout time.Duration
	CacheSize          int
	
	// Convergence criteria
	ConvergenceThreshold float64
	StagnationLimit     int
}

// ObjectiveFunction defines an optimization objective
type ObjectiveFunction interface {
	Name() string
	Evaluate(solution *RoutingSolution) float64
	IsMinimizing() bool
	Weight() float64
}

// RoutingSolution represents a candidate routing solution
type RoutingSolution struct {
	Path              []*graph.NetworkNode
	ObjectiveValues   map[string]float64
	Fitness          float64
	DominationRank   int
	CrowdingDistance float64
	
	// Path characteristics
	TotalLatency     time.Duration
	MinThroughput    float64
	AvgReliability   float64
	TotalCost        float64
	HopCount         int
}

// OptimizationRequest defines parameters for multi-objective optimization
type OptimizationRequest struct {
	SourceID       int64
	TargetID       int64
	Objectives     []ObjectiveFunction
	Constraints    []OptimizationConstraint
	MaxSolutions   int
	TimeLimit      time.Duration
	Context        context.Context
}

// OptimizationConstraint defines hard constraints for optimization
type OptimizationConstraint interface {
	Name() string
	Evaluate(solution *RoutingSolution) bool
	Description() string
}

// OptimizationResult contains the Pareto-optimal solutions
type OptimizationResult struct {
	ParetoSolutions  []*RoutingSolution
	BestCompromise   *RoutingSolution
	Generations      int
	ConvergenceTime  time.Duration
	
	// Quality metrics
	HyperVolume      float64
	Spacing          float64
	Spread           float64
	
	// Performance data
	EvaluationCount  int
	CacheHitRate     float64
}

// ParetoFrontier manages the Pareto-optimal solutions
type ParetoFrontier struct {
	solutions map[string]*RoutingSolution
	mutex     sync.RWMutex
}

// NewMultiObjectiveOptimizer creates a new multi-objective optimizer
func NewMultiObjectiveOptimizer(config *OptimizerConfig) *MultiObjectiveOptimizer {
	if config == nil {
		config = DefaultOptimizerConfig()
	}
	
	return &MultiObjectiveOptimizer{
		config:               config,
		paretoFront:         NewParetoFrontier(),
		objectives:          []ObjectiveFunction{},
		optimizationMetrics: NewOptimizationMetrics(),
	}
}

// AddObjective adds an objective function to the optimizer
func (moo *MultiObjectiveOptimizer) AddObjective(objective ObjectiveFunction) {
	moo.mutex.Lock()
	defer moo.mutex.Unlock()
	
	moo.objectives = append(moo.objectives, objective)
}

// Optimize performs multi-objective optimization to find Pareto-optimal solutions
func (moo *MultiObjectiveOptimizer) Optimize(request OptimizationRequest) (*OptimizationResult, error) {
	startTime := time.Now()
	
	// Validate request
	if err := moo.validateRequest(request); err != nil {
		return nil, fmt.Errorf("invalid optimization request: %w", err)
	}
	
	// Use provided objectives or default ones
	objectives := request.Objectives
	if len(objectives) == 0 {
		objectives = moo.getDefaultObjectives()
	}
	
	// Initialize population
	population := moo.initializePopulation(request, objectives)
	
	// Evolution loop (NSGA-II algorithm)
	generation := 0
	stagnationCounter := 0
	var previousHyperVolume float64
	
	for generation < moo.config.MaxGenerations {
		// Check timeout
		if request.TimeLimit > 0 && time.Since(startTime) > request.TimeLimit {
			break
		}
		
		// Evaluate population
		moo.evaluatePopulation(population, objectives, request.Constraints)
		
		// Non-dominated sorting
		fronts := moo.nonDominatedSorting(population)
		
		// Crowding distance calculation
		for _, front := range fronts {
			moo.calculateCrowdingDistance(front, objectives)
		}
		
		// Selection for next generation
		newPopulation := moo.selection(fronts)
		
		// Crossover and mutation
		offspring := moo.crossoverAndMutation(newPopulation, request)
		
		// Combine parent and offspring
		combined := append(population, offspring...)
		population = combined
		
		// Check convergence
		currentHyperVolume := moo.calculateHyperVolume(fronts[0], objectives)
		if math.Abs(currentHyperVolume-previousHyperVolume) < moo.config.ConvergenceThreshold {
			stagnationCounter++
			if stagnationCounter >= moo.config.StagnationLimit {
				break
			}
		} else {
			stagnationCounter = 0
		}
		previousHyperVolume = currentHyperVolume
		
		generation++
	}
	
	// Extract final Pareto front
	finalFronts := moo.nonDominatedSorting(population)
	paretoSolutions := finalFronts[0]
	
	// Select best compromise solution using TOPSIS
	bestCompromise := moo.selectBestCompromise(paretoSolutions, objectives)
	
	// Calculate quality metrics
	hyperVolume := moo.calculateHyperVolume(paretoSolutions, objectives)
	spacing := moo.calculateSpacing(paretoSolutions, objectives)
	spread := moo.calculateSpread(paretoSolutions, objectives)
	
	result := &OptimizationResult{
		ParetoSolutions:  paretoSolutions,
		BestCompromise:   bestCompromise,
		Generations:      generation,
		ConvergenceTime:  time.Since(startTime),
		HyperVolume:      hyperVolume,
		Spacing:          spacing,
		Spread:           spread,
		EvaluationCount:  generation * moo.config.PopulationSize,
		CacheHitRate:     moo.optimizationMetrics.GetCacheHitRate(),
	}
	
	// Update metrics
	moo.optimizationMetrics.RecordOptimization(result)
	
	return result, nil
}

// nonDominatedSorting implements the non-dominated sorting algorithm
func (moo *MultiObjectiveOptimizer) nonDominatedSorting(population []*RoutingSolution) [][]*RoutingSolution {
	fronts := make([][]*RoutingSolution, 0)
	
	// Calculate domination relationships
	dominated := make(map[*RoutingSolution][]*RoutingSolution)
	dominationCount := make(map[*RoutingSolution]int)
	
	for _, p := range population {
		dominated[p] = make([]*RoutingSolution, 0)
		dominationCount[p] = 0
		
		for _, q := range population {
			if p != q {
				if moo.dominates(p, q) {
					dominated[p] = append(dominated[p], q)
				} else if moo.dominates(q, p) {
					dominationCount[p]++
				}
			}
		}
		
		if dominationCount[p] == 0 {
			p.DominationRank = 0
			if len(fronts) == 0 {
				fronts = append(fronts, make([]*RoutingSolution, 0))
			}
			fronts[0] = append(fronts[0], p)
		}
	}
	
	// Build subsequent fronts
	i := 0
	for len(fronts) > i && len(fronts[i]) > 0 {
		nextFront := make([]*RoutingSolution, 0)
		
		for _, p := range fronts[i] {
			for _, q := range dominated[p] {
				dominationCount[q]--
				if dominationCount[q] == 0 {
					q.DominationRank = i + 1
					nextFront = append(nextFront, q)
				}
			}
		}
		
		if len(nextFront) > 0 {
			fronts = append(fronts, nextFront)
		}
		i++
	}
	
	return fronts
}

// dominates checks if solution p dominates solution q
func (moo *MultiObjectiveOptimizer) dominates(p, q *RoutingSolution) bool {
	betterInAtLeastOne := false
	
	for objName, pValue := range p.ObjectiveValues {
		qValue, exists := q.ObjectiveValues[objName]
		if !exists {
			continue
		}
		
		// Find the objective function to check if it's minimizing
		var isMinimizing bool
		for _, obj := range moo.objectives {
			if obj.Name() == objName {
				isMinimizing = obj.IsMinimizing()
				break
			}
		}
		
		if isMinimizing {
			if pValue > qValue {
				return false // p is worse than q in this objective
			}
			if pValue < qValue {
				betterInAtLeastOne = true
			}
		} else {
			if pValue < qValue {
				return false // p is worse than q in this objective
			}
			if pValue > qValue {
				betterInAtLeastOne = true
			}
		}
	}
	
	return betterInAtLeastOne
}

// calculateCrowdingDistance calculates crowding distance for diversity preservation
func (moo *MultiObjectiveOptimizer) calculateCrowdingDistance(front []*RoutingSolution, objectives []ObjectiveFunction) {
	if len(front) <= 2 {
		for _, solution := range front {
			solution.CrowdingDistance = math.Inf(1)
		}
		return
	}
	
	// Initialize distances
	for _, solution := range front {
		solution.CrowdingDistance = 0.0
	}
	
	// For each objective
	for _, objective := range objectives {
		objName := objective.Name()
		
		// Sort solutions by this objective
		sort.Slice(front, func(i, j int) bool {
			if objective.IsMinimizing() {
				return front[i].ObjectiveValues[objName] < front[j].ObjectiveValues[objName]
			}
			return front[i].ObjectiveValues[objName] > front[j].ObjectiveValues[objName]
		})
		
		// Set boundary solutions to infinite distance
		front[0].CrowdingDistance = math.Inf(1)
		front[len(front)-1].CrowdingDistance = math.Inf(1)
		
		// Calculate range
		objRange := math.Abs(front[len(front)-1].ObjectiveValues[objName] - front[0].ObjectiveValues[objName])
		if objRange == 0 {
			continue
		}
		
		// Calculate distances for intermediate solutions
		for i := 1; i < len(front)-1; i++ {
			distance := math.Abs(front[i+1].ObjectiveValues[objName] - front[i-1].ObjectiveValues[objName])
			front[i].CrowdingDistance += distance / objRange
		}
	}
}

// Default objective functions

// LatencyObjective minimizes total path latency
type LatencyObjective struct {
	weight float64
}

func (lo *LatencyObjective) Name() string { return "latency" }
func (lo *LatencyObjective) Evaluate(solution *RoutingSolution) float64 {
	return float64(solution.TotalLatency.Microseconds())
}
func (lo *LatencyObjective) IsMinimizing() bool { return true }
func (lo *LatencyObjective) Weight() float64 { return lo.weight }

// ThroughputObjective maximizes minimum path throughput
type ThroughputObjective struct {
	weight float64
}

func (to *ThroughputObjective) Name() string { return "throughput" }
func (to *ThroughputObjective) Evaluate(solution *RoutingSolution) float64 {
	return solution.MinThroughput
}
func (to *ThroughputObjective) IsMinimizing() bool { return false }
func (to *ThroughputObjective) Weight() float64 { return to.weight }

// ReliabilityObjective maximizes average path reliability
type ReliabilityObjective struct {
	weight float64
}

func (ro *ReliabilityObjective) Name() string { return "reliability" }
func (ro *ReliabilityObjective) Evaluate(solution *RoutingSolution) float64 {
	return solution.AvgReliability
}
func (ro *ReliabilityObjective) IsMinimizing() bool { return false }
func (ro *ReliabilityObjective) Weight() float64 { return ro.weight }

// CostObjective minimizes total path cost
type CostObjective struct {
	weight float64
}

func (co *CostObjective) Name() string { return "cost" }
func (co *CostObjective) Evaluate(solution *RoutingSolution) float64 {
	return solution.TotalCost
}
func (co *CostObjective) IsMinimizing() bool { return true }
func (co *CostObjective) Weight() float64 { return co.weight }

// getDefaultObjectives returns the standard set of optimization objectives
func (moo *MultiObjectiveOptimizer) getDefaultObjectives() []ObjectiveFunction {
	return []ObjectiveFunction{
		&LatencyObjective{weight: moo.config.LatencyWeight},
		&ThroughputObjective{weight: moo.config.ThroughputWeight},
		&ReliabilityObjective{weight: moo.config.ReliabilityWeight},
		&CostObjective{weight: moo.config.CostWeight},
	}
}

// DefaultOptimizerConfig returns default optimizer configuration
func DefaultOptimizerConfig() *OptimizerConfig {
	return &OptimizerConfig{
		PopulationSize:       100,
		MaxGenerations:       50,
		CrossoverRate:        0.8,
		MutationRate:         0.1,
		LatencyWeight:        0.3,
		ThroughputWeight:     0.3,
		ReliabilityWeight:    0.2,
		CostWeight:          0.2,
		MaxConcurrentOpts:    10,
		OptimizationTimeout: 30 * time.Second,
		CacheSize:           1000,
		ConvergenceThreshold: 0.001,
		StagnationLimit:     5,
	}
}

// OptimizationMetrics tracks optimizer performance
type OptimizationMetrics struct {
	totalOptimizations int64
	totalEvaluations   int64
	cacheHits         int64
	averageTime       time.Duration
	mutex             sync.Mutex
}

func NewOptimizationMetrics() *OptimizationMetrics {
	return &OptimizationMetrics{}
}

func (om *OptimizationMetrics) RecordOptimization(result *OptimizationResult) {
	om.mutex.Lock()
	defer om.mutex.Unlock()
	
	om.totalOptimizations++
	om.totalEvaluations += int64(result.EvaluationCount)
	// Update average time using exponential moving average
	if om.totalOptimizations == 1 {
		om.averageTime = result.ConvergenceTime
	} else {
		om.averageTime = time.Duration((float64(om.averageTime)*0.9) + (float64(result.ConvergenceTime)*0.1))
	}
}

func (om *OptimizationMetrics) GetCacheHitRate() float64 {
	om.mutex.Lock()
	defer om.mutex.Unlock()
	
	if om.totalEvaluations == 0 {
		return 0.0
	}
	return float64(om.cacheHits) / float64(om.totalEvaluations) * 100.0
}

// validateRequest validates an optimization request
func (moo *MultiObjectiveOptimizer) validateRequest(request OptimizationRequest) error {
	if request.SourceID <= 0 || request.TargetID <= 0 {
		return fmt.Errorf("invalid source or target ID")
	}
	
	if request.SourceID == request.TargetID {
		return fmt.Errorf("source and target cannot be the same")
	}
	
	return nil
}

// initializePopulation creates the initial population for optimization
func (moo *MultiObjectiveOptimizer) initializePopulation(request OptimizationRequest, objectives []ObjectiveFunction) []*RoutingSolution {
	population := make([]*RoutingSolution, moo.config.PopulationSize)
	
	for i := 0; i < moo.config.PopulationSize; i++ {
		// Generate random or heuristic-based initial solutions
		solution := moo.generateRandomSolution(request)
		population[i] = solution
	}
	
	return population
}

// evaluatePopulation evaluates all solutions in the population
func (moo *MultiObjectiveOptimizer) evaluatePopulation(population []*RoutingSolution, objectives []ObjectiveFunction, constraints []OptimizationConstraint) {
	for _, solution := range population {
		moo.evaluateSolution(solution, objectives, constraints)
	}
}

// evaluateSolution evaluates a single solution against all objectives
func (moo *MultiObjectiveOptimizer) evaluateSolution(solution *RoutingSolution, objectives []ObjectiveFunction, constraints []OptimizationConstraint) {
	solution.ObjectiveValues = make(map[string]float64)
	
	// Calculate objective values
	totalFitness := 0.0
	for _, objective := range objectives {
		value := objective.Evaluate(solution)
		solution.ObjectiveValues[objective.Name()] = value
		
		// Weighted fitness calculation
		weight := objective.Weight()
		if objective.IsMinimizing() {
			totalFitness += weight * (1.0 / (1.0 + value)) // Invert for minimizing objectives
		} else {
			totalFitness += weight * value
		}
	}
	
	solution.Fitness = totalFitness
	
	// Check constraints
	for _, constraint := range constraints {
		if !constraint.Evaluate(solution) {
			solution.Fitness *= 0.1 // Heavily penalize constraint violations
		}
	}
}

// selection implements selection for the next generation
func (moo *MultiObjectiveOptimizer) selection(fronts [][]*RoutingSolution) []*RoutingSolution {
	newPopulation := make([]*RoutingSolution, 0, moo.config.PopulationSize)
	
	// Add entire fronts until we approach population size
	for _, front := range fronts {
		if len(newPopulation)+len(front) <= moo.config.PopulationSize {
			newPopulation = append(newPopulation, front...)
		} else {
			// Sort this front by crowding distance and add best ones
			remaining := moo.config.PopulationSize - len(newPopulation)
			sortedFront := moo.sortByCrowdingDistance(front)
			newPopulation = append(newPopulation, sortedFront[:remaining]...)
			break
		}
	}
	
	return newPopulation
}

// crossoverAndMutation performs crossover and mutation operations
func (moo *MultiObjectiveOptimizer) crossoverAndMutation(population []*RoutingSolution, request OptimizationRequest) []*RoutingSolution {
	offspring := make([]*RoutingSolution, 0, len(population))
	
	for i := 0; i < len(population); i += 2 {
		parent1 := population[i]
		parent2 := population[(i+1)%len(population)]
		
		// Crossover
		if moo.randomFloat() < moo.config.CrossoverRate {
			child1, child2 := moo.crossover(parent1, parent2, request)
			offspring = append(offspring, child1, child2)
		} else {
			offspring = append(offspring, moo.copySolution(parent1), moo.copySolution(parent2))
		}
	}
	
	// Mutation
	for _, solution := range offspring {
		if moo.randomFloat() < moo.config.MutationRate {
			moo.mutate(solution, request)
		}
	}
	
	return offspring
}

// calculateHyperVolume calculates the hypervolume indicator for a front
func (moo *MultiObjectiveOptimizer) calculateHyperVolume(front []*RoutingSolution, objectives []ObjectiveFunction) float64 {
	if len(front) == 0 {
		return 0.0
	}
	
	// Simplified hypervolume calculation for 2D case
	if len(objectives) == 2 {
		return moo.calculateHyperVolume2D(front, objectives)
	}
	
	// For higher dimensions, use approximation
	return moo.approximateHyperVolume(front, objectives)
}

// calculateHyperVolume2D calculates hypervolume for 2 objectives
func (moo *MultiObjectiveOptimizer) calculateHyperVolume2D(front []*RoutingSolution, objectives []ObjectiveFunction) float64 {
	if len(front) == 0 {
		return 0.0
	}
	
	// Sort solutions by first objective
	obj1Name := objectives[0].Name()
	obj2Name := objectives[1].Name()
	
	// Simple area calculation
	area := 0.0
	for i, solution := range front {
		if i == 0 {
			area += solution.ObjectiveValues[obj1Name] * solution.ObjectiveValues[obj2Name]
		} else {
			prevSolution := front[i-1]
			width := solution.ObjectiveValues[obj1Name] - prevSolution.ObjectiveValues[obj1Name]
			height := solution.ObjectiveValues[obj2Name]
			area += width * height
		}
	}
	
	return area
}

// approximateHyperVolume provides an approximation for higher dimensions
func (moo *MultiObjectiveOptimizer) approximateHyperVolume(front []*RoutingSolution, objectives []ObjectiveFunction) float64 {
	if len(front) == 0 {
		return 0.0
	}
	
	// Simple approximation: product of normalized objective ranges
	volume := 1.0
	
	for _, objective := range objectives {
		objName := objective.Name()
		minVal := math.Inf(1)
		maxVal := math.Inf(-1)
		
		for _, solution := range front {
			val := solution.ObjectiveValues[objName]
			if val < minVal {
				minVal = val
			}
			if val > maxVal {
				maxVal = val
			}
		}
		
		if maxVal > minVal {
			volume *= (maxVal - minVal)
		}
	}
	
	return volume
}

// calculateSpacing calculates the spacing metric for diversity
func (moo *MultiObjectiveOptimizer) calculateSpacing(front []*RoutingSolution, objectives []ObjectiveFunction) float64 {
	if len(front) <= 1 {
		return 0.0
	}
	
	distances := make([]float64, len(front))
	
	for i, solution := range front {
		minDistance := math.Inf(1)
		
		for j, other := range front {
			if i != j {
				distance := moo.calculateObjectiveSpaceDistance(solution, other, objectives)
				if distance < minDistance {
					minDistance = distance
				}
			}
		}
		
		distances[i] = minDistance
	}
	
	// Calculate mean distance
	mean := 0.0
	for _, d := range distances {
		mean += d
	}
	mean /= float64(len(distances))
	
	// Calculate variance
	variance := 0.0
	for _, d := range distances {
		variance += (d - mean) * (d - mean)
	}
	variance /= float64(len(distances))
	
	return math.Sqrt(variance)
}

// calculateSpread calculates the spread metric
func (moo *MultiObjectiveOptimizer) calculateSpread(front []*RoutingSolution, objectives []ObjectiveFunction) float64 {
	if len(front) <= 1 {
		return 0.0
	}
	
	// Find extreme solutions for each objective
	extremeDistances := 0.0
	
	for _, objective := range objectives {
		objName := objective.Name()
		var minSolution, maxSolution *RoutingSolution
		minVal := math.Inf(1)
		maxVal := math.Inf(-1)
		
		for _, solution := range front {
			val := solution.ObjectiveValues[objName]
			if val < minVal {
				minVal = val
				minSolution = solution
			}
			if val > maxVal {
				maxVal = val
				maxSolution = solution
			}
		}
		
		if minSolution != nil && maxSolution != nil {
			extremeDistances += moo.calculateObjectiveSpaceDistance(minSolution, maxSolution, objectives)
		}
	}
	
	return extremeDistances / float64(len(objectives))
}

// selectBestCompromise selects the best compromise solution using TOPSIS
func (moo *MultiObjectiveOptimizer) selectBestCompromise(solutions []*RoutingSolution, objectives []ObjectiveFunction) *RoutingSolution {
	if len(solutions) == 0 {
		return nil
	}
	
	if len(solutions) == 1 {
		return solutions[0]
	}
	
	// TOPSIS implementation
	bestScore := math.Inf(-1)
	var bestSolution *RoutingSolution
	
	for _, solution := range solutions {
		score := moo.calculateTOPSISScore(solution, solutions, objectives)
		if score > bestScore {
			bestScore = score
			bestSolution = solution
		}
	}
	
	return bestSolution
}

// Helper methods

func (moo *MultiObjectiveOptimizer) generateRandomSolution(request OptimizationRequest) *RoutingSolution {
	// This would generate a random path from source to target
	// For now, return a basic solution
	return &RoutingSolution{
		Path:            make([]*graph.NetworkNode, 0),
		ObjectiveValues: make(map[string]float64),
		TotalLatency:    time.Duration(1000 + moo.randomInt(5000)) * time.Microsecond,
		MinThroughput:   100.0 + moo.randomFloat()*900.0,
		AvgReliability:  0.5 + moo.randomFloat()*0.5,
		TotalCost:       10.0 + moo.randomFloat()*90.0,
		HopCount:        2 + moo.randomInt(8),
	}
}

func (moo *MultiObjectiveOptimizer) sortByCrowdingDistance(front []*RoutingSolution) []*RoutingSolution {
	// Sort by crowding distance (descending)
	sorted := make([]*RoutingSolution, len(front))
	copy(sorted, front)
	
	for i := 0; i < len(sorted)-1; i++ {
		for j := 0; j < len(sorted)-i-1; j++ {
			if sorted[j].CrowdingDistance < sorted[j+1].CrowdingDistance {
				sorted[j], sorted[j+1] = sorted[j+1], sorted[j]
			}
		}
	}
	
	return sorted
}

func (moo *MultiObjectiveOptimizer) crossover(parent1, parent2 *RoutingSolution, request OptimizationRequest) (*RoutingSolution, *RoutingSolution) {
	// Simple path crossover - in production would use more sophisticated methods
	child1 := moo.copySolution(parent1)
	child2 := moo.copySolution(parent2)
	
	// Swap some characteristics
	child1.MinThroughput = parent2.MinThroughput
	child2.MinThroughput = parent1.MinThroughput
	
	child1.AvgReliability = parent2.AvgReliability
	child2.AvgReliability = parent1.AvgReliability
	
	return child1, child2
}

func (moo *MultiObjectiveOptimizer) mutate(solution *RoutingSolution, request OptimizationRequest) {
	// Random mutation of solution characteristics
	if moo.randomFloat() < 0.5 {
		solution.TotalLatency += time.Duration((moo.randomFloat()-0.5)*1000) * time.Microsecond
	}
	
	if moo.randomFloat() < 0.5 {
		solution.MinThroughput += (moo.randomFloat() - 0.5) * 100.0
		if solution.MinThroughput < 0 {
			solution.MinThroughput = 10.0
		}
	}
	
	if moo.randomFloat() < 0.5 {
		solution.AvgReliability += (moo.randomFloat() - 0.5) * 0.2
		if solution.AvgReliability < 0 {
			solution.AvgReliability = 0.1
		}
		if solution.AvgReliability > 1 {
			solution.AvgReliability = 1.0
		}
	}
}

func (moo *MultiObjectiveOptimizer) copySolution(original *RoutingSolution) *RoutingSolution {
	solutionCopy := &RoutingSolution{
		Path:              make([]*graph.NetworkNode, len(original.Path)),
		ObjectiveValues:   make(map[string]float64),
		Fitness:          original.Fitness,
		DominationRank:   original.DominationRank,
		CrowdingDistance: original.CrowdingDistance,
		TotalLatency:     original.TotalLatency,
		MinThroughput:    original.MinThroughput,
		AvgReliability:   original.AvgReliability,
		TotalCost:        original.TotalCost,
		HopCount:         original.HopCount,
	}
	
	copy(solutionCopy.Path, original.Path)
	
	for k, v := range original.ObjectiveValues {
		solutionCopy.ObjectiveValues[k] = v
	}
	
	return solutionCopy
}

func (moo *MultiObjectiveOptimizer) calculateObjectiveSpaceDistance(sol1, sol2 *RoutingSolution, objectives []ObjectiveFunction) float64 {
	distance := 0.0
	
	for _, objective := range objectives {
		objName := objective.Name()
		val1 := sol1.ObjectiveValues[objName]
		val2 := sol2.ObjectiveValues[objName]
		distance += (val1 - val2) * (val1 - val2)
	}
	
	return math.Sqrt(distance)
}

func (moo *MultiObjectiveOptimizer) calculateTOPSISScore(solution *RoutingSolution, allSolutions []*RoutingSolution, objectives []ObjectiveFunction) float64 {
	// Simplified TOPSIS scoring
	score := 0.0
	
	for _, objective := range objectives {
		objName := objective.Name()
		weight := objective.Weight()
		value := solution.ObjectiveValues[objName]
		
		if objective.IsMinimizing() {
			score += weight * (1.0 / (1.0 + value))
		} else {
			score += weight * value
		}
	}
	
	return score
}

func (moo *MultiObjectiveOptimizer) randomFloat() float64 {
	// Simple pseudo-random number - in production use crypto/rand
	return 0.5 // Placeholder
}

func (moo *MultiObjectiveOptimizer) randomInt(max int) int {
	// Simple pseudo-random int - in production use crypto/rand
	return max / 2 // Placeholder
}

// NewParetoFrontier creates a new Pareto frontier manager
func NewParetoFrontier() *ParetoFrontier {
	return &ParetoFrontier{
		solutions: make(map[string]*RoutingSolution),
	}
}