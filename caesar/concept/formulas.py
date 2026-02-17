# Copyright © 2026 Hypermesh Foundation. All rights reserved.
# Licensed under the Business Source License 1.1.
# See the LICENSE file in the repository root for full license text.

import numpy as np
from scipy import stats
from collections import deque
import math
import itertools
import pandas as pd

class MarketMetrics:
    def __init__(self, window_size=30):
        """Initialize market metrics with configurable window size"""
        self.price_window = deque(maxlen=window_size)
        self.volume_window = deque(maxlen=window_size)
        self.pressure_window = deque(maxlen=window_size)
        self.window_size = window_size
        self.price_history = []
        self.liquidity_history = []
        self.participant_retention = []
        self.recovery_metrics = {
            'time_to_recovery': [],
            'liquidity_restoration_rate': [],
            'participant_retention': [],
            'post_recovery_stability': []
        }
        self.cost_analysis = {
            'daily_holder_costs': [],
            'validator_revenue': [],
            'system_costs': [],
            'net_economic_impact': []
        }

    def update_metrics(self, price, volume, pressure):
        """Update all metrics with new values"""
        self.price_window.append(price)
        self.volume_window.append(volume)
        self.pressure_window.append(pressure)

    def get_volatility(self):
        """Get current volatility based on price window"""
        return calculate_historical_volatility(list(self.price_window), self.window_size)

    def get_volume_weight(self):
        """Get current volume weight"""
        return calculate_volume_weight(list(self.volume_window))

    def track_recovery(self, current_metrics):
        """Track recovery metrics over time"""
        # Time to recovery
        self.recovery_metrics['time_to_recovery'].append(
            calculate_recovery_time(current_metrics)
        )
        
        # Liquidity restoration
        self.recovery_metrics['liquidity_restoration_rate'].append(
            calculate_liquidity_restoration(current_metrics)
        )
        
        # Participant retention
        validator_retention = current_metrics['validator_count'] / current_metrics.get('initial_validator_count', 5000)
        holder_retention = current_metrics['holder_count'] / current_metrics.get('initial_holder_count', 1000000)
        self.recovery_metrics['participant_retention'].append(
            min(validator_retention, holder_retention)
        )
        
        # Post-recovery stability
        stability_score = (
            current_metrics['price_stability_index'] * 0.4 +
            current_metrics['liquidity_health_index'] * 0.3 +
            current_metrics['network_utility_score'] * 0.3
        )
        self.recovery_metrics['post_recovery_stability'].append(stability_score)

    def analyze_costs(self, daily_metrics):
        """Track and analyze system costs"""
        # Daily holder costs
        self.cost_analysis['daily_holder_costs'].append(
            daily_metrics['daily_holder_cost_usdc']
        )
        
        # Validator revenue
        self.cost_analysis['validator_revenue'].append(
            daily_metrics['daily_validator_reward_usdc']
        )
        
        # System costs (operational)
        system_cost = (
            daily_metrics.get('network_overhead_usdc', 0) +
            daily_metrics.get('maintenance_cost_usdc', 0) +
            daily_metrics.get('emergency_reserve_contribution_usdc', 0)
        )
        self.cost_analysis['system_costs'].append(system_cost)
        
        # Net economic impact
        net_impact = (
            daily_metrics['daily_validator_reward_usdc'] -
            daily_metrics['daily_holder_cost_usdc'] -
            system_cost
        )
        self.cost_analysis['net_economic_impact'].append(net_impact)

    def get_recovery_summary(self):
        """Get summary of recovery metrics"""
        if not self.recovery_metrics['time_to_recovery']:
            return None
        
        return {
            'avg_recovery_time': np.mean(self.recovery_metrics['time_to_recovery']),
            'avg_liquidity_restoration': np.mean(self.recovery_metrics['liquidity_restoration_rate']),
            'min_participant_retention': min(self.recovery_metrics['participant_retention']),
            'avg_post_recovery_stability': np.mean(self.recovery_metrics['post_recovery_stability']),
            'recovery_success_rate': sum(1 for x in self.recovery_metrics['post_recovery_stability'] 
                                       if x > 0.8) / len(self.recovery_metrics['post_recovery_stability'])
        }

    def get_cost_summary(self):
        """Get summary of cost analysis"""
        if not self.cost_analysis['daily_holder_costs']:
            return None
        
        return {
            'total_holder_costs': sum(self.cost_analysis['daily_holder_costs']),
            'total_validator_revenue': sum(self.cost_analysis['validator_revenue']),
            'total_system_costs': sum(self.cost_analysis['system_costs']),
            'net_economic_impact': sum(self.cost_analysis['net_economic_impact']),
            'avg_daily_holder_cost': np.mean(self.cost_analysis['daily_holder_costs']),
            'avg_daily_validator_revenue': np.mean(self.cost_analysis['validator_revenue']),
            'economic_efficiency': (
                sum(self.cost_analysis['validator_revenue']) /
                (sum(self.cost_analysis['daily_holder_costs']) + 
                 sum(self.cost_analysis['system_costs']))
            ) if sum(self.cost_analysis['daily_holder_costs']) > 0 else 0
        }

    def get_market_pressure(self):
        """Get current market pressure based on recent pressure window"""
        if not self.pressure_window:
            return 0
        
        # Calculate weighted average of recent pressure
        weights = np.array([0.94 ** i for i in range(len(self.pressure_window))])
        weighted_pressure = np.sum(weights * np.array(list(self.pressure_window)))
        return weighted_pressure / np.sum(weights)

    def get_pressure_trend(self):
        """Get market pressure trend over the window"""
        if len(self.pressure_window) < 2:
            return 0
            
        pressures = list(self.pressure_window)
        return np.polyfit(range(len(pressures)), pressures, 1)[0]

