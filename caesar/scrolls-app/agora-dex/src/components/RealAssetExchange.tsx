// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { Package, FileText, Zap } from 'lucide-react';
import { useWallet } from '../hooks/useWallet';
import { useTokenBalance } from '../hooks/useTokenBalance';
import { formatTokenAmount } from '../utils/formatters';
import GoldPriceIndicator from '../../../shared/components/GoldPriceIndicator';
import GoldPriceService, { type GoldEconomicData } from '../../../shared/services/GoldPriceService';

// Real asset categories supported by Hypermesh tensor-mesh architecture
const ASSET_CATEGORIES = [
  { id: 'services', name: 'Professional Services', icon: FileText },
  { id: 'physical', name: 'Physical Assets', icon: Package },
  { id: 'contracts', name: 'Work Contracts', icon: Zap },
] as const;

// Real utility transactions, not speculation - Enhanced for Hypermesh
const UTILITY_ACTIONS = [
  {
    id: 'service_payment',
    name: 'Service Payment',
    description: 'Pay for professional services with Caesar tokens',
    demurrage_benefit: 'Immediate use - no demurrage applied',
    hypermesh_integration: 'Direct service provider verification',
    cross_chain: false
  },
  {
    id: 'asset_purchase',
    name: 'Asset Purchase', 
    description: 'Purchase tokenized real-world assets via Hypermesh tensor-mesh',
    demurrage_benefit: 'Active transaction - encourages token velocity',
    hypermesh_integration: 'Real-world asset verification and tracking',
    cross_chain: true
  },
  {
    id: 'contract_execution',
    name: 'Work Contract Execution',
    description: 'Execute smart work contracts with milestone-based payments',
    demurrage_benefit: 'Utility usage - aligns with token purpose',
    hypermesh_integration: 'Automated escrow and completion verification',
    cross_chain: true
  },
  {
    id: 'cross_chain_bridge',
    name: 'Cross-Chain Asset Transfer',
    description: 'Transfer assets across chains via LayerZero V2 OFT',
    demurrage_benefit: 'Network utility - maintains token velocity',
    hypermesh_integration: 'Tensor-mesh coordination across networks',
    cross_chain: true
  },
  {
    id: 'fiat_conversion',
    name: 'Fiat Off-Ramp',
    description: 'Convert Caesar based on gold price (±deviation bands) for real-world expenses',
    demurrage_benefit: 'Exit utility - fees vary based on price band position',
    hypermesh_integration: 'Gold price oracle and compliance verification',
    cross_chain: false
  }
] as const;

const RealAssetExchange: React.FC = () => {
  const { wallet } = useWallet();
  const [selectedCategory, setSelectedCategory] = useState<string>('services');
  const [selectedAction, setSelectedAction] = useState<string>('service_payment');
  const [amount, setAmount] = useState('');
  const [description, setDescription] = useState('');
  const [goldData, setGoldData] = useState<GoldEconomicData | null>(null);
  
  useEffect(() => {
    const goldService = GoldPriceService.getInstance();
    const unsubscribe = goldService.subscribe((data: GoldEconomicData) => {
      setGoldData(data);
    });
    
    return unsubscribe;
  }, []);

  // Get Caesar token balance (anti-speculative token)
  const caesarBalance = useTokenBalance(
    '0x6299744254422aadb6a57183f47eaae1678cf86cc58a0c78dfc4fd2caa3ba2a4', // Real deployed Caesar token
    wallet.account, 
    wallet.provider
  );

  const selectedActionData = UTILITY_ACTIONS.find(action => action.id === selectedAction);
  
  // Calculate real-time cost with gold-based pricing
  const calculateTransactionCost = () => {
    if (!goldData || !amount) return { caesarAmount: 0, goldValue: 0, feeAdjustment: 1.0, incentiveInfo: '' };
    
    const caesarAmount = parseFloat(amount);
    const goldValue = caesarAmount * goldData.caesarPrice.current;
    
    // Apply fee adjustments based on deviation band position
    let feeAdjustment = 1.0;
    let incentiveInfo = '';
    
    if (goldData.deviationBand.status === 'above') {
      feeAdjustment = goldData.incentives.proportionalCostFactor;
      incentiveInfo = `+${(feeAdjustment * 100 - 100).toFixed(1)}% penalty (price above gold band)`;
    } else if (goldData.deviationBand.status === 'below') {
      feeAdjustment = goldData.incentives.proportionalCostFactor;  
      incentiveInfo = `${((feeAdjustment - 1) * 100).toFixed(1)}% reward (price below gold band)`;
    } else {
      incentiveInfo = 'Standard rates (within optimal gold band)';
    }
    
    return { caesarAmount, goldValue, feeAdjustment, incentiveInfo };
  };
  
  const costCalculation = calculateTransactionCost();

  const handleUtilityTransaction = async () => {
    if (!wallet.isConnected || !amount || !description) {
      alert('Please connect wallet and provide transaction details');
      return;
    }

    if (parseFloat(amount) <= 0) {
      alert('Amount must be greater than 0');
      return;
    }

    try {
      console.log('Real utility transaction:', {
        type: selectedAction,
        category: selectedCategory,
        amount: amount,
        description: description,
        purpose: 'Real economic activity',
        anti_speculation: true
      });
      
      alert(`Utility transaction prepared: ${selectedActionData?.name}\nAmount: ${amount} CAESAR\nPurpose: Real economic activity (not speculation)`);
    } catch (error) {
      console.error('Utility transaction failed:', error);
      alert('Transaction failed: ' + (error as Error).message);
    }
  };

  return (
    <div className="card max-w-lg mx-auto">
      <div className="flex items-center gap-3 mb-6">
        <Package className="text-caesar-gold" size={24} />
        <div>
          <h3 className="text-xl font-semibold">Real Asset Exchange</h3>
          <p className="text-sm text-gray-400">Utility-focused transactions, not speculation</p>
        </div>
      </div>

      {/* Gold Price Indicator */}
      <div className="mb-6">
        <GoldPriceIndicator />
      </div>

      {/* Asset Category Selection */}
      <div className="mb-6">
        <label className="text-sm text-gray-400 mb-3 block">Asset Category</label>
        <div className="grid grid-cols-3 gap-2">
          {ASSET_CATEGORIES.map((category) => (
            <button
              key={category.id}
              onClick={() => setSelectedCategory(category.id)}
              className={`p-3 rounded-lg border text-center transition-colors ${
                selectedCategory === category.id
                  ? 'border-caesar-gold bg-caesar-gold/10 text-caesar-gold'
                  : 'border-gray-600 bg-gray-800 hover:border-gray-500'
              }`}
            >
              <category.icon size={20} className="mx-auto mb-1" />
              <span className="text-xs font-medium">{category.name}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Utility Action Selection */}
      <div className="mb-6">
        <label className="text-sm text-gray-400 mb-3 block">Transaction Type</label>
        <select
          value={selectedAction}
          onChange={(e) => setSelectedAction(e.target.value)}
          className="w-full bg-gray-800 border border-gray-600 rounded-lg p-3 focus:border-caesar-gold focus:outline-none"
        >
          {UTILITY_ACTIONS.map((action) => (
            <option key={action.id} value={action.id}>
              {action.name} - {action.description}
            </option>
          ))}
        </select>
        {selectedActionData && (
          <div className="mt-3 space-y-2">
            <p className="text-xs text-green-400">
              ✓ {selectedActionData.demurrage_benefit}
            </p>
            <p className="text-xs text-blue-400">
              🔗 {selectedActionData.hypermesh_integration}
            </p>
            {selectedActionData.cross_chain && (
              <p className="text-xs text-purple-400">
                🌐 Cross-chain compatible via LayerZero V2 OFT
              </p>
            )}
          </div>
        )}
      </div>

      {/* Amount Input */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm text-gray-400">Caesar Amount</label>
          <span className="text-sm text-gray-400">
            Balance: {formatTokenAmount(caesarBalance.balance)} CAESAR
          </span>
        </div>
        <div className="bg-gray-800 rounded-lg p-4">
          <div className="flex items-center justify-between">
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.0"
              step="0.000001"
              className="bg-transparent text-2xl font-medium outline-none flex-1"
            />
            <div className="flex items-center gap-2 bg-gray-700 px-3 py-2 rounded-lg">
              <img
                src="/caesar-logo.png"
                alt="CAESAR"
                className="w-6 h-6 rounded-full"
              />
              <span className="font-medium">CAESAR</span>
            </div>
          </div>
        </div>
        
        {/* Real-Time Cost Calculation */}
        {goldData && amount && parseFloat(amount) > 0 && (
          <div className="mt-3 p-3 bg-gray-900/50 rounded-lg border border-gray-600">
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-gray-400">Gold Value:</span>
                <div className="font-bold text-yellow-400">
                  ${costCalculation.goldValue.toFixed(2)}
                </div>
              </div>
              <div>
                <span className="text-gray-400">Current Rate:</span>
                <div className="font-bold">
                  ${goldData.caesarPrice.current.toFixed(3)}/CAESAR
                </div>
              </div>
            </div>
            <div className="mt-2 text-xs">
              <span className={`font-medium ${
                goldData.deviationBand.status === 'above' ? 'text-red-400' :
                goldData.deviationBand.status === 'below' ? 'text-green-400' : 'text-blue-400'
              }`}>
                {costCalculation.incentiveInfo}
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Transaction Description */}
      <div className="mb-6">
        <label className="text-sm text-gray-400 mb-2 block">Transaction Purpose</label>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Describe the real-world utility or service being paid for..."
          className="w-full bg-gray-800 border border-gray-600 rounded-lg p-3 focus:border-caesar-gold focus:outline-none resize-none"
          rows={3}
        />
      </div>

      {/* Utility Transaction Button */}
      <button
        onClick={handleUtilityTransaction}
        disabled={!wallet.isConnected || !amount || !description}
        className="w-full btn-primary disabled:bg-gray-600 disabled:cursor-not-allowed"
      >
        {!wallet.isConnected 
          ? 'Connect Wallet' 
          : `Execute ${selectedActionData?.name || 'Utility Transaction'}`
        }
      </button>

      {/* Enhanced Hypermesh Integration Notice */}
      <div className="mt-4 p-4 bg-blue-900/30 border border-blue-700 rounded-lg">
        <h4 className="font-semibold text-blue-200 mb-3">Hypermesh Tensor-Mesh Architecture</h4>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
          <div>
            <span className="text-blue-400">Real-World Asset Tracking:</span>
            <span className="ml-2 text-blue-100">Physical asset tokenization and verification</span>
          </div>
          <div>
            <span className="text-blue-400">Cross-Chain Coordination:</span>
            <span className="ml-2 text-blue-100">LayerZero V2 OFT for seamless transfers</span>
          </div>
          <div>
            <span className="text-blue-400">Smart Contract Escrow:</span>
            <span className="ml-2 text-blue-100">Automated milestone-based payments</span>
          </div>
          <div>
            <span className="text-blue-400">Compliance Integration:</span>
            <span className="ml-2 text-blue-100">Real-world regulatory verification</span>
          </div>
        </div>
        <div className="mt-3 p-2 bg-green-900/30 border border-green-700 rounded text-xs">
          <span className="text-green-200">
            <strong>Gold-Based Stability:</strong> All transactions priced against gold with deviation controls. 
            Hypermesh tracks price bands and applies incentives/penalties to maintain utility-driven stability.
          </span>
        </div>
      </div>
    </div>
  );
};

export default RealAssetExchange;