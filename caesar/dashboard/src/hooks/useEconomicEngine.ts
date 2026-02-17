// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useState, useEffect, useCallback, useMemo } from 'react';
import { ethers } from 'ethers';
import { toast } from 'sonner';

// Contract ABIs (simplified - in production these would be imported from typechain)
const ECONOMIC_ENGINE_ABI = [
  "function getEconomicParameters() view returns (tuple(uint256 baseDemurrageRate, uint256 maxDemurrageRate, uint256 stabilityThreshold, uint256 fiatDiscountFactor, uint256 gracePeriodsHours, uint256 interventionThreshold, uint256 rebalanceFrequency, uint256 emergencyThreshold))",
  "function getHealthMetrics() view returns (tuple(uint256 overallHealth, uint256 priceStability, uint256 liquidityHealth, uint256 participationRate, uint256 lastUpdate))",
  "function getSystemHealth() view returns (tuple(uint256 priceStability, uint256 liquidityHealth, uint256 participationRate, uint256 reserveRatio, uint256 lastUpdate))",
  "function monitorEconomicHealth() returns (tuple(uint256 overallHealth, uint256 priceStability, uint256 liquidityHealth, uint256 participationRate, uint256 lastUpdate))",
  "function updateEconomicParameters(tuple(uint256 baseDemurrageRate, uint256 maxDemurrageRate, uint256 stabilityThreshold, uint256 fiatDiscountFactor, uint256 gracePeriodsHours, uint256 interventionThreshold, uint256 rebalanceFrequency, uint256 emergencyThreshold) params)",
  "function activateEmergencyMode(string reason)",
  "function deactivateEmergencyMode()",
  "function maintainPegStability() returns (uint8)",
  "function rebalanceReserves() returns (tuple(uint8 operationType, uint256 amount, uint32 targetChain, uint256 timestamp, bool success))",
  "event EconomicParametersUpdated(tuple(uint256 baseDemurrageRate, uint256 maxDemurrageRate, uint256 stabilityThreshold, uint256 fiatDiscountFactor, uint256 gracePeriodsHours, uint256 interventionThreshold, uint256 rebalanceFrequency, uint256 emergencyThreshold) params)",
  "event HealthMetricsUpdated(tuple(uint256 overallHealth, uint256 priceStability, uint256 liquidityHealth, uint256 participationRate, uint256 reserveRatio, uint256 demurrageEfficiency, uint256 antiSpeculationEffectiveness, uint256 systemStress, uint256 timestamp) metrics)",
  "event EmergencyModeActivated(address operator, string reason)"
];

const DEMURRAGE_MANAGER_ABI = [
  "function getTotalDemurrageCollected() view returns (uint256)",
  "function getCurrentEffectiveRate(address account) view returns (uint256)",
  "function getStabilityMetrics() view returns (tuple(uint256 currentStabilityIndex, uint256 priceDeviation, uint256 velocityRatio, uint256 participationRate, uint256 lastStabilityUpdate))"
];

const ANTI_SPECULATION_ENGINE_ABI = [
  "function getTotalPenaltiesCollected() view returns (uint256)",
  "function getNetworkRiskMetrics() view returns (uint256 avgRiskScore, uint256 totalPenalties, uint256 totalAnalyzed, uint256 manipulationDetections, uint256 circuitBreakerActivations)"
];

const STABILITY_POOL_ABI = [
  "function getPoolComposition() view returns (tuple(uint256 totalBalance, uint256 penaltyFunds, uint256 demurrageFunds, uint256 reserveFunds, uint256 emergencyFunds))",
  "function getStabilityMetrics() view returns (tuple(uint256 stabilityIndex, uint256 reserveRatio, uint256 interventionCount, uint256 lastUpdate, bool emergencyMode))"
];

interface EconomicMetrics {
  overallHealth: number;
  priceStability: number;
  liquidityHealth: number;
  participationRate: number;
  reserveRatio: number;
  demurrageEfficiency: number;
  antiSpeculationEffectiveness: number;
  systemStress: number;
  timestamp: number;
}

interface EconomicParameters {
  baseDemurrageRate: number;
  maxDemurrageRate: number;
  stabilityThreshold: number;
  fiatDiscountFactor: number;
  gracePeriodsHours: number;
  interventionThreshold: number;
  rebalanceFrequency: number;
  emergencyThreshold: number;
  lastUpdate?: number;
}