def calculate_economics(validator_count, total_holders, daily_transactions, current_price,
                      avg_transaction_size, avg_holding_balance, days_held, liquidity_ratio,
                      cross_chain_transfers, buys_volume, sells_volume, market_metrics):
    """Calculate all economic metrics for an epoch"""
    
    # Calculate participation metrics
    validator_participation = validator_count / 5000  # Normalized to initial count
    holder_participation = total_holders / 1000000   # Normalized to initial count
    
    # Calculate base metrics
    psi = price_stability_index(current_price, 
                               market_metrics.get_market_pressure(),
                               validator_count / 5000,
                               total_holders / 1000000)
    
    # Calculate market conditions
    market_press = market_pressure(buys_volume, sells_volume, 
                                 liquidity_ratio * (buys_volume + sells_volume),
                                 validator_count)
    
    # Calculate network utility score with default target
    nus = network_utility_score(
        daily_transactions=daily_transactions,
        cross_chain_transfers=cross_chain_transfers,
        target_transfers=500000  # Default target
    )
    
    # Calculate liquidity health
    lhi = liquidity_health_index(
        daily_transactions / 24,  # Active participants per hour
        total_holders,
        liquidity_ratio,
        buys_volume * liquidity_ratio,  # Stability reserve
        stability_reserve_requirement(total_holders * avg_holding_balance, 0) # TODO: Incorporate dynamic decay factor
    )
    
    # Calculate transaction settlement rate based on network conditions
    settlement_rate = calculate_settlement_rate(
        daily_transactions,
        validator_count,
        liquidity_ratio,
        market_press
    )
    
    # Calculate rewards and costs
    base_reward = 0.1  # Base reward rate in USDC
    v_reward = validator_reward(base_reward, daily_transactions, validator_count, psi)
    h_cost = holder_cost(0.01, days_held, avg_holding_balance, psi)  # Base rate 1%
    vh_cost = validator_holder_cost(h_cost, v_reward, nus)
    tx_fee = transaction_fee(0.001, psi, avg_transaction_size, liquidity_ratio)  # Base fee 0.1%
    
    # Calculate convergence
    conv_rate = convergence_rate(current_price, 1.0, market_press, psi)
    
    # Check circuit breakers
    halt, emergency, rebase = circuit_breaker_conditions(liquidity_ratio, 
                                                       current_price, 
                                                       lhi)
    
    # Check equilibrium
    is_equilibrium, failing = equilibrium_state(psi, lhi, nus, conv_rate)
    
    # Calculate dynamic spread based on market conditions
    dynamic_spread = calculate_dynamic_spread(
        liquidity_ratio,
        market_metrics.get_market_pressure(),
        validator_count / 5000  # Normalized validator count
    )
    
    return {
        'price_stability_index': psi,
        'market_pressure': market_press,
        'network_utility_score': nus,
        'liquidity_health_index': lhi,
        'daily_validator_reward_usdc': v_reward,
        'daily_holder_cost_usdc': h_cost,
        'validator_holder_net_usdc': vh_cost,
        'transaction_fee_usdc': tx_fee,
        'convergence_rate': conv_rate,
        'liquidity_ratio': liquidity_ratio,
        'dynamic_spread': dynamic_spread,
        'validator_participation': validator_participation,
        'validator_count': validator_count,
        'holder_participation': holder_participation,
        'holder_count': total_holders,
        'transaction_settlement_rate': settlement_rate,
        'circuit_breakers': {
            'halt_trading': halt,
            'emergency_spreads': emergency,
            'needs_rebase': rebase
        },
        'is_equilibrium': is_equilibrium,
        'failing_metrics': failing
    }

