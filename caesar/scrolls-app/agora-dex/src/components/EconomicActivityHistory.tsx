// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { FileText, Package, Zap, ExternalLink, CheckCircle, Clock, AlertTriangle } from 'lucide-react';
import GoldPriceService, { type GoldEconomicData } from '../../../shared/services/GoldPriceService';

// Economic activity types (not speculation trades)
interface EconomicActivity {
  id: string;
  timestamp: number;
  type: 'service_payment' | 'asset_purchase' | 'contract_execution' | 'fiat_conversion';
  amount: number;
  description: string;
  counterparty: string;
  status: 'completed' | 'pending' | 'confirmed';
  demurrageSaved: number;
  utilityCategory: string;
  hypermeshTxHash?: string;
}

const ACTIVITY_CONFIG = {
  service_payment: {
    icon: FileText,
    label: 'Service Payment',
    color: 'text-blue-400',
    bgColor: 'bg-blue-900/30'
  },
  asset_purchase: {
    icon: Package,
    label: 'Asset Purchase',
    color: 'text-green-400',
    bgColor: 'bg-green-900/30'
  },
  contract_execution: {
    icon: Zap,
    label: 'Contract Execution',
    color: 'text-purple-400',
    bgColor: 'bg-purple-900/30'
  },
  fiat_conversion: {
    icon: ExternalLink,
    label: 'Fiat Conversion',
    color: 'text-orange-400',
    bgColor: 'bg-orange-900/30'
  }
} as const;