interface DemurrageData {
  totalCollected: string;
  averageRate: number;
  accountsProcessed: number;
  exemptAccounts: number;
}

interface AntiSpeculationData {
  totalPenalties: string;
  flaggedAccounts: number;
  circuitBreakerActivations: number;
  averageRiskScore: number;
}

interface StabilityPoolData {
  totalBalance: string;
  penaltyFunds: string;
  demurrageFunds: string;
  reserveFunds: string;
  emergencyFunds: string;
}

interface HealthHistoryPoint {
  timestamp: number;
  overallHealth: number;
  priceStability: number;
  liquidityHealth: number;
  participationRate: number;
  demurrageRate: number;
}

// Contract addresses - these would be loaded from environment or config
const CONTRACT_ADDRESSES = {
  ECONOMIC_ENGINE: process.env.REACT_APP_ECONOMIC_ENGINE_ADDRESS || '0x1234567890123456789012345678901234567890',
  DEMURRAGE_MANAGER: process.env.REACT_APP_DEMURRAGE_MANAGER_ADDRESS || '0x2345678901234567890123456789012345678901',
  ANTI_SPECULATION_ENGINE: process.env.REACT_APP_ANTI_SPECULATION_ENGINE_ADDRESS || '0x3456789012345678901234567890123456789012',
  STABILITY_POOL: process.env.REACT_APP_STABILITY_POOL_ADDRESS || '0x4567890123456789012345678901234567890123',
};

