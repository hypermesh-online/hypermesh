# Copyright © 2026 Hypermesh Foundation. All rights reserved.
# Licensed under the Business Source License 1.1.
# See the LICENSE file in the repository root for full license text.

import statistics
from datetime import datetime, timedelta
import requests
from typing import Dict
import os
from dotenv import load_dotenv
import json

class GoldAnalyzer:
    def __init__(self, gold_api_key: str, metal_api_key: str):
        self.gold_api_key = gold_api_key
        self.metal_api_key = metal_api_key
        self.cache_file = "gold_price_cache.json"
        self.cache_expiry_hours = 24
        # Known estimates of world's gold supply (in metric tons)
        # Sources: World Gold Council, USGS, Reuters, Bloomberg, Goldman Sachs
        self.gold_supply_estimates = {
            "World Gold Council": 212582,  # Above ground gold as of 2021
            "USGS": 244000,                # Estimated total gold ever mined
        }
        
        # API endpoints for different gold price sources
        self.price_endpoints = {
            "GoldAPI": "https://www.goldapi.io/api/XAU/USD",
            "MetalPriceAPI": "https://api.metalpriceapi.com/v1/latest?api_key={}&base=USD&currencies=XAU",
            "SwissQuote": "https://forex-data-feed.swissquote.com/public-quotes/bboquotes/instrument/XAU/USD",
        }

    def calculate_total_gold_grams(self):
        # Convert metric tons to grams (1 metric ton = 1,000,000 grams)
        gold_in_grams = [tons * 1_000_000 for tons in self.gold_supply_estimates.values()]
        
        stats = {
            "mean": statistics.mean(gold_in_grams),
            "median": statistics.median(gold_in_grams),
            "std_dev": statistics.stdev(gold_in_grams),
            "min": min(gold_in_grams),
            "max": max(gold_in_grams),
            "total": max(gold_in_grams)  # Changed from sum() to max()
        }
        
        return stats

    def fetch_gold_price(self) -> Dict[str, float]:
        """Fetch real-time gold prices from multiple APIs with caching"""
        # Check cache first
        if os.path.exists(self.cache_file):
            with open(self.cache_file, 'r') as f:
                cache = json.loads(f.read())
                cache_time = datetime.fromisoformat(cache['timestamp'])
                if datetime.now() - cache_time < timedelta(hours=self.cache_expiry_hours):
                    return cache['prices']

        prices = {}
        
        # GoldAPI request
        try:
            response = requests.get(
                self.price_endpoints["GoldAPI"],
                headers={
                    'x-access-token': self.gold_api_key,
                    'Content-Type': 'application/json'
                }
            )
            if response.status_code == 200:
                data = response.json()
                price_per_gram = float(data['price']) / 31.1034768
                prices["GoldAPI"] = price_per_gram
        except Exception as e:
            print(f"Error fetching from GoldAPI: {e}")

        # MetalPriceAPI request
        try:
            price_per_gram = self.fetch_metal_price_api()
            prices["MetalPriceAPI"] = price_per_gram
        except Exception as e:
            print(f"Error fetching from MetalPriceAPI: {e}")

        # Add fallback logic if all APIs fail
        if not prices:
            print("Warning: Using fallback static prices due to API failure")
            prices = {
                "Fallback_Price": 62.84
            }

        # Cache the results
        cache_data = {
            'timestamp': datetime.now().isoformat(),
            'prices': prices
        }
        with open(self.cache_file, 'w') as f:
            json.dump(cache_data, f)

        return prices

    def analyze_gold_prices(self):
        # Fetch real-time prices
        prices = self.fetch_gold_price()
        price_values = [p for p in prices.values() if p > 0]  # Filter out zero values
        
        stats = {
            "mean": statistics.mean(price_values),
            "median": statistics.median(price_values),
            "std_dev": statistics.stdev(price_values) if len(price_values) > 1 else 0,
            "min": min(price_values),
            "max": max(price_values),
            "sources": list(prices.keys())
        }
        
        return stats

    def generate_report(self):
        gold_stats = self.calculate_total_gold_grams()
        price_stats = self.analyze_gold_prices()
        
        report = f"""Gold Supply and Price Analysis Report
Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}

Total World Gold Supply Statistics (in grams):
Mean: {gold_stats['mean']:,.2f}
Median: {gold_stats['median']:,.2f}
Standard Deviation: {gold_stats['std_dev']:,.2f}
Range: {gold_stats['min']:,.2f} to {gold_stats['max']:,.2f}

Gold Price Statistics (USD per gram):
Mean: ${price_stats['mean']:.2f}
Median: ${price_stats['median']:.2f}
Standard Deviation: ${price_stats['std_dev']:.2f}
Range: ${price_stats['min']:.2f} to ${price_stats['max']:.2f}

Estimated Total Value of World's Gold:
Mean Value: ${(gold_stats['mean'] * price_stats['mean']):,.2f} USD
Total Supply: {gold_stats['total']:,.2f}
Total Value: ${(gold_stats['total'] * price_stats['mean']):,.2f} USD

Data Sources:
Supply Estimates: {', '.join(self.gold_supply_estimates.keys())}
Price Sources: {', '.join(self.price_endpoints.keys())}
"""
        return report

    def fetch_metal_price_api(self) -> float:
        """Fetch gold price from MetalPriceAPI in USD/gram"""
        url = f"https://api.metalpriceapi.com/v1/latest?api_key={self.metal_api_key}&base=USD&currencies=XAU"
        response = requests.get(url)
        data = response.json()
        
        # Convert to USD/gram
        oz_to_gram = 31.1034768
        usd_per_oz = 1 / data['rates']['XAU']  # Invert to get USD/oz
        return usd_per_oz / oz_to_gram

def main():
    # Load environment variables from .env file
    load_dotenv()
    
    gold_api_key = os.getenv('GOLD_API_KEY')
    metal_api_key = os.getenv('METALPRICE_API_KEY')
    
    if not gold_api_key or not metal_api_key:
        raise ValueError("Please set both GOLD_API_KEY and METALPRICE_API_KEY in .env file")
    
    analyzer = GoldAnalyzer(gold_api_key, metal_api_key)
    print(analyzer.generate_report())

if __name__ == "__main__":
    main()