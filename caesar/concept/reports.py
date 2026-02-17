# Copyright © 2026 Hypermesh Foundation. All rights reserved.
# Licensed under the Business Source License 1.1.
# See the LICENSE file in the repository root for full license text.

import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd
import numpy as np
import os
from datetime import datetime

def plot_temporal_analysis(df, report_dir, scenario_name):
    """Plot metrics across different time scales as defined in precept"""
    # Time scales from precept: minute (6 epochs), hour (360), day (8640), week (60480)
    time_scales = {
        'minute': 6,
        'hour': 360,
        'day': 8640,
        'week': 60480
    }
    
    fig, axes = plt.subplots(2, 2, figsize=(15, 12))
    axes = axes.flatten()
    
    for idx, (scale, epochs) in enumerate(time_scales.items()):
        # Resample data for each time scale
        df_resampled = df.groupby(df.index // epochs).agg({
            'current_price': ['mean', 'std'],
            'liquidity_ratio': ['mean', 'min'],
            'network_utility_score': 'mean',
            'transaction_fee_usdc': 'sum'
        })
        
        ax = axes[idx]
        ax.plot(df_resampled.index, df_resampled[('current_price', 'mean')], label='Avg Price')
        ax.fill_between(df_resampled.index, 
                       df_resampled[('current_price', 'mean')] - df_resampled[('current_price', 'std')],
                       df_resampled[('current_price', 'mean')] + df_resampled[('current_price', 'std')],
                       alpha=0.2)
        ax.set_title(f'{scale.capitalize()} Scale Analysis')
        ax.legend()
    
    plt.tight_layout()
    plt.savefig(f"{report_dir}/temporal_analysis.png")
    plt.close()

def plot_participant_dynamics(df, report_dir, scenario_name):
    """Plot participant dynamics as defined in precept section 3.1"""
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 10))
    
    # Participant Evolution (dV/dt, dH/dt, dT/dt)
    ax1.plot(df['epoch'], df['validator_count'], label='Validators (V(t))')
    ax1.plot(df['epoch'], df['holder_count'], label='Holders (H(t))')
    ax1.plot(df['epoch'], df['transaction_volume'], label='Transactions (T(t))')
    ax1.set_title('Participant Evolution')
    ax1.legend()
    
    # Network Balance Ratio (0.8 ≤ V(t)/T(t) ≤ 1.2)
    v_t_ratio = df['validator_count'] / df['transaction_volume']
    ax2.plot(df['epoch'], v_t_ratio, label='V(t)/T(t) Ratio')
    ax2.axhline(y=0.8, color='r', linestyle='--', alpha=0.3, label='Min Threshold')
    ax2.axhline(y=1.2, color='r', linestyle='--', alpha=0.3, label='Max Threshold')
    ax2.set_title('Network Balance Ratio')
    ax2.legend()
    
    plt.tight_layout()
    plt.savefig(f"{report_dir}/participant_dynamics.png")
    plt.close()

def plot_circuit_breaker_analysis(df, report_dir, scenario_name):
    """Plot circuit breaker conditions and activations"""
    fig, ax = plt.subplots(figsize=(12, 6))
    
    # Circuit breaker conditions from precept section 5.2
    ax.plot(df['epoch'], df['liquidity_ratio'], label='Liquidity Ratio')
    ax.axhline(y=0.1, color='r', linestyle='--', alpha=0.3, label='Halt Threshold')
    ax.axhline(y=0.2, color='y', linestyle='--', alpha=0.3, label='Emergency Threshold')
    
    # Mark circuit breaker activations
    halt_points = df[df['circuit_breakers'].apply(lambda x: x['halt_trading'])]
    emergency_points = df[df['circuit_breakers'].apply(lambda x: x['emergency_spreads'])]
    rebase_points = df[df['circuit_breakers'].apply(lambda x: x['needs_rebase'])]
    
    ax.scatter(halt_points['epoch'], halt_points['liquidity_ratio'], 
              color='red', marker='x', s=100, label='Trading Halt')
    ax.scatter(emergency_points['epoch'], emergency_points['liquidity_ratio'],
              color='yellow', marker='s', s=100, label='Emergency Measures')
    ax.scatter(rebase_points['epoch'], rebase_points['liquidity_ratio'],
              color='purple', marker='^', s=100, label='Rebase Events')
    
    ax.set_title('Circuit Breaker Analysis')
    ax.legend()
    plt.tight_layout()
    plt.savefig(f"{report_dir}/circuit_breakers.png")
    plt.close()