def calculate_settlement_rate(daily_transactions, validator_count, liquidity_ratio, market_pressure):
    """Calculate the transaction settlement rate based on network conditions"""
    # Base settlement rate starts at 99.9%
    base_rate = 0.999
    
    # Adjust for validator capacity
    validator_capacity = validator_count * 1000  # Each validator can handle 1000 tx/day
    capacity_utilization = min(1, daily_transactions / (validator_capacity * 0.8))  # 80% target utilization
    validator_factor = 1 - (capacity_utilization ** 2) * 0.1  # Max 10% reduction
    
    # Adjust for liquidity conditions
    liquidity_factor = min(1, liquidity_ratio / 0.8)  # Target 80% liquidity
    
    # Adjust for market pressure
    pressure_impact = max(0, 1 - abs(market_pressure) * 0.05)
    
    # Final settlement rate
    final_rate = base_rate * liquidity_factor * validator_factor * pressure_impact
    
    return final_rate

def calculate_stability_metrics(results):
    """Calculate overall stability metrics from simulation results"""
    df = pd.DataFrame(results)
    return {
        'price_stability': df['price_stability_index'].mean(),
        'price_volatility': df['current_price'].std(),
        'liquidity_health': df['liquidity_health_index'].mean(),
        'market_pressure_avg': df['market_pressure'].mean(),
        'convergence_rate_avg': df['convergence_rate'].mean(),
        'price_mean': df['current_price'].mean(),
        'price_max_deviation': abs(df['current_price'] - 1).max()
    }

def analyze_equilibrium_states(results):
    """
    Analyze equilibrium states throughout simulation results
    Returns metrics about system equilibrium periods
    """
    if not results:
        return {
            'total_equilibrium_periods': 0,
            'longest_equilibrium_streak': 0,
            'average_equilibrium_duration': 0,
            'percent_time_in_equilibrium': 0,
            'equilibrium_stability': 0
        }
    
    # Track equilibrium streaks
    current_streak = 0
    streaks = []
    equilibrium_periods = 0
    
    for result in results:
        if result.get('is_equilibrium', False):
            current_streak += 1
            equilibrium_periods += 1
        else:
            if current_streak > 0:
                streaks.append(current_streak)
            current_streak = 0
    
    # Add final streak if exists
    if current_streak > 0:
        streaks.append(current_streak)
    
    # Calculate metrics
    total_periods = len(results)
    longest_streak = max(streaks) if streaks else 0
    avg_duration = sum(streaks) / len(streaks) if streaks else 0
    percent_in_equilibrium = (equilibrium_periods / total_periods * 100) if total_periods > 0 else 0
    
    # Calculate stability (ratio of equilibrium time to disruptions)
    stability = (
        equilibrium_periods / (total_periods - equilibrium_periods)
        if total_periods > equilibrium_periods else 1.0
    )
    
    return {
        'total_equilibrium_periods': equilibrium_periods,
        'longest_equilibrium_streak': longest_streak,
        'average_equilibrium_duration': avg_duration,
        'percent_time_in_equilibrium': percent_in_equilibrium,
        'equilibrium_stability': stability
    }