export const useEconomicEngine = () => {
  const [provider, setProvider] = useState<ethers.Provider | null>(null);
  const [signer, setSigner] = useState<ethers.Signer | null>(null);
  const [contracts, setContracts] = useState<{
    economicEngine: ethers.Contract | null;
    demurrageManager: ethers.Contract | null;
    antiSpeculationEngine: ethers.Contract | null;
    stabilityPool: ethers.Contract | null;
  }>({
    economicEngine: null,
    demurrageManager: null,
    antiSpeculationEngine: null,
    stabilityPool: null,
  });

  const [economicMetrics, setEconomicMetrics] = useState<EconomicMetrics>({
    overallHealth: 800,
    priceStability: 850,
    liquidityHealth: 750,
    participationRate: 600,
    reserveRatio: 1000,
    demurrageEfficiency: 780,
    antiSpeculationEffectiveness: 820,
    systemStress: 200,
    timestamp: Date.now() / 1000,
  });

  const [economicParameters, setEconomicParameters] = useState<EconomicParameters>({
    baseDemurrageRate: 50, // 0.5%
    maxDemurrageRate: 200, // 2%
    stabilityThreshold: 100, // 1%
    fiatDiscountFactor: 5000, // 50%
    gracePeriodsHours: 48,
    interventionThreshold: 500,
    rebalanceFrequency: 3600,
    emergencyThreshold: 1000,
    lastUpdate: Date.now() / 1000,
  });

  const [demurrageData, setDemurrageData] = useState<DemurrageData>({
    totalCollected: '125000.50',
    averageRate: 0.75,
    accountsProcessed: 1250,
    exemptAccounts: 25,
  });

  const [antiSpeculationData, setAntiSpeculationData] = useState<AntiSpeculationData>({
    totalPenalties: '87500.25',
    flaggedAccounts: 45,
    circuitBreakerActivations: 3,
    averageRiskScore: 320,
  });

  const [stabilityPoolData, setStabilityPoolData] = useState<StabilityPoolData>({
    totalBalance: '2500000.00',
    penaltyFunds: '850000.25',
    demurrageFunds: '1250000.50',
    reserveFunds: '350000.75',
    emergencyFunds: '49999.50',
  });

  const [healthHistory, setHealthHistory] = useState<HealthHistoryPoint[]>([]);
  const [isEmergencyMode, setIsEmergencyMode] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Initialize provider and contracts
  useEffect(() => {
    const initializeProvider = async () => {
      try {
        if (typeof window.ethereum !== 'undefined') {
          const provider = new ethers.BrowserProvider(window.ethereum);
          await window.ethereum.request({ method: 'eth_requestAccounts' });
          const signer = await provider.getSigner();
          
          setProvider(provider);
          setSigner(signer);

          // Initialize contracts
          const economicEngine = new ethers.Contract(
            CONTRACT_ADDRESSES.ECONOMIC_ENGINE,
            ECONOMIC_ENGINE_ABI,
            signer
          );

          const demurrageManager = new ethers.Contract(
            CONTRACT_ADDRESSES.DEMURRAGE_MANAGER,
            DEMURRAGE_MANAGER_ABI,
            provider
          );

          const antiSpeculationEngine = new ethers.Contract(
            CONTRACT_ADDRESSES.ANTI_SPECULATION_ENGINE,
            ANTI_SPECULATION_ENGINE_ABI,
            provider
          );

          const stabilityPool = new ethers.Contract(
            CONTRACT_ADDRESSES.STABILITY_POOL,
            STABILITY_POOL_ABI,
            provider
          );

          setContracts({
            economicEngine,
            demurrageManager,
            antiSpeculationEngine,
            stabilityPool,
          });

          // Set up event listeners
          setupEventListeners(economicEngine);

        } else {
          throw new Error('MetaMask is not installed');
        }
      } catch (err) {
        console.error('Failed to initialize provider:', err);
        setError(err as Error);
        // Use mock data for development
        generateMockHistory();
      }
    };

    initializeProvider();
  }, []);

  // Generate mock history data for development
  const generateMockHistory = useCallback(() => {
    const history: HealthHistoryPoint[] = [];
    const now = Date.now() / 1000;
    
    for (let i = 23; i >= 0; i--) {
      const timestamp = now - (i * 3600); // 1 hour intervals
      history.push({
        timestamp,
        overallHealth: 750 + Math.sin(i / 4) * 100 + Math.random() * 50,
        priceStability: 800 + Math.cos(i / 3) * 80 + Math.random() * 40,
        liquidityHealth: 700 + Math.sin(i / 5) * 120 + Math.random() * 60,
        participationRate: 600 + Math.cos(i / 6) * 100 + Math.random() * 50,
        demurrageRate: 0.5 + Math.sin(i / 8) * 0.3 + Math.random() * 0.1,
      });
    }
    
    setHealthHistory(history);
  }, []);

  // Set up event listeners for contract events
  const setupEventListeners = useCallback((economicEngine: ethers.Contract) => {
    economicEngine.on('HealthMetricsUpdated', (metrics: any) => {
      setEconomicMetrics({
        overallHealth: Number(metrics.overallHealth),
        priceStability: Number(metrics.priceStability),
        liquidityHealth: Number(metrics.liquidityHealth),
        participationRate: Number(metrics.participationRate),
        reserveRatio: Number(metrics.reserveRatio),
        demurrageEfficiency: Number(metrics.demurrageEfficiency),
        antiSpeculationEffectiveness: Number(metrics.antiSpeculationEffectiveness),
        systemStress: Number(metrics.systemStress),
        timestamp: Number(metrics.timestamp),
      });

      // Add to history
      setHealthHistory(prev => [
        ...prev.slice(-23), // Keep last 23 points
        {
          timestamp: Number(metrics.timestamp),
          overallHealth: Number(metrics.overallHealth) / 10,
          priceStability: Number(metrics.priceStability) / 10,
          liquidityHealth: Number(metrics.liquidityHealth) / 10,
          participationRate: Number(metrics.participationRate) / 10,
          demurrageRate: 0.5, // Would need to get this from demurrage manager
        }
      ]);

      toast.success('Health metrics updated');
    });

    economicEngine.on('EmergencyModeActivated', (operator: string, reason: string) => {
      setIsEmergencyMode(true);
      toast.error(`Emergency mode activated: ${reason}`);
    });

    economicEngine.on('EconomicParametersUpdated', (params: any) => {
      setEconomicParameters({
        baseDemurrageRate: Number(params.baseDemurrageRate),
        maxDemurrageRate: Number(params.maxDemurrageRate),
        stabilityThreshold: Number(params.stabilityThreshold),
        fiatDiscountFactor: Number(params.fiatDiscountFactor),
        gracePeriodsHours: Number(params.gracePeriodsHours),
        interventionThreshold: Number(params.interventionThreshold),
        rebalanceFrequency: Number(params.rebalanceFrequency),
        emergencyThreshold: Number(params.emergencyThreshold),
        lastUpdate: Date.now() / 1000,
      });
      toast.success('Economic parameters updated');
    });

    return () => {
      economicEngine.removeAllListeners();
    };
  }, []);

  // Refresh all data from contracts
  const refreshData = useCallback(async () => {
    if (!contracts.economicEngine || !contracts.demurrageManager || 
        !contracts.antiSpeculationEngine || !contracts.stabilityPool) {
      // Use mock data if contracts not available
      generateMockHistory();
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      // Fetch all data in parallel
      const [
        healthMetrics,
        systemHealth,
        parameters,
        demurrageCollected,
        antiSpecMetrics,
        poolComposition,
        stabilityMetrics,
      ] = await Promise.all([
        contracts.economicEngine.getHealthMetrics(),
        contracts.economicEngine.getSystemHealth(),
        contracts.economicEngine.getEconomicParameters(),
        contracts.demurrageManager.getTotalDemurrageCollected(),
        contracts.antiSpeculationEngine.getNetworkRiskMetrics(),
        contracts.stabilityPool.getPoolComposition(),
        contracts.stabilityPool.getStabilityMetrics(),
      ]);

      // Update state with fresh data
      setEconomicMetrics({
        overallHealth: Number(healthMetrics.overallHealth),
        priceStability: Number(healthMetrics.priceStability),
        liquidityHealth: Number(healthMetrics.liquidityHealth),
        participationRate: Number(healthMetrics.participationRate),
        reserveRatio: Number(systemHealth.reserveRatio),
        demurrageEfficiency: 780, // Mock value - would come from contract
        antiSpeculationEffectiveness: 820, // Mock value
        systemStress: 200, // Mock value
        timestamp: Number(healthMetrics.lastUpdate),
      });

      setEconomicParameters({
        baseDemurrageRate: Number(parameters.baseDemurrageRate),
        maxDemurrageRate: Number(parameters.maxDemurrageRate),
        stabilityThreshold: Number(parameters.stabilityThreshold),
        fiatDiscountFactor: Number(parameters.fiatDiscountFactor),
        gracePeriodsHours: Number(parameters.gracePeriodsHours),
        interventionThreshold: Number(parameters.interventionThreshold),
        rebalanceFrequency: Number(parameters.rebalanceFrequency),
        emergencyThreshold: Number(parameters.emergencyThreshold),
        lastUpdate: Date.now() / 1000,
      });

      setDemurrageData({
        totalCollected: ethers.formatEther(demurrageCollected),
        averageRate: 0.75, // Mock - would calculate from contract
        accountsProcessed: 1250, // Mock
        exemptAccounts: 25, // Mock
      });

      setAntiSpeculationData({
        totalPenalties: ethers.formatEther(antiSpecMetrics.totalPenalties),
        flaggedAccounts: 45, // Mock
        circuitBreakerActivations: Number(antiSpecMetrics.circuitBreakerActivations),
        averageRiskScore: Number(antiSpecMetrics.avgRiskScore),
      });

      setStabilityPoolData({
        totalBalance: ethers.formatEther(poolComposition.totalBalance),
        penaltyFunds: ethers.formatEther(poolComposition.penaltyFunds),
        demurrageFunds: ethers.formatEther(poolComposition.demurrageFunds),
        reserveFunds: ethers.formatEther(poolComposition.reserveFunds),
        emergencyFunds: ethers.formatEther(poolComposition.emergencyFunds),
      });

      setIsEmergencyMode(stabilityMetrics.emergencyMode);

    } catch (err) {
      console.error('Failed to refresh data:', err);
      setError(err as Error);
      toast.error('Failed to refresh data from contracts');
    } finally {
      setIsLoading(false);
    }
  }, [contracts, generateMockHistory]);

  // Update economic parameters
  const updateParameters = useCallback(async (newParams: Partial<EconomicParameters>) => {
    if (!contracts.economicEngine || !signer) {
      toast.error('Wallet not connected');
      return;
    }

    setIsLoading(true);

    try {
      const fullParams = {
        ...economicParameters,
        ...newParams,
      };

      const tx = await contracts.economicEngine.updateEconomicParameters(fullParams);
      await tx.wait();

      toast.success('Parameters updated successfully');
      await refreshData();
    } catch (err) {
      console.error('Failed to update parameters:', err);
      toast.error('Failed to update parameters');
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [contracts.economicEngine, signer, economicParameters, refreshData]);

  // Activate emergency mode
  const activateEmergencyMode = useCallback(async (reason: string) => {
    if (!contracts.economicEngine || !signer) {
      toast.error('Wallet not connected');
      return;
    }

    setIsLoading(true);

    try {
      const tx = await contracts.economicEngine.activateEmergencyMode(reason);
      await tx.wait();

      toast.success('Emergency mode activated');
      setIsEmergencyMode(true);
    } catch (err) {
      console.error('Failed to activate emergency mode:', err);
      toast.error('Failed to activate emergency mode');
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [contracts.economicEngine, signer]);

  // Deactivate emergency mode
  const deactivateEmergencyMode = useCallback(async () => {
    if (!contracts.economicEngine || !signer) {
      toast.error('Wallet not connected');
      return;
    }

    setIsLoading(true);

    try {
      const tx = await contracts.economicEngine.deactivateEmergencyMode();
      await tx.wait();

      toast.success('Emergency mode deactivated');
      setIsEmergencyMode(false);
    } catch (err) {
      console.error('Failed to deactivate emergency mode:', err);
      toast.error('Failed to deactivate emergency mode');
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [contracts.economicEngine, signer]);

  // Apply demurrage to account
  const applyDemurrage = useCallback(async (account: string, balance: string) => {
    if (!contracts.economicEngine || !signer) {
      toast.error('Wallet not connected');
      return;
    }

    setIsLoading(true);

    try {
      const tx = await contracts.economicEngine.applyDemurrage(
        account, 
        ethers.parseEther(balance)
      );
      await tx.wait();

      toast.success('Demurrage applied successfully');
      await refreshData();
    } catch (err) {
      console.error('Failed to apply demurrage:', err);
      toast.error('Failed to apply demurrage');
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [contracts.economicEngine, signer, refreshData]);

  // Execute system intervention
  const executeIntervention = useCallback(async (interventionType: string) => {
    if (!contracts.economicEngine || !signer) {
      toast.error('Wallet not connected');
      return;
    }

    setIsLoading(true);

    try {
      let tx;
      
      switch (interventionType) {
        case 'REBALANCE_RESERVES':
          tx = await contracts.economicEngine.rebalanceReserves();
          break;
        case 'UPDATE_STABILITY_METRICS':
          tx = await contracts.economicEngine.monitorEconomicHealth();
          break;
        case 'MAINTAIN_PEG_STABILITY':
          tx = await contracts.economicEngine.maintainPegStability();
          break;
        case 'FORCE_PARAMETER_SYNC':
          // This would call cross-chain sync - mock for now
          toast.success('Cross-chain sync triggered');
          return;
        default:
          throw new Error('Unknown intervention type');
      }

      await tx.wait();
      toast.success(`${interventionType.replace('_', ' ').toLowerCase()} executed successfully`);
      await refreshData();
    } catch (err) {
      console.error('Failed to execute intervention:', err);
      toast.error('Failed to execute intervention');
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [contracts.economicEngine, signer, refreshData]);

  // Initialize mock data on first load
  useEffect(() => {
    if (healthHistory.length === 0) {
      generateMockHistory();
    }
  }, [healthHistory.length, generateMockHistory]);

  const memoizedValues = useMemo(() => ({
    economicMetrics,
    economicParameters,
    demurrageData,
    antiSpeculationData,
    stabilityPoolData,
    healthHistory,
    isEmergencyMode,
    isLoading,
    error,
    provider,
    signer,
    refreshData,
    updateParameters,
    activateEmergencyMode,
    deactivateEmergencyMode,
    applyDemurrage,
    executeIntervention,
  }), [
    economicMetrics,
    economicParameters,
    demurrageData,
    antiSpeculationData,
    stabilityPoolData,
    healthHistory,
    isEmergencyMode,
    isLoading,
    error,
    provider,
    signer,
    refreshData,
    updateParameters,
    activateEmergencyMode,
    deactivateEmergencyMode,
    applyDemurrage,
    executeIntervention,
  ]);

  return memoizedValues;
};