def create_scenario_summary(df, scenario_name):
    """Create comprehensive summary statistics for a scenario"""
    summary = {
        'scenario_name': scenario_name,
        
        # Price Stability Metrics
        'price_mean': df['current_price'].mean(),
        'price_std': df['current_price'].std(),
        'price_max_deviation': abs(df['current_price'] - 1).max(),
        'price_stability_score': df['price_stability_index'].mean(),
        
        # Liquidity Metrics
        'liquidity_mean': df['liquidity_ratio'].mean(),
        'liquidity_min': df['liquidity_ratio'].min(),
        'liquidity_variance': df['liquidity_ratio'].var(),
        
        # Participant Metrics
        'validator_retention': (df['validator_count'].iloc[-1] / df['validator_count'].iloc[0]),
        'holder_retention': (df['holder_count'].iloc[-1] / df['holder_count'].iloc[0]),
        'avg_transaction_volume': df['transaction_volume'].mean(),
        
        # Network Health
        'network_utility_mean': df['network_utility_score'].mean(),
        'network_utility_min': df['network_utility_score'].min(),
        'settlement_rate': df['transaction_settlement_rate'].mean(),
        
        # Economic Impact
        'total_validator_rewards': df['daily_validator_reward_usdc'].sum(),
        'total_holder_costs': df['daily_holder_cost_usdc'].sum(),
        'avg_transaction_fee': df['transaction_fee_usdc'].mean(),
        'net_economic_impact': (df['daily_validator_reward_usdc'] - 
                              df['daily_holder_cost_usdc']).sum(),
        
        # Circuit Breaker Events
        'trading_halts': df['circuit_breakers'].apply(lambda x: x['halt_trading']).sum(),
        'emergency_measures': df['circuit_breakers'].apply(lambda x: x['emergency_spreads']).sum(),
        'rebase_events': df['circuit_breakers'].apply(lambda x: x['needs_rebase']).sum(),
        
        # Equilibrium Analysis
        'equilibrium_percentage': (df['is_equilibrium'].sum() / len(df)) * 100,
        'time_in_equilibrium': df['is_equilibrium'].sum() * df['epoch_duration'].mean(),
    }
    
    # Check against performance targets from precept
    summary.update({
        'meets_price_target': summary['price_max_deviation'] <= 0.02,
        'meets_liquidity_target': summary['liquidity_variance'] <= 0.1,
        'meets_retention_target': min(summary['validator_retention'], 
                                    summary['holder_retention']) >= 0.9,
        'meets_settlement_target': summary['settlement_rate'] >= 0.99
    })
    
    return summary

def plot_recovery_metrics(df, report_dir, scenario_name):
    """Plot recovery-specific metrics and analysis"""
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(15, 12))
    
    # 1. Price Recovery Trajectory
    ax1.plot(df['epoch'], df['current_price'], label='Price')
    ax1.axhline(y=1.0, color='r', linestyle='--', alpha=0.3, label='Target')
    ax1.fill_between(df['epoch'], 0.98, 1.02, color='g', alpha=0.1, label='Target Zone')
    ax1.set_title('Price Recovery Trajectory')
    ax1.legend()
    
    # 2. Liquidity Restoration
    ax2.plot(df['epoch'], df['liquidity_ratio'], label='Liquidity')
    ax2.axhline(y=0.8, color='g', linestyle='--', alpha=0.3, label='Target')
    ax2.set_title('Liquidity Restoration')
    ax2.legend()
    
    # 3. Participant Recovery
    ax3.plot(df['epoch'], df['validator_count'] / df['validator_count'].iloc[0], 
             label='Validator Retention')
    ax3.plot(df['epoch'], df['holder_count'] / df['holder_count'].iloc[0], 
             label='Holder Retention')
    ax3.axhline(y=0.9, color='r', linestyle='--', alpha=0.3, label='Min Target')
    ax3.set_title('Participant Retention')
    ax3.legend()
    
    # 4. Recovery Costs
    cumulative_costs = df['daily_holder_cost_usdc'].cumsum()
    cumulative_rewards = df['daily_validator_reward_usdc'].cumsum()
    ax4.plot(df['epoch'], cumulative_costs, label='Cumulative Holder Costs')
    ax4.plot(df['epoch'], cumulative_rewards, label='Cumulative Validator Rewards')
    ax4.plot(df['epoch'], cumulative_rewards - cumulative_costs, 
             label='Net Economic Impact', linestyle='--')
    ax4.set_title('Recovery Economics')
    ax4.legend()
    
    plt.tight_layout()
    plt.savefig(f"{report_dir}/{scenario_name}_recovery_analysis.png")
    plt.close()
    
    # Additional recovery metrics plot
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 10))
    
    # Recovery Speed Metrics
    ax1.plot(df['epoch'], df['recovery_time'], label='Est. Time to Recovery')
    ax1.plot(df['epoch'], df['convergence_rate'], label='Convergence Rate')
    ax1.set_title('Recovery Speed Metrics')
    ax1.legend()
    
    # System Health During Recovery
    ax2.plot(df['epoch'], df['network_utility_score'], label='Network Utility')
    ax2.plot(df['epoch'], df['price_stability_index'], label='Price Stability')
    ax2.set_title('System Health During Recovery')
    ax2.legend()
    
    plt.tight_layout()
    plt.savefig(f"{report_dir}/{scenario_name}_recovery_metrics.png")
    plt.close()
    