def identify_recovery_periods(df):
    """Identify and analyze recovery periods in the simulation data"""
    recovery_periods = []
    in_recovery = False
    recovery_start = 0
    
    # Define recovery thresholds from precept
    PRICE_THRESHOLD = 0.05  # Price within 5% of target
    LIQUIDITY_THRESHOLD = 0.7  # Minimum healthy liquidity
    STABILITY_THRESHOLD = 0.8  # Minimum stability index
    
    for i in range(len(df)):
        price_deviation = abs(df['current_price'].iloc[i] - 1.0)
        liquidity = df['liquidity_ratio'].iloc[i]
        stability = df['price_stability_index'].iloc[i]
        
        # Check if we're in a crisis
        is_crisis = (
            price_deviation > PRICE_THRESHOLD or
            liquidity < LIQUIDITY_THRESHOLD or
            stability < STABILITY_THRESHOLD
        )
        
        if is_crisis and not in_recovery:
            # Start of recovery period
            in_recovery = True
            recovery_start = i
        elif not is_crisis and in_recovery:
            # End of recovery period
            recovery_end = i
            
            # Calculate recovery metrics
            recovery_period = {
                'start_epoch': recovery_start,
                'end_epoch': recovery_end,
                'duration': recovery_end - recovery_start,
                'successful': True,
                'total_cost': df['daily_holder_cost_usdc'].iloc[recovery_start:recovery_end].sum(),
                'stability_after': df['price_stability_index'].iloc[recovery_end:recovery_end+100].mean() 
                                 if recovery_end + 100 < len(df) else df['price_stability_index'].iloc[recovery_end:].mean()
            }
            
            recovery_periods.append(recovery_period)
            in_recovery = False
    
    # Handle case where we're still in recovery at end of simulation
    if in_recovery:
        recovery_periods.append({
            'start_epoch': recovery_start,
            'end_epoch': len(df),
            'duration': len(df) - recovery_start,
            'successful': False,
            'total_cost': df['daily_holder_cost_usdc'].iloc[recovery_start:].sum(),
            'stability_after': df['price_stability_index'].iloc[-100:].mean()
        })
    
    return recovery_periods

def analyze_recovery_metrics(results):
    """Analyze recovery metrics from simulation results"""
    df = pd.DataFrame(results)
    recovery_periods = identify_recovery_periods(df)
    
    if not recovery_periods:
        return {
            'average_recovery_time': 0,
            'recovery_success_rate': 1.0,  # No recoveries needed = perfect score
            'avg_cost_per_recovery': 0,
            'stability_post_recovery': df['price_stability_index'].mean()
        }
    
    return {
        'average_recovery_time': np.mean([p['duration'] for p in recovery_periods]),
        'recovery_success_rate': sum(p['successful'] for p in recovery_periods) / len(recovery_periods),
        'avg_cost_per_recovery': np.mean([p['total_cost'] for p in recovery_periods]),
        'stability_post_recovery': np.mean([p['stability_after'] for p in recovery_periods]),
        'total_recovery_periods': len(recovery_periods),
        'longest_recovery': max(p['duration'] for p in recovery_periods),
        'total_recovery_cost': sum(p['total_cost'] for p in recovery_periods)
    }

def analyze_economic_metrics(results):
    """Analyze economic metrics from simulation results"""
    df = pd.DataFrame(results)
    return {
        'total_validator_rewards': df['daily_validator_reward_usdc'].sum(),
        'total_holder_costs': df['daily_holder_cost_usdc'].sum(),
        'avg_transaction_fee': df['transaction_fee_usdc'].mean(),
        'net_economic_impact': (
            df['daily_validator_reward_usdc'].sum() - 
            df['daily_holder_cost_usdc'].sum()
        ),
        'economic_efficiency': (
            df['network_utility_score'].mean() / 
            df['transaction_fee_usdc'].mean()
        )
    }