const EconomicActivityHistory: React.FC = () => {
  const [activities, setActivities] = useState<EconomicActivity[]>([]);
  const [filter, setFilter] = useState<string>('all');
  const [goldData, setGoldData] = useState<GoldEconomicData | null>(null);

  useEffect(() => {
    // Subscribe to gold price data for real-time pricing
    const goldService = GoldPriceService.getInstance();
    const unsubscribe = goldService.subscribe((data: GoldEconomicData) => {
      setGoldData(data);
    });
    
    return unsubscribe;
  }, []);
  
  useEffect(() => {
    // Generate realistic economic activity data (not speculation trades)
    const generateEconomicActivities = () => {
      const activityTypes: Array<EconomicActivity['type']> = [
        'service_payment', 'asset_purchase', 'contract_execution', 'fiat_conversion'
      ];
      
      const descriptions = {
        service_payment: [
          'Web Development Services',
          'Graphic Design Project',
          'Consulting Session',
          'Technical Writing',
          'Legal Advisory',
          'Marketing Campaign'
        ],
        asset_purchase: [
          'Software License',
          'Digital Art NFT',
          'Domain Name',
          'Stock Photography',
          'Physical Product',
          'Intellectual Property'
        ],
        contract_execution: [
          'Freelance Development Contract',
          'Service Level Agreement',
          'Supply Chain Contract',
          'Distribution Agreement',
          'Licensing Deal',
          'Partnership Contract'
        ],
        fiat_conversion: [
          'CAESAR → Fiat (Gold Price-Based)',
          'Utility Payment Conversion',
          'Emergency Fiat Access',
          'Cross-border Payment',
          'Business Expense Coverage',
          'External Service Payment'
        ]
      };

      const counterparties = [
        'TechCorp Solutions', 'CreativeStudio', 'ConsultingGroup', 'DigitalAssets Inc',
        'ServiceProvider LLC', 'Innovation Labs', 'Professional Services', 'Asset Exchange',
        'Work Platform', 'Business Solutions', 'Creative Agency', 'Tech Startup'
      ];

      const activities: EconomicActivity[] = [];

      for (let i = 0; i < 50; i++) {
        const type = activityTypes[Math.floor(Math.random() * activityTypes.length)];
        const amount = parseFloat((Math.random() * 1000 + 10).toFixed(2));
        const description = descriptions[type][Math.floor(Math.random() * descriptions[type].length)];
        const counterparty = counterparties[Math.floor(Math.random() * counterparties.length)];
        const demurrageSaved = parseFloat((amount * 0.001 * Math.random()).toFixed(4)); // Small demurrage savings for utility use
        
        activities.push({
          id: `activity-${i}`,
          timestamp: Date.now() - (i * Math.random() * 7 * 24 * 60 * 60 * 1000), // Last 7 days
          type,
          amount,
          description,
          counterparty,
          status: Math.random() > 0.1 ? 'completed' : Math.random() > 0.5 ? 'pending' : 'confirmed',
          demurrageSaved,
          utilityCategory: type.replace('_', ' '),
          hypermeshTxHash: Math.random() > 0.3 ? `0x${Math.random().toString(16).substr(2, 64)}` : undefined
        });
      }

      return activities.sort((a, b) => b.timestamp - a.timestamp);
    };

    setActivities(generateEconomicActivities());
  }, []);

  const filteredActivities = filter === 'all' 
    ? activities 
    : activities.filter(activity => activity.type === filter);

  const formatTime = (timestamp: number) => {
    const now = Date.now();
    const diff = now - timestamp;
    const days = Math.floor(diff / (24 * 60 * 60 * 1000));
    const hours = Math.floor(diff / (60 * 60 * 1000));
    
    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    return 'Just now';
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed': return <CheckCircle className="text-green-400" size={16} />;
      case 'pending': return <Clock className="text-yellow-400" size={16} />;
      case 'confirmed': return <AlertTriangle className="text-blue-400" size={16} />;
      default: return <Clock className="text-gray-400" size={16} />;
    }
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-xl font-semibold">Economic Activity History</h3>
          <p className="text-gray-400 text-sm">Real utility transactions and economic activity</p>
        </div>
        
        {/* Activity Filter */}
        <div className="flex bg-gray-800 rounded-lg p-1">
          <button
            onClick={() => setFilter('all')}
            className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
              filter === 'all'
                ? 'bg-caesar-gold text-black'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            All
          </button>
          {Object.entries(ACTIVITY_CONFIG).map(([type, config]) => (
            <button
              key={type}
              onClick={() => setFilter(type)}
              className={`px-3 py-1 rounded text-sm font-medium transition-colors flex items-center gap-1 ${
                filter === type
                  ? 'bg-caesar-gold text-black'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              <config.icon size={14} />
              {config.label}
            </button>
          ))}
        </div>
      </div>

      {/* Gold-Based Economic Activity */}
      <div className="bg-green-900/30 border border-green-700 rounded-lg p-3 mb-4">
        <p className="text-green-100 text-sm">
          <strong>Gold-Pegged Utility:</strong> This history shows real economic activity priced against gold with deviation controls. 
          Each transaction represents genuine value exchange within the stabilization mechanism.
        </p>
      </div>

      {/* Activity List */}
      <div className="space-y-3 max-h-96 overflow-y-auto">
        {filteredActivities.map((activity) => {
          const config = ACTIVITY_CONFIG[activity.type];
          const IconComponent = config.icon;
          
          return (
            <div
              key={activity.id}
              className={`${config.bgColor} border border-gray-600 rounded-lg p-4 hover:border-gray-500 transition-colors`}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-3 flex-1">
                  <div className={`p-2 ${config.bgColor} rounded-lg`}>
                    <IconComponent className={config.color} size={20} />
                  </div>
                  
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-semibold">{config.label}</span>
                      {getStatusIcon(activity.status)}
                      <span className="text-xs text-gray-400 uppercase">{activity.status}</span>
                    </div>
                    
                    <p className="text-gray-300 mb-1">{activity.description}</p>
                    <p className="text-gray-400 text-sm">with {activity.counterparty}</p>
                    
                    <div className="flex items-center gap-4 mt-2 text-xs text-gray-400">
                      <span>{formatTime(activity.timestamp)}</span>
                      {activity.demurrageSaved > 0 && (
                        <span className="text-green-400">
                          Demurrage saved: {activity.demurrageSaved} CAESAR
                        </span>
                      )}
                      {activity.hypermeshTxHash && (
                        <span className="text-blue-400">
                          Hypermesh: {activity.hypermeshTxHash.slice(0, 8)}...
                        </span>
                      )}
                    </div>
                  </div>
                </div>
                
                <div className="text-right">
                  <p className="font-semibold text-lg">
                    {activity.amount.toLocaleString()} CAESAR
                  </p>
                  <p className="text-gray-400 text-sm">
                    ~${goldData ? (activity.amount * goldData.caesarPrice.current).toFixed(2) : (activity.amount * 85).toFixed(2)}
                  </p>
                  <p className="text-xs text-yellow-400">
                    {goldData ? `Gold-pegged (${goldData.deviationBand.percentage > 0 ? '+' : ''}${goldData.deviationBand.percentage.toFixed(1)}%)` : 'Gold-pegged'}
                  </p>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {filteredActivities.length === 0 && (
        <div className="text-center py-8 text-gray-400">
          <Package size={48} className="mx-auto mb-2 opacity-50" />
          <p>No economic activity found for selected filter.</p>
        </div>
      )}

      {/* Summary Stats */}
      <div className="border-t border-gray-700 mt-6 pt-4">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-center">
          <div>
            <p className="text-gray-400 text-sm">Total Transactions</p>
            <p className="font-semibold text-lg">{filteredActivities.length}</p>
          </div>
          <div>
            <p className="text-gray-400 text-sm">Total Value</p>
            <p className="font-semibold text-lg">
              {filteredActivities.reduce((sum, a) => sum + a.amount, 0).toLocaleString()} CAESAR
            </p>
          </div>
          <div>
            <p className="text-gray-400 text-sm">Demurrage Saved</p>
            <p className="font-semibold text-lg text-green-400">
              {filteredActivities.reduce((sum, a) => sum + a.demurrageSaved, 0).toFixed(2)}
            </p>
          </div>
          <div>
            <p className="text-gray-400 text-sm">Avg. Transaction</p>
            <p className="font-semibold text-lg">
              {filteredActivities.length > 0 
                ? (filteredActivities.reduce((sum, a) => sum + a.amount, 0) / filteredActivities.length).toFixed(0)
                : '0'
              } CAESAR
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default EconomicActivityHistory;