def create_analysis_report(scenarios_results, analysis, report_dir, scenario_name):
    """Generate comprehensive analysis report with time series visualizations"""
    
    # Create report directory and ensure parent reports directory exists
    os.makedirs(report_dir, exist_ok=True)
    os.makedirs("reports", exist_ok=True)
    
    # Convert results to DataFrame
    df_scenario = pd.DataFrame(scenarios_results)
    
    # Generate plots
    plot_stability_metrics(df_scenario, report_dir, scenario_name)
    plot_economic_metrics(df_scenario, report_dir, scenario_name)
    plot_network_metrics(df_scenario, report_dir, scenario_name)
    plot_temporal_analysis(df_scenario, report_dir, scenario_name)
    plot_participant_dynamics(df_scenario, report_dir, scenario_name)
    plot_circuit_breaker_analysis(df_scenario, report_dir, scenario_name)
    
    # Create summary statistics
    summary = create_scenario_summary(df_scenario, scenario_name)
    
    # Create scenario-specific HTML page
    create_scenario_page(summary, report_dir)
    
    # Update main index with new scenario
    update_main_index(summary)
    
    return summary

def create_scenario_page(summary, report_dir):
    """Create individual scenario HTML page"""
    scenario_html = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <title>{summary['scenario_name']} - Analysis</title>
        <link rel="stylesheet" href="../styles.css">
    </head>
    <body>
        <div class="content">
            <h1>{summary['scenario_name']}</h1>
            <div class="section">
                <h3>Summary Statistics</h3>
                <table>
                    <tr><td>Price Stability Score:</td><td>{summary['price_stability_score']:.4f}</td></tr>
                    <tr><td>Liquidity Mean:</td><td>{summary['liquidity_mean']:.4f}</td></tr>
                    <tr><td>Network Utility Mean:</td><td>{summary['network_utility_mean']:.4f}</td></tr>
                    <tr><td>Validator Retention:</td><td>{summary['validator_retention']:.2%}</td></tr>
                    <tr><td>Holder Retention:</td><td>{summary['holder_retention']:.2%}</td></tr>
                </table>
            </div>
            
            <div class="section">
                <h3>Temporal Analysis</h3>
                <img src="temporal_analysis.png" alt="Temporal Analysis">
                
                <h3>Participant Dynamics</h3>
                <img src="participant_dynamics.png" alt="Participant Dynamics">
                
                <h3>Circuit Breaker Analysis</h3>
                <img src="circuit_breakers.png" alt="Circuit Breakers">
                
                <h3>Stability Metrics</h3>
                <img src="stability_metrics.png" alt="Stability Metrics">
                
                <h3>Economic Metrics</h3>
                <img src="economics_impacts.png" alt="Economic Metrics">
                
                <h3>Network Metrics</h3>
                <img src="network_health.png" alt="Network Metrics">
            </div>
        </div>
    </body>
    </html>
    """
    
    with open(f"{report_dir}/index.html", 'w') as f:
        f.write(scenario_html)

def update_main_index(summary):
    """Update the main index.html with new scenario"""
    index_path = "reports/index.html"
    styles_path = "reports/styles.css"
    
    # Create main index if it doesn't exist
    if not os.path.exists(index_path):
        create_main_index()
        create_styles_file()
    
    with open(index_path, 'r') as f:
        content = f.read()
    
    # Add new nav item
    nav_item = f'<div class="nav-item" onclick="window.location.href=\'{summary["scenario_name"].replace(" ", "_").lower()}/index.html\'">{summary["scenario_name"]}</div>'
    content = content.replace('</div><!-- nav-menu-end -->', f'{nav_item}\n</div><!-- nav-menu-end -->')
    
    # Add summary to overview section
    overview_item = f"""
    <div class="scenario-card">
        <h3>{summary['scenario_name']}</h3>
        <table>
            <tr><td>Price Stability:</td><td>{summary['price_stability_score']:.4f}</td></tr>
            <tr><td>Liquidity Mean:</td><td>{summary['liquidity_mean']:.4f}</td></tr>
            <tr><td>Network Utility:</td><td>{summary['network_utility_mean']:.4f}</td></tr>
        </table>
        <a href="{summary['scenario_name'].replace(' ', '_').lower()}/index.html">View Details</a>
    </div>
    """
    content = content.replace('</div><!-- overview-content-end -->', f'{overview_item}\n</div><!-- overview-content-end -->')
    
    with open(index_path, 'w') as f:
        f.write(content)

def create_main_index():
    """Create the main index.html file"""
    html = """
    <!DOCTYPE html>
    <html>
    <head>
        <title>Stability Analysis Reports</title>
        <link rel="stylesheet" href="styles.css">
    </head>
    <body>
        <div class="nav-menu">
            <div class="nav-item active" onclick="window.location.href='#'">Overview</div>
        </div><!-- nav-menu-end -->
        
        <div class="content">
            <div class="overview-section">
                <h1>Stability Analysis Reports</h1>
                <h2>Generated: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}</h2>
                <div class="scenarios-grid">
                </div>
            </div><!-- overview-content-end -->
        </div>
    </body>
    </html>
    """
    
    with open("reports/index.html", 'w') as f:
        f.write(html)

def create_styles_file():
    """Create the shared CSS styles file"""

    if os.path.exists("reports/styles.css"):
        return

    css = """
    body {
        font-family: Arial, sans-serif;
        margin: 0;
        padding: 0;
        background-color: #f5f5f5;
    }
    
    .nav-menu {
        position: fixed;
        width: 200px;
        height: 100%;
        background: #333;
        padding: 20px 0;
        color: white;
    }
    
    .content {
        margin-left: 220px;
        padding: 20px;
    }
    
    .nav-item {
        padding: 10px 20px;
        cursor: pointer;
        transition: background 0.3s;
        color: white;
        text-decoration: none;
    }
    
    .nav-item:hover {
        background: #444;
    }
    
    .nav-item.active {
        background: #555;
    }
    
    .scenario-card {
        background: white;
        padding: 20px;
        margin: 10px;
        border-radius: 5px;
        box-shadow: 0 2px 5px rgba(0,0,0,0.1);
    }
    
    .scenarios-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
        gap: 20px;
        padding: 20px 0;
    }
    
    table {
        width: 100%;
        margin: 10px 0;
    }
    
    td {
        padding: 5px;
    }
    
    img {
        max-width: 100%;
        height: auto;
        margin: 10px 0;
        border: 1px solid #ddd;
        border-radius: 4px;
    }
    
    .section {
        margin-bottom: 30px;
    }
    """
    
    with open("reports/styles.css", 'w') as f:
        f.write(css)

def plot_stability_metrics(df, report_dir, scenario_name):
    """Plot core stability metrics over time"""
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 10))
    
    # Price and Stability Metrics
    ax1.plot(df['epoch'], df['current_price'], label='Price')
    ax1.plot(df['epoch'], df['price_stability_index'], label='Price Stability')
    ax1.axhline(y=1.0, color='r', linestyle='--', alpha=0.3)
    ax1.set_title('Price and Stability Over Time')
    ax1.legend()
    
    # Liquidity and Market Pressure
    ax2.plot(df['epoch'], df['liquidity_ratio'], label='Liquidity')
    ax2.plot(df['epoch'], df['market_pressure'], label='Market Pressure')
    ax2.set_title('Liquidity and Market Pressure')
    ax2.legend()
    
    plt.tight_layout()
    plt.savefig(f"{report_dir}/stability_metrics.png")
    plt.close()

def plot_economic_metrics(df, report_dir, scenario_name):
    """Plot economic metrics over time"""
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 10))
    
    # Validator and Holder Economics
    ax1.plot(df['epoch'], df['daily_validator_reward_usdc'], label='Validator Rewards')
    ax1.plot(df['epoch'], df['daily_holder_cost_usdc'], label='Holder Costs')
    ax1.plot(df['epoch'], df['validator_holder_net_usdc'], label='Net Position')
    ax1.set_title('Economic Impacts Over Time')
    ax1.legend()
    
    # Transaction Costs
    ax2.plot(df['epoch'], df['transaction_fee_usdc'], label='Transaction Fees')
    ax2.plot(df['epoch'], df['dynamic_spread'], label='Market Spread')
    ax2.set_title('Transaction Costs Over Time')
    ax2.legend()
    
    plt.tight_layout()
    plt.savefig(f"{report_dir}/economics_impacts.png")
    plt.close()

def plot_network_metrics(df, report_dir, scenario_name):
    """Plot network health metrics over time"""
    fig, ax = plt.subplots(figsize=(12, 6))
    
    ax.plot(df['epoch'], df['network_utility_score'], label='Network Utility')
    ax.plot(df['epoch'], df['validator_participation'], label='Validator Participation')
    ax.plot(df['epoch'], df['holder_participation'], label='Holder Participation')
    
    ax.set_title('Network Health Metrics')
    ax.legend()
    plt.tight_layout()
    plt.savefig(f"{report_dir}/network_health.png")
    plt.close()

def create_summary_statistics(scenarios_results):
    """Generate summary statistics for all scenarios"""
    summaries = []
    
    for scenario in scenarios_results:
        df = pd.DataFrame(scenario['epoch_results'])
        
        summary = {
            'scenario_name': scenario['name'],
            'mean_price': df['current_price'].mean(),
            'price_volatility': df['current_price'].std(),
            'min_liquidity': df['liquidity_ratio'].min(),
            'avg_network_utility': df['network_utility_score'].mean(),
            'total_validator_rewards': df['daily_validator_reward_usdc'].sum(),
            'total_holder_costs': df['daily_holder_cost_usdc'].sum(),
            'avg_transaction_fee': df['transaction_fee_usdc'].mean(),
            'equilibrium_percentage': (df['is_equilibrium'].sum() / len(df)) * 100
        }
        
        # Add recovery metrics if applicable
        if 'Recovery' in scenario['name']:
            summary.update({
                'time_to_recovery': df['recovery_time'].min(),
                'final_stability': df['price_stability_index'].iloc[-1]
            })
            
        summaries.append(summary)
    
    return pd.DataFrame(summaries)

def print_scenario_results(description, results):
    """Print formatted scenario results to console for debugging"""
    print("\n" + "=" * 80)
    print(f"Scenario: {description}")
    print("=" * 80 + "\n")

    print("Core Stability Metrics:")
    print(f"Price Stability Index: {results['price_stability_index']:.4f}")
    print(f"Network Utility Score: {results['network_utility_score']:.4f}")
    print(f"Liquidity Health Index: {results['liquidity_health_index']:.4f}\n")

    print("Market Conditions:")
    print(f"Market Pressure: {results['market_pressure']:.4f}")
    print(f"Convergence Rate: {results['convergence_rate']:.4f}")
    print(f"Dynamic Spread: {results['dynamic_spread']:.4f}\n")

    print("Economic Impacts:")
    print(f"Validator Daily Reward: {results['daily_validator_reward_usdc']:.6f} USDC")
    print(f"Holder Daily Cost: {results['daily_holder_cost_usdc']:.6f} USDC")
    print(f"Validator Net Position: {results['validator_holder_net_usdc']:.6f} USDC")
    print(f"Transaction Fee: {results['transaction_fee_usdc']:.6f} USDC\n")

    print("System State:")
    breakers = results['circuit_breakers']
    print(f"Circuit Breakers: [Halt: {breakers['halt_trading']}, "
          f"Emergency: {breakers['emergency_spreads']}, "
          f"Rebase: {breakers['needs_rebase']}]")
    
    equilibrium = results['equilibrium']
    print(f"Equilibrium: {equilibrium['is_equilibrium']}")
    if not equilibrium['is_equilibrium']:
        print(f"Failing Metrics: {', '.join(equilibrium['failing_metrics'])}")
    print()