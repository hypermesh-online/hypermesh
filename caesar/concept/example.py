# Copyright © 2026 Hypermesh Foundation. All rights reserved.
# Licensed under the Business Source License 1.1.
# See the LICENSE file in the repository root for full license text.

from formulas import *
from reports import *
import time
from datetime import datetime

class MarketSimulation:
    def __init__(self, initial_conditions, simulation_duration):
        """Initialize market simulation with conditions and duration"""
        self.conditions = initial_conditions
        self.duration = simulation_duration
        self.market_metrics = MarketMetrics()
        self.results = []
        
    def run_epoch(self, epoch_number):
        """Run a single epoch of the simulation"""
        # Calculate epoch duration based on conditions
        epoch_duration = determine_epoch_duration(
            self.conditions['daily_transactions'],
            self.conditions['validator_count'] * 1000,
            self.market_metrics.get_volatility()
        )
        
        # Calculate economics for this epoch
        economics = calculate_economics(
            **self.conditions,
            market_metrics=self.market_metrics
        )
        
        # Update conditions based on results
        self._update_conditions(economics)
        
        # Calculate transaction volume
        transaction_volume = (
            self.conditions['daily_transactions'] * 
            self.conditions['avg_transaction_size']
        )
        
        # Create epoch result with all necessary data
        epoch_result = {
            'epoch': epoch_number,
            'epoch_duration': epoch_duration,
            'current_price': float(self.conditions['current_price']),
            'liquidity_ratio': float(self.conditions['liquidity_ratio']),
            'validator_count': int(self.conditions['validator_count']),
            'total_holders': int(self.conditions['total_holders']),
            'transaction_volume': float(transaction_volume),
            'daily_transactions': int(self.conditions['daily_transactions']),
            **economics
        }
        
        # Store the results
        self.results.append(epoch_result)
        
    def _update_conditions(self, economics):
        """Update market conditions based on economic results"""
        # Apply market pressure effects
        if economics['circuit_breakers']['needs_rebase']:
            self.conditions['current_price'] = max(0.95, min(1.05, 
                (1 + self.conditions['current_price']) / 2))
        
        # Update volumes based on market pressure
        pressure_adjustment = economics['market_pressure'] * 0.1
        self.conditions['buys_volume'] *= (1 - pressure_adjustment)
        self.conditions['sells_volume'] *= (1 + pressure_adjustment)
        
        # Update participation metrics
        if economics['price_stability_index'] < 0.8:
            self.conditions['daily_transactions'] *= 0.95
            self.conditions['total_holders'] *= 0.99
        else:
            self.conditions['daily_transactions'] *= 1.01
            self.conditions['total_holders'] *= 1.005

def run_comprehensive_simulation(initial_conditions, duration_days=7):
    """Run comprehensive market simulation"""
    # Store scenario name if it exists
    scenario_name = initial_conditions.get('name', 'Base Scenario')
    
    # Create a copy of conditions without the 'name' key for simulation
    sim_conditions = {k: v for k, v in initial_conditions.items() if k != 'name'}
    
    # Calculate total epochs based on duration
    total_epochs = int(duration_days * 8640)  # From precept: 8640 epochs per day
    
    # Initialize simulation
    sim = MarketSimulation(sim_conditions, total_epochs)
    
    # Run simulation
    print(f"\nStarting simulation for {scenario_name}: {duration_days} days ({total_epochs} epochs)")
    start_time = time.time()
    
    for epoch in range(total_epochs):
        sim.run_epoch(epoch)
        
        # Progress update every 1000 epochs
        if epoch % 1000 == 0:
            progress = (epoch / total_epochs) * 100
            print(f"Progress: {progress:.1f}% complete")
    
    duration = time.time() - start_time
    print(f"\nSimulation completed in {duration:.2f} seconds")
    
    # Generate analysis
    report_path = f"reports/{scenario_name.replace(' ', '_').lower()}"
    
    analysis = analyze_simulation_results(sim.results, {
        'price_deviation_max': 0.02,
        'liquidity_variance_max': 0.1,
        'participant_retention_min': 0.9,
        'settlement_rate_min': 0.99
    })
    
    # Create detailed report
    create_analysis_report(sim.results, analysis, report_path, scenario_name)
    
    # Add scenario name to analysis
    analysis['scenario_name'] = scenario_name
    
    return sim.results, analysis

def analyze_simulation_results(results, targets):
    """Analyze simulation results against targets"""
    analysis = {
        'stability_metrics': calculate_stability_metrics(results),
        'equilibrium_states': analyze_equilibrium_states(results),
        'recovery_metrics': analyze_recovery_metrics(results),
        'economic_metrics': analyze_economic_metrics(results)
    }
    
    # Validate against targets
    analysis['success_criteria'] = validate_targets(analysis, targets)
    
    return analysis