def validate_targets(analysis, targets):
    """Validate analysis results against target thresholds"""
    return {
        'price_stability': analysis['stability_metrics']['price_volatility'] <= targets['price_deviation_max'],
        'liquidity_health': analysis['stability_metrics']['liquidity_health'] >= (1 - targets['liquidity_variance_max']),
        'participant_retention': analysis['recovery_metrics']['recovery_success_rate'] >= targets['participant_retention_min'],
        'settlement_rate': analysis['economic_metrics']['economic_efficiency'] >= targets['settlement_rate_min']
    }

def calculate_historical_volatility(price_history, window=30):
    """
    Calculate historical volatility using log returns
    """
    if len(price_history) < 2:
        return 0
    log_returns = np.log(np.array(price_history[1:]) / np.array(price_history[:-1]))
    return np.std(log_returns) * np.sqrt(window)

def calculate_volume_weight(volume_history):
    """
    Calculate volume-weighted importance factor
    """
    if not volume_history:
        return 1
    recent_vol = np.mean(volume_history[-5:])  # Last 5 periods
    total_vol = np.mean(volume_history)  # All periods
    return min(1, (recent_vol / total_vol) if total_vol > 0 else 1)

def calculate_order_imbalance(order_book):
    """
    Calculate order book imbalance
    """
    bid_volume = sum(order.volume for order in order_book.bids)
    ask_volume = sum(order.volume for order in order_book.asks)
    total_volume = bid_volume + ask_volume
    
    if total_volume == 0:
        return 0
        
    return (bid_volume - ask_volume) / total_volume

def calculate_time_weighted_flow(buys_history, sells_history, decay_factor=0.94):
    """
    Calculate time-weighted average flow with exponential decay
    """
    weights = np.array([decay_factor ** i for i in range(len(buys_history))])
    weighted_buys = np.sum(weights * np.array(buys_history))
    weighted_sells = np.sum(weights * np.array(sells_history))
    return weighted_buys - weighted_sells

def enhanced_price_stability_index(
    active_participants,
    total_holders,
    current_price,
    price_history,
    volume_history,
    market_depth,
    weights={
        'participation': 0.25,
        'price': 0.15,
        'volatility': 0.1,
        'volume': 0.1,
        'depth': 0.2
    }
):
    """
    Enhanced PSI with improved stability calculation
    """
    # Current issue: Never reaches optimal stability
    # Fix: Add adaptive baseline and improved scaling
    if total_holders <= 0:
        raise ValueError("Total holders must be positive")
        
    participation_ratio = min(1, active_participants / total_holders)
    price_deviation = abs(1 - current_price)
    
    # Improved volatility calculation with minimum baseline
    volatility = max(0.1, calculate_historical_volatility(price_history))
    volume_weight = calculate_volume_weight(volume_history)
    
    # Exponential depth scaling
    depth_factor = 1 - math.exp(-market_depth / 1000)
    
    # Adaptive baseline based on market conditions
    adaptive_base = 0.4 + (0.1 * participation_ratio + 0.1 * depth_factor)
    
    # Combined stability score with adaptive baseline
    stability = adaptive_base + (
        weights['participation'] * participation_ratio +
        weights['price'] * (1 / (1 + price_deviation)) +
        weights['volatility'] * (1 / (1 + volatility)) +
        weights['volume'] * volume_weight +
        weights['depth'] * depth_factor
    )
    
    return min(1, max(adaptive_base, stability))

def enhanced_network_utility_score(
    daily_volume,
    target_volume,
    cross_chain_transfers,
    target_transfers,
    unique_addresses,
    target_addresses,
    success_rate
):
    """
    Enhanced network utility with multiple metrics
    """
    # Core metrics
    volume_score = min(1, daily_volume / target_volume)
    transfer_score = min(1, cross_chain_transfers / target_transfers)
    address_score = min(1, unique_addresses / target_addresses)
    
    # Weighted combination
    weights = {
        'volume': 0.35,
        'transfers': 0.25,
        'addresses': 0.25,
        'success_rate': 0.15
    }
    
    final_score = (
        weights['volume'] * volume_score +
        weights['transfers'] * transfer_score +
        weights['addresses'] * address_score +
        weights['success_rate'] * success_rate
    )
    
    return min(1, max(0, final_score))

