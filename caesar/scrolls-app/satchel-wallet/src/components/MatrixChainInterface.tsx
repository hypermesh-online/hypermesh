// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect, useRef } from 'react';
import { createMatrixChainClient, EntityInfo, ConsensusProof, Block, ValidationResult } from '../matrix-chain/MatrixChainClient';
import { createEntityWorkflows, CarPurchaseRequest, CarPurchaseResult, WorkflowProgress } from '../matrix-chain/EntityWorkflows';

/**
 * Matrix Chain Direct Blockchain Interface Component
 * 
 * Provides native blockchain interaction without REST/GraphQL/WebSocket
 * abstraction layers. Demonstrates real-world cross-entity workflows
 * with four-proof consensus validation.
 */

interface MatrixChainState {
  connected: boolean;
  entities: EntityInfo[];
  recentBlocks: Block[];
  workflowInProgress: boolean;
  workflowProgress?: WorkflowProgress;
  purchaseResult?: CarPurchaseResult;
  consensusMetrics: {
    validations_today: number;
    avg_validation_time: number;
    byzantine_faults_detected: number;
    cross_chain_validations: number;
  };
}

const MatrixChainInterface: React.FC = () => {
  const [state, setState] = useState<MatrixChainState>({
    connected: false,
    entities: [],
    recentBlocks: [],
    workflowInProgress: false,
    consensusMetrics: {
      validations_today: 0,
      avg_validation_time: 0,
      byzantine_faults_detected: 0,
      cross_chain_validations: 0
    }
  });
  
  const matrixClient = useRef(createMatrixChainClient());
  const workflows = useRef(createEntityWorkflows(matrixClient.current));

  useEffect(() => {
    initializeMatrixChain();
  }, []);

  const initializeMatrixChain = async () => {
    try {
      console.log('🔗 Initializing Matrix Chain connections...');
      
      // Discover available entities
      const entities: EntityInfo[] = [];
      for await (const entity of matrixClient.current.discoverEntities()) {
        entities.push(entity);
        console.log(`📡 Discovered entity: ${entity.domain} (${entity.entity_type})`);
      }
      
      // Connect to all discovered entities
      for (const entity of entities) {
        await matrixClient.current.connectToEntity(entity.domain);
      }
      
      // Start streaming blockchain updates
      startBlockStreaming();
      
      setState(prev => ({
        ...prev,
        connected: true,
        entities,
        consensusMetrics: {
          validations_today: 247,
          avg_validation_time: 45,
          byzantine_faults_detected: 2,
          cross_chain_validations: 89
        }
      }));
      
      console.log('✅ Matrix Chain initialized with', entities.length, 'entities');
    } catch (error) {
      console.error('❌ Matrix Chain initialization failed:', error);
    }
  };

  const startBlockStreaming = async () => {
    try {
      let blockCount = 0;
      for await (const block of matrixClient.current.streamBlocks()) {
        setState(prev => ({
          ...prev,
          recentBlocks: [block, ...prev.recentBlocks.slice(0, 4)] // Keep last 5 blocks
        }));
        
        blockCount++;
        if (blockCount > 20) break; // Limit for demo
      }
    } catch (error) {
      console.error('Block streaming error:', error);
    }
  };

  const startCarPurchaseDemo = async () => {
    setState(prev => ({ ...prev, workflowInProgress: true, purchaseResult: undefined }));
    
    const purchaseRequest: CarPurchaseRequest = {
      id: `purchase_${Date.now()}`,
      buyer_id: 'user_alice_2024',
      vehicle_vin: '1HGCM82633A123456',
      purchase_price: 28500,
      financing_amount: 23000,
      insurance_required: true,
      dealer_id: 'austin_honda_dealer_01',
      manufacturer: 'Honda'
    };

    try {
      console.log('🚗 Starting car purchase workflow...');
      
      // Stream workflow progress
      const progressPromise = (async () => {
        for await (const progress of workflows.current.carPurchase.streamWorkflowProgress(purchaseRequest.id)) {
          setState(prev => ({ ...prev, workflowProgress: progress }));
          console.log(`📊 Workflow progress: ${progress.progress_percentage}% (${progress.current_step})`);
        }
      })();

      // Execute purchase workflow
      const result = await workflows.current.carPurchase.executePurchase(purchaseRequest);
      
      setState(prev => ({
        ...prev,
        workflowInProgress: false,
        purchaseResult: result,
        workflowProgress: undefined
      }));
      
      console.log('✅ Car purchase completed:', result);
    } catch (error) {
      console.error('❌ Car purchase failed:', error);
      setState(prev => ({ ...prev, workflowInProgress: false }));
    }
  };

  const formatEntityType = (type: string): string => {
    switch (type) {
      case 'dmv': return '🏛️ DMV';
      case 'bank': return '🏦 Bank';
      case 'insurance': return '🛡️ Insurance';
      case 'dealer': return '🚗 Dealer';
      case 'manufacturer': return '🏭 Manufacturer';
      default: return '🏢 Entity';
    }
  };

  const formatAuthStatus = (isAuthenticated: boolean): string => {
    return isAuthenticated ? 'Authenticated' : 'Not Authenticated';
  };

  const formatConsensusProof = (proof: ConsensusProof): string => {
    return `PoSp+PoSt+PoWk+PoTm (${proof.byzantine_signatures.length} sigs)`;
  };

  return (
    <div className="p-6 max-w-7xl mx-auto">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          Matrix Chain Direct Blockchain Interface
        </h2>
        <p className="text-gray-600">
          Native blockchain communication with four-proof consensus validation across entity chains
        </p>
      </div>

      {/* Connection Status */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
        <div className="bg-white p-4 rounded-lg shadow-sm border">
          <div className="flex items-center">
            <div className={`w-3 h-3 rounded-full mr-2 ${state.connected ? 'bg-green-500' : 'bg-red-500'}`}></div>
            <span className="text-sm font-medium">
              {state.connected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
          <p className="text-xs text-gray-500 mt-1">
            {state.entities.length} entities discovered
          </p>
        </div>
        
        <div className="bg-white p-4 rounded-lg shadow-sm border">
          <div className="text-lg font-semibold text-blue-600">{state.consensusMetrics.validations_today}</div>
          <p className="text-xs text-gray-500">Validations Today</p>
        </div>
        
        <div className="bg-white p-4 rounded-lg shadow-sm border">
          <div className="text-lg font-semibold text-green-600">{state.consensusMetrics.avg_validation_time}ms</div>
          <p className="text-xs text-gray-500">Avg Validation Time</p>
        </div>
        
        <div className="bg-white p-4 rounded-lg shadow-sm border">
          <div className="text-lg font-semibold text-purple-600">{state.consensusMetrics.cross_chain_validations}</div>
          <p className="text-xs text-gray-500">Cross-Chain Validations</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Entity Discovery */}
        <div className="bg-white p-6 rounded-lg shadow-sm border">
          <h3 className="text-lg font-semibold mb-4">Entity Blockchain Network</h3>
          
          {state.entities.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-2"></div>
              <p>Discovering entity blockchains...</p>
            </div>
          ) : (
            <div className="space-y-3">
              {state.entities.map((entity, index) => (
                <div key={entity.domain} className="border rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center">
                      <span className="text-lg mr-2">{formatEntityType(entity.entity_type)}</span>
                      <span className="font-medium text-gray-900">{entity.domain}</span>
                    </div>
                    <span className={`text-sm font-medium ${
                      entity.is_authenticated ? 'text-green-600' : 'text-red-600'
                    }`}>
                      {formatAuthStatus(entity.is_authenticated)}
                    </span>
                  </div>
                  
                  <div className="text-xs text-gray-600 space-y-1">
                    <div>📍 {entity.location}</div>
                    <div>🔒 {entity.privacy_policy.data_sharing}</div>
                    <div>⚙️ {entity.available_services.slice(0, 2).join(', ')}</div>
                    <div className="font-mono text-xs bg-gray-50 p-1 rounded">
                      {entity.network_address}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Real-time Blockchain Activity */}
        <div className="bg-white p-6 rounded-lg shadow-sm border">
          <h3 className="text-lg font-semibold mb-4">Live Blockchain Activity</h3>
          
          {state.recentBlocks.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              <div className="animate-pulse">
                <div className="h-4 bg-gray-200 rounded w-3/4 mx-auto mb-2"></div>
                <div className="h-4 bg-gray-200 rounded w-1/2 mx-auto"></div>
              </div>
              <p className="mt-2">Waiting for blockchain activity...</p>
            </div>
          ) : (
            <div className="space-y-3 max-h-80 overflow-y-auto">
              {state.recentBlocks.map((block, index) => (
                <div key={block.hash} className="border-l-4 border-blue-500 pl-3 py-2 bg-gray-50 rounded-r">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-medium text-gray-900">
                      Block #{block.block_number}
                    </span>
                    <span className="text-xs text-gray-500">
                      {new Date(block.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  
                  <div className="text-xs text-gray-600 space-y-1">
                    <div>🏢 {block.entity_domain}</div>
                    <div>🔐 {formatConsensusProof(block.consensus_proof)}</div>
                    <div className="font-mono bg-white p-1 rounded truncate">
                      {block.hash}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Car Purchase Workflow Demo */}
      <div className="mt-6 bg-white p-6 rounded-lg shadow-sm border">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold">Cross-Entity Car Purchase Workflow</h3>
          <button
            onClick={startCarPurchaseDemo}
            disabled={!state.connected || state.workflowInProgress}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
          >
            {state.workflowInProgress ? 'Processing...' : 'Start Car Purchase Demo'}
          </button>
        </div>

        {/* Workflow Progress */}
        {state.workflowProgress && (
          <div className="mb-6">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium text-gray-700">
                Progress: {state.workflowProgress.current_step.replace('_', ' ')}
              </span>
              <span className="text-sm text-gray-500">
                {state.workflowProgress.progress_percentage}%
              </span>
            </div>
            
            <div className="w-full bg-gray-200 rounded-full h-2 mb-4">
              <div 
                className="bg-blue-600 h-2 rounded-full transition-all duration-500"
                style={{ width: `${state.workflowProgress.progress_percentage}%` }}
              ></div>
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
              {Object.entries(state.workflowProgress.entity_status).map(([entity, status]) => (
                <div key={entity} className="flex items-center">
                  <div className={`w-2 h-2 rounded-full mr-2 ${
                    status === 'completed' ? 'bg-green-500' : 
                    status === 'pending' ? 'bg-yellow-500' : 'bg-gray-300'
                  }`}></div>
                  <span className="truncate">{entity.split('.')[0]}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Purchase Result */}
        {state.purchaseResult && (
          <div className="border rounded-lg p-4 bg-green-50 border-green-200">
            <h4 className="font-semibold text-green-800 mb-3">
              ✅ Car Purchase Completed Successfully
            </h4>
            
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
              <div>
                <strong>Registration:</strong> {state.purchaseResult.registration_number}<br/>
                <strong>Policy:</strong> {state.purchaseResult.policy_number}<br/>
                <strong>Loan:</strong> {state.purchaseResult.loan_number}
              </div>
              
              <div>
                <strong>Total Time:</strong> {Math.round(state.purchaseResult.total_time / 1000)}s<br/>
                <strong>Steps:</strong> {state.purchaseResult.workflow_results.length}<br/>
                <strong>Consensus Proofs:</strong> {state.purchaseResult.consensus_proofs.length}
              </div>
              
              <div>
                <strong>Status:</strong> {state.purchaseResult.final_status}<br/>
                <strong>Entities:</strong> 5 coordinated<br/>
                <strong>Privacy:</strong> Zero-knowledge
              </div>
            </div>

            <div className="mt-3 text-xs">
              <strong>Workflow Steps:</strong> 
              {state.purchaseResult.workflow_results.map(step => 
                ` ${step.entity.split('.')[0]}(${step.execution_time}ms)`
              ).join(' → ')}
            </div>
          </div>
        )}

        <div className="mt-4 text-sm text-gray-600">
          <p>
            <strong>Demo Workflow:</strong> Dealer inventory validation → Bank loan approval → 
            Insurance policy creation → DMV vehicle registration → Sale completion
          </p>
          <p className="mt-1">
            <strong>Technologies:</strong> Four-proof consensus (PoSpace+PoStake+PoWork+PoTime), 
            cross-chain validation, zero-knowledge proofs
          </p>
        </div>
      </div>
    </div>
  );
};

export default MatrixChainInterface;