if __name__ == "__main__":
    initial_conditions = {
        "validator_count": 5000,
        "total_holders": 1000000,
        "daily_transactions": 24305,
        "current_price": 1.00,
        "avg_transaction_size": 6000,
        "avg_holding_balance": 10000,
        "days_held": 30,
        "liquidity_ratio": 0.8,
        "cross_chain_transfers": 1000,
        "buys_volume": 50000000,
        "sells_volume": 50000000
    }

    # === COMBINED CRISIS SCENARIOS ===
    stress_scenarios = [
        {
            "name": "Combined Crisis - Price Shock + High Pressure",
            "validator_count": 3000,
            "total_holders": 700000,
            "daily_transactions": 60000000,
            "current_price": 1.45,              # Price shock high
            "avg_transaction_size": 7000,
            "avg_holding_balance": 9000,
            "days_held": 10,
            "liquidity_ratio": 0.4,            # Pressure on liquidity
            "cross_chain_transfers": 300000,
            "buys_volume": 200000000000,       # Heavy buy pressure
            "sells_volume": 50000000000
        },
        {
            "name": "Combined Crisis - Mass Exodus + Liquidity Drain",
            "validator_count": 2000,           # Validator exodus
            "total_holders": 400000,           # Holder exodus
            "daily_transactions": 80000000,    # Panic transactions
            "current_price": 0.6,
            "avg_transaction_size": 3000,
            "avg_holding_balance": 5000,
            "days_held": 3,                    # Very short hold time
            "liquidity_ratio": 0.1,           # Severe liquidity crisis
            "cross_chain_transfers": 600000,   # High cross-chain exits
            "buys_volume": 20000000000,
            "sells_volume": 180000000000      # Heavy selling
        },
        {
            "name": "Combined Crisis - Network Stress + Price Instability",
            "validator_count": 1500,           # Low validator count
            "total_holders": 300000,
            "daily_transactions": 90000000,    # Very high transaction load
            "current_price": 0.55,            # Severe price drop
            "avg_transaction_size": 2500,
            "avg_holding_balance": 4000,
            "days_held": 2,
            "liquidity_ratio": 0.05,          # Critical liquidity
            "cross_chain_transfers": 700000,
            "buys_volume": 10000000000,
            "sells_volume": 200000000000
        },

        # === GROWTH STRESS SCENARIOS ===
        {
            "name": "Rapid Growth - Network Expansion",
            "validator_count": 20000,          # Rapidly growing validator set
            "total_holders": 5000000,
            "daily_transactions": 100000000,
            "current_price": 1.15,
            "avg_transaction_size": 8000,
            "avg_holding_balance": 12000,
            "days_held": 45,
            "liquidity_ratio": 0.9,           # High liquidity
            "cross_chain_transfers": 250000,
            "buys_volume": 300000000000,
            "sells_volume": 200000000000
        },
        {
            "name": "Rapid Growth - Price Appreciation",
            "validator_count": 15000,
            "total_holders": 4000000,
            "daily_transactions": 85000000,
            "current_price": 1.3,             # Strong price growth
            "avg_transaction_size": 10000,
            "avg_holding_balance": 15000,
            "days_held": 60,
            "liquidity_ratio": 0.85,
            "cross_chain_transfers": 200000,
            "buys_volume": 400000000000,      # Heavy buying
            "sells_volume": 150000000000
        },

        # === VOLATILITY SCENARIOS ===
        {
            "name": "High Volatility - Price Oscillation",
            "validator_count": 4500,
            "total_holders": 900000,
            "daily_transactions": 55000000,
            "current_price": 1.2,             # Starting above peg
            "avg_transaction_size": 7000,
            "avg_holding_balance": 9000,
            "days_held": 15,
            "liquidity_ratio": 0.6,
            "cross_chain_transfers": 350000,
            "buys_volume": 150000000000,
            "sells_volume": 150000000000      # Equal pressure
        },
        {
            "name": "High Volatility - Network Participation",
            "validator_count": 5000,
            "total_holders": 1200000,
            "daily_transactions": 75000000,    # High but volatile tx count
            "current_price": 0.85,
            "avg_transaction_size": 5500,
            "avg_holding_balance": 8000,
            "days_held": 20,
            "liquidity_ratio": 0.5,
            "cross_chain_transfers": 400000,
            "buys_volume": 100000000000,
            "sells_volume": 120000000000
        },

        # === RECOVERY STRESS SCENARIOS ===
        {
            "name": "Stressed Recovery - Post Crisis",
            "validator_count": 3000,
            "total_holders": 800000,           # Recovering holder base
            "daily_transactions": 40000000,
            "current_price": 0.95,            # Approaching peg
            "avg_transaction_size": 6000,
            "avg_holding_balance": 9000,
            "days_held": 30,
            "liquidity_ratio": 0.65,          # Recovering liquidity
            "cross_chain_transfers": 150000,
            "buys_volume": 140000000000,      # Buy pressure returning
            "sells_volume": 100000000000
        },
        {
            "name": "Stressed Recovery - Network Rebuilding",
            "validator_count": 4000,           # Growing validator count
            "total_holders": 900000,
            "daily_transactions": 50000000,
            "current_price": 0.92,
            "avg_transaction_size": 6500,
            "avg_holding_balance": 10000,
            "days_held": 35,
            "liquidity_ratio": 0.7,
            "cross_chain_transfers": 180000,
            "buys_volume": 160000000000,
            "sells_volume": 120000000000
        }
    ]

    # Run base simulation
    results, analysis = run_comprehensive_simulation(initial_conditions)
    
    # Run stress scenarios
    for scenario in stress_scenarios:
        print(f"\nRunning stress scenario: {scenario['name']}")
        scenario_results, scenario_analysis = run_comprehensive_simulation(scenario)
        
        # Compare results
        print("\nScenario Analysis:")
        stability_metrics = scenario_analysis['stability_metrics']
        metrics_to_display = {
            'Price Stability': 'price_stability',
            'Liquidity Health': 'liquidity_health',
            'Network Utility': 'network_utility'
        }
        
        for label, metric in metrics_to_display.items():
            value = stability_metrics.get(metric, 'N/A')
            if value != 'N/A':
                print(f"{label}: {value:.2f}")
            else:
                print(f"{label}: {value}")#