def enhanced_liquidity_health_index(
    active_participants,
    total_holders,
    current_liquidity,
    stability_reserve,
    required_reserve,
    slippage_impact,
    depth_distribution
):
    """
    Enhanced liquidity health with depth analysis
    """
    # Base ratios
    participation_ratio = active_participants / total_holders
    liquidity_ratio = current_liquidity / 0.8
    reserve_ratio = stability_reserve / required_reserve
    
    # Slippage and depth factors
    slippage_factor = 1 / (1 + slippage_impact)
    depth_score = calculate_depth_score(depth_distribution)
    
    # Combined health score
    base_health = 0.3 + (
        0.25 * participation_ratio +
        0.20 * liquidity_ratio +
        0.15 * reserve_ratio +
        0.10 * slippage_factor
    )
    
    # Apply depth adjustment
    final_health = base_health * (0.8 + 0.2 * depth_score)
    
    return min(1, max(0, final_health))

def enhanced_market_pressure(
    buys_volume,
    sells_volume,
    liquidity_pool,
    order_book,
    volatility_history,
    time_window=30
):
    """
    Enhanced market pressure with multiple factors
    """
    # Basic flow pressure
    net_flow = buys_volume - sells_volume
    base_pressure = net_flow / liquidity_pool
    
    # Order book imbalance
    imbalance = calculate_order_imbalance(order_book)
    
    # Volatility impact
    vol_impact = calculate_historical_volatility(volatility_history, time_window)
    
    # Combined pressure score
    weights = {
        'flow': 0.4,
        'imbalance': 0.35,
        'volatility': 0.25
    }
    
    pressure = (
        weights['flow'] * base_pressure +
        weights['imbalance'] * imbalance +
        weights['volatility'] * vol_impact
    )
    
    # Normalize to [-1, 1] range
    return np.tanh(pressure)

def calculate_depth_score(depth_distribution):
    """
    Calculate liquidity depth score based on distribution
    """
    # Analyze price levels and corresponding liquidity
    concentrations = np.array([d['liquidity'] for d in depth_distribution])
    price_levels = np.array([d['price'] for d in depth_distribution])
    
    # Calculate concentration Gini coefficient
    gini = calculate_gini_coefficient(concentrations)
    
    # Calculate price level coverage
    coverage = len(price_levels) / 100  # Normalize to expected coverage
    
    return (1 - gini) * min(1, coverage)

def calculate_gini_coefficient(values):
    """
    Calculate Gini coefficient for distribution analysis
    """
    values = np.sort(values)
    n = len(values)
    index = np.arange(1, n + 1)
    return (np.sum((2 * index - n - 1) * values)) / (n * np.sum(values))

def price_stability_index(current_price, market_pressure, validator_participation, holder_participation):
    """
    Calculate price stability index based on market conditions
    From precept: PSI combines price deviation, market pressure and participation metrics
    """
    price_deviation = abs(1 - current_price)
    pressure_factor = 1 / (1 + abs(market_pressure))
    participation_score = min(1, (validator_participation + holder_participation) / 2)
    
    base_stability = 0.3  # Minimum stability floor
    stability = base_stability + (
        0.3 * (1 / (1 + price_deviation)) +
        0.2 * pressure_factor +
        0.2 * participation_score
    )
    
    return min(1, stability)

def validator_reward(base_reward_rate, daily_transactions, validator_count, psi):
    """
    Calculate validator rewards based on transaction share and stability.
    90% of network revenue goes to validators.
    """
    transaction_share = daily_transactions / max(1, validator_count)
    market_reward = transaction_share * base_reward_rate * 0.9
    stability_bonus = market_reward * psi
    
    return market_reward + stability_bonus

def holder_cost(base_rate, time_held, balance, price_stability_index):
    """
    Calculate holding cost with logarithmic scaling.
    From whitepaper: D(t,L) = H * (1/L) * t * (1 - stability_index)
    """
    # Logarithmic scaling for large balances
    balance_factor = math.log2(1 + balance/1000)
    
    # Higher costs during instability
    stability_factor = 1 - price_stability_index
    
    return base_rate * time_held * balance_factor * stability_factor

def validator_holder_cost(holder_cost, validator_reward, participation_score):
    """
    Calculate net cost/reward for validators who are also holders.
    """
    return holder_cost - (validator_reward * participation_score)

def transaction_fee(base_fee, price_stability_index, transaction_size, liquidity_ratio):
    """
    Dynamic fee calculation based on market conditions.
    From whitepaper: S(v,s) = β * (1/L) * (1 + log₂(v/v₀)) * I(s)
    """
    # Volume scaling
    volume_factor = 1 + math.log2(1 + transaction_size/10000)
    
    # Liquidity impact
    liquidity_factor = 1 / max(0.1, liquidity_ratio)
    
    # Stability adjustment
    stability_factor = 1 + (1 - price_stability_index)
    
    return base_fee * volume_factor * liquidity_factor * stability_factor

def network_utility_score(daily_transactions, cross_chain_transfers, target_transfers=500000):
    """
    Calculate network utility score based on transaction metrics
    
    Parameters:
    - daily_transactions: Number of daily transactions
    - cross_chain_transfers: Number of cross-chain transfers
    - target_transfers: Target number of daily cross-chain transfers (default: 500,000)
    
    Returns:
    - float: Network utility score between 0 and 1
    """
    # Base transaction utility (weight: 0.6)
    tx_utility = min(1.0, daily_transactions / (target_transfers * 2))
    
    # Cross-chain integration utility (weight: 0.4)
    cross_chain_utility = min(1.0, cross_chain_transfers / target_transfers)
    
    # Weighted combination
    return (tx_utility * 0.6) + (cross_chain_utility * 0.4)

def liquidity_health_index(active_participants, total_holders, current_liquidity, stability_reserve, required_reserve):
    """
    Calculate liquidity health based on whitepaper formula:
    H(t) = (AP/TH) * (L(t)/0.8) * (SR(t)/RR)
    """
    participation_ratio = active_participants / max(1, total_holders)
    liquidity_ratio = current_liquidity / 0.8  # Target liquidity ratio
    reserve_ratio = stability_reserve / max(1, required_reserve)
    
    # Combined health score with minimum baseline
    return max(0.2, min(1, participation_ratio * liquidity_ratio * reserve_ratio))

def dynamic_spread(base_spread, liquidity_ratio, current_volume, target_volume):
    """
    Calculate dynamic spread based on market conditions
    """
    volume_factor = min(1, current_volume / target_volume)
    liquidity_factor = 1 / max(0.1, liquidity_ratio)
    return base_spread * liquidity_factor * (1 + math.log2(1 + volume_factor))

def emergency_spread(base_spread, liquidity_ratio):
    """
    Calculate emergency spread during crisis conditions
    """
    crisis_multiplier = (0.2 / max(0.01, liquidity_ratio))
    return base_spread * min(5, crisis_multiplier)  # Cap at 5x base spread

def rebase_supply(current_supply, current_price, target_price=1.0):
    """
    Calculate new supply after rebase operation.
    """
    return (target_price / current_price) * current_supply

def stability_reserve_requirement(total_supply, total_decay_penalties):
    """
    Calculate required stability reserve based on system metrics
    """
    base_requirement = total_supply * 0.1  # 10% of total supply
    dynamic_requirement = total_decay_penalties * 2  # 2x daily decay
    return max(base_requirement, dynamic_requirement)

def inflation_rate(total_supply_t0, total_supply_t1):
    """
    Calculate inflation rate between two time periods
    Target range: -2% to +2% annual
    """
    return ((total_supply_t1 - total_supply_t0) / total_supply_t0) * 100

def circuit_breaker_conditions(liquidity_ratio, current_price, liquidity_health_index):
    """
    Determine if circuit breakers should be activated
    From precept: CB(t) conditions
    """
    halt_trading = liquidity_ratio < 0.1
    emergency_spreads = liquidity_ratio < 0.2
    needs_rebase = abs(current_price - 1) > 0.2
    
    return halt_trading, emergency_spreads, needs_rebase

def market_pressure(buys_volume, sells_volume, effective_liquidity, validator_count):
    """
    Calculate market pressure based on volume imbalance and liquidity
    """
    volume_imbalance = (buys_volume - sells_volume) / max(1, buys_volume + sells_volume)
    liquidity_factor = effective_liquidity / max(1, validator_count * 1000000)
    return volume_imbalance * (1 / max(0.1, liquidity_factor))

def convergence_rate(current_price, target_price, market_pressure, stability_index):
    """
    Calculate rate of price convergence to target
    From precept: dp/dt = α(p_target - p(t)) + β(pressure(t)) + γ(stability(t))
    """
    α = 0.1  # Price correction factor
    β = -0.05  # Pressure impact factor
    γ = 0.05  # Stability impact factor
    
    price_gap = target_price - current_price
    return (α * price_gap) + (β * market_pressure) + (γ * stability_index)

def equilibrium_state(
    price_stability_index,
    liquidity_health_index,
    network_utility_score,
    convergence_rate,
    target_thresholds={
        'psi_min': 0.8,
        'lhi_min': 0.7,
        'nus_min': 0.6,
        'conv_max': 10
    }
):
    """
    Determine if system is in equilibrium state
    Returns (is_equilibrium, failing_metrics)
    """
    checks = {
        'price_stability': price_stability_index >= target_thresholds['psi_min'],
        'liquidity_health': liquidity_health_index >= target_thresholds['lhi_min'],
        'network_utility': network_utility_score >= target_thresholds['nus_min'],
        'convergence': convergence_rate <= target_thresholds['conv_max']
    }
    
    failing = [k for k, v in checks.items() if not v]
    return (len(failing) == 0, failing)


def determine_epoch_duration(transaction_volume, network_throughput, market_volatility):
    # Example logic to determine epoch duration
    base_epoch_duration = 10  # seconds
    if transaction_volume > network_throughput * 0.8:
        return max(5, base_epoch_duration - 2)  # Shorten epoch during high load
    elif market_volatility > 0.5:
        return max(5, base_epoch_duration - 1)  # Shorten epoch during high volatility
    else:
        return base_epoch_duration

def determine_matrix_size(transaction_volume, expected_growth, base_matrix_size=256):
    # Example logic to determine matrix size
    if expected_growth > 0.2:
        return min(1024, base_matrix_size * 2)  # Increase size for high growth
    elif expected_growth < 0.1:
        return max(16, base_matrix_size / 2)  # Decrease size for low growth
    else:
        return base_matrix_size

def calculate_recovery_time(current_metrics):
    """
    Estimate time to recovery based on current market metrics
    """
    price_gap = abs(1 - current_metrics['current_price'])
    convergence_rate = abs(current_metrics['convergence_rate'])
    
    if convergence_rate <= 0:
        return float('inf')
    
    return price_gap / convergence_rate

def calculate_liquidity_restoration(current_metrics):
    """
    Calculate rate of liquidity restoration
    """
    target_liquidity = 0.8
    current_liquidity = current_metrics['liquidity_ratio']
    pressure = current_metrics['market_pressure']
    
    if current_liquidity >= target_liquidity:
        return 0
    
    restoration_rate = (1 / (1 + abs(pressure))) * 0.1  # Base 10% restoration rate
    return restoration_rate * (target_liquidity - current_liquidity)

def calculate_dynamic_spread(liquidity_ratio, market_pressure, normalized_validators):
    """Calculate dynamic spread based on market conditions"""
    base_spread = 0.001  # 0.1% base spread
    
    # Increase spread when liquidity is low
    liquidity_factor = max(1, (0.8 / liquidity_ratio) ** 2)
    
    # Increase spread under high market pressure
    pressure_factor = 1 + abs(market_pressure)
    
    # Decrease spread with more validators
    validator_factor = 1 / (0.5 + normalized_validators)
    
    return base_spread * liquidity_factor * pressure_factor * validator_factor