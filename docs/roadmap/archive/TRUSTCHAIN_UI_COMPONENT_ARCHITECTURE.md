# TrustChain UI Component Architecture Specification

## 🎯 **Architecture Overview**

This document defines the technical architecture for consolidating TrustChain UI components, extracting unique features from Svelte UI and integrating them into the existing React TypeScript system.

**Base Integration**: Existing `TrustChainModule.tsx` (1,130 lines) with React Router and comprehensive security features.

---

## 📋 **Component Specifications**

### **1. NodeConfigurationSettings.tsx**

#### **Interface Definitions**
```typescript
// /ui/frontend/lib/types/TrustChainTypes.ts

export interface NodeConfiguration {
  nodeId: string;
  ipv6Address: string;
  region: string;
  zone: string;
  proxyEnabled: boolean;
  autoDiscovery: boolean;
  maxConnections: number;
  bandwidth: {
    upload: number;    // Mbps
    download: number;  // Mbps
  };
  networkSettings: {
    port: number;
    protocol: 'TCP' | 'UDP' | 'QUIC';
    encryption: boolean;
    compression: boolean;
  };
  resourceLimits: {
    cpu: number;       // percentage
    memory: number;    // GB
    storage: number;   // GB
  };
  status: 'active' | 'inactive' | 'maintenance' | 'error';
  lastUpdated: string;
}

export interface NodeConfigurationUpdate {
  nodeId: string;
  updates: Partial<Omit<NodeConfiguration, 'nodeId' | 'status' | 'lastUpdated'>>;
}

export interface NodeValidationResult {
  valid: boolean;
  errors: Array<{
    field: string;
    message: string;
    severity: 'error' | 'warning' | 'info';
  }>;
  warnings: string[];
}
```

#### **Component Architecture**
```typescript
// /ui/frontend/components/trustchain/NodeConfigurationSettings.tsx

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Slider } from '@/components/ui/slider';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useNodeConfiguration, useUpdateNodeConfiguration, useValidateNodeConfiguration } from '@/hooks/api/useTrustChain';
import { cn } from '@/lib/utils';
import { 
  Settings, 
  Server, 
  Network, 
  Shield, 
  AlertTriangle, 
  CheckCircle, 
  Save,
  RotateCcw 
} from 'lucide-react';

interface NodeConfigurationSettingsProps {
  nodeId?: string;
  onConfigurationChange?: (config: NodeConfiguration) => void;
  readOnly?: boolean;
}

export function NodeConfigurationSettings({ 
  nodeId, 
  onConfigurationChange, 
  readOnly = false 
}: NodeConfigurationSettingsProps) {
  // Component implementation with:
  // - Real-time validation
  // - Auto-save functionality
  // - IPv6 address validation
  // - Bandwidth testing integration
  // - Resource limit enforcement
  // - Connection status monitoring
}
```

#### **API Integration**
```typescript
// /ui/frontend/hooks/api/useTrustChain.ts (additions)

export function useNodeConfiguration(nodeId?: string) {
  return useQuery({
    queryKey: ['trustchain', 'node-config', nodeId],
    queryFn: () => trustChainAPI.getNodeConfiguration(nodeId),
    enabled: !!nodeId,
    staleTime: 30000,
    refetchInterval: 60000, // 1 minute for config changes
  });
}

export function useUpdateNodeConfiguration() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (update: NodeConfigurationUpdate) => 
      trustChainAPI.updateNodeConfiguration(update),
    onSuccess: (updatedConfig) => {
      queryClient.setQueryData(['trustchain', 'node-config', updatedConfig.nodeId], updatedConfig);
      queryClient.invalidateQueries({ queryKey: ['trustchain', 'nodes'] });
    },
  });
}

export function useValidateNodeConfiguration() {
  return useMutation({
    mutationFn: (config: Partial<NodeConfiguration>) => 
      trustChainAPI.validateNodeConfiguration(config),
    // Real-time validation for form fields
  });
}
```

---

### **2. QuantumSecuritySettings.tsx**

#### **Interface Definitions**
```typescript
export interface QuantumSecurityConfiguration {
  quantumSafe: boolean;
  algorithms: {
    signing: 'FALCON-1024' | 'DILITHIUM-3' | 'SPHINCS+';
    keyExchange: 'KYBER-1024' | 'NTRU' | 'SABER';
    encryption: 'AES-256-GCM' | 'ChaCha20-Poly1305';
  };
  certificateSettings: {
    keySize: 1024 | 2048 | 4096;
    validityPeriod: number; // days
    autoRotation: boolean;
    rotationInterval: number; // days
  };
  protocolSettings: {
    tlsVersion: '1.3' | '1.4';
    certificateValidation: 'strict' | 'permissive' | 'custom';
    ocspStapling: boolean;
    hsts: boolean;
  };
  quantumMetrics: {
    keyStrength: number;     // bits
    signatureSize: number;   // bytes
    verificationTime: number; // ms
    keyGenTime: number;      // ms
  };
  lastUpdated: string;
}

export interface FalconCertificateInfo {
  algorithm: 'FALCON-1024';
  publicKeySize: number;
  privateKeySize: number;
  signatureSize: number;
  securityLevel: number; // NIST level
  quantumResistant: boolean;
  performanceMetrics: {
    keyGeneration: number;  // ops/sec
    signing: number;        // ops/sec
    verification: number;   // ops/sec
  };
}
```

#### **Component Architecture**
```typescript
// /ui/frontend/components/trustchain/QuantumSecuritySettings.tsx

interface QuantumSecuritySettingsProps {
  nodeId?: string;
  showAdvanced?: boolean;
  onSecurityChange?: (config: QuantumSecurityConfiguration) => void;
}

export function QuantumSecuritySettings({ 
  nodeId, 
  showAdvanced = false, 
  onSecurityChange 
}: QuantumSecuritySettingsProps) {
  // Component features:
  // - Algorithm selection with performance comparison
  // - FALCON-1024 detailed configuration
  // - Certificate rotation policy management
  // - Quantum resistance assessment
  // - Performance impact analysis
  // - Security level visualization
}
```

#### **FALCON-1024 Certificate Enhancement**
```typescript
// /ui/frontend/components/trustchain/FalconCertificateDetails.tsx

export function FalconCertificateDetails({ 
  certificate, 
  showTechnicalDetails = false 
}: {
  certificate: Certificate;
  showTechnicalDetails?: boolean;
}) {
  // Enhanced certificate display with:
  // - FALCON-1024 algorithm details
  // - Quantum resistance indicators
  // - Key size and signature analysis
  // - Performance metrics
  // - Security level assessment
  // - Extension management
}
```

---

### **3. ConsensusMetricsPanel.tsx**

#### **Interface Definitions**
```typescript
export interface FourProofConsensusMetrics {
  proofs: {
    space: ProofMetrics;    // PoSpace
    stake: ProofMetrics;    // PoStake
    work: ProofMetrics;     // PoWork
    time: ProofMetrics;     // PoTime
  };
  consensus: {
    currentRound: number;
    participationRate: number;
    byzantineDetected: number;
    agreementRate: number;
    networkSync: boolean;
    lastBlock: string;
  };
  performance: {
    validationTime: number;   // ms
    throughput: number;       // transactions/sec
    latency: number;          // ms
    efficiency: number;       // percentage
  };
  historical: {
    hourly: ConsensusDataPoint[];
    daily: ConsensusDataPoint[];
    weekly: ConsensusDataPoint[];
  };
}

export interface ProofMetrics {
  type: 'PoSpace' | 'PoStake' | 'PoWork' | 'PoTime';
  coverage: number;         // percentage
  validity: number;         // percentage
  performance: number;      // operations/sec
  resourceUsage: number;    // percentage
  lastProof: string;        // timestamp
  status: 'active' | 'warning' | 'error';
}

export interface ConsensusDataPoint {
  timestamp: string;
  round: number;
  participationRate: number;
  validationTime: number;
  proofCoverage: {
    space: number;
    stake: number;
    work: number;
    time: number;
  };
}
```

#### **Component Architecture**
```typescript
// /ui/frontend/components/trustchain/ConsensusMetricsPanel.tsx

interface ConsensusMetricsPanelProps {
  nodeId?: string;
  timeRange?: 'hour' | 'day' | 'week' | 'month';
  showHistorical?: boolean;
  realTimeUpdates?: boolean;
}

export function ConsensusMetricsPanel({ 
  nodeId, 
  timeRange = 'hour', 
  showHistorical = true,
  realTimeUpdates = true 
}: ConsensusMetricsPanelProps) {
  // Component features:
  // - Real-time four-proof metrics
  // - Interactive consensus visualization
  // - Historical data analysis
  // - Performance trend analysis
  // - Byzantine fault detection alerts
  // - Network synchronization status
}
```

#### **Real-time Metrics Integration**
```typescript
// /ui/frontend/hooks/api/useConsensusMetrics.ts

export function useConsensusMetrics(nodeId?: string, realTime = true) {
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<string | null>(null);

  const query = useQuery({
    queryKey: ['trustchain', 'consensus-metrics', nodeId],
    queryFn: () => trustChainAPI.getConsensusMetrics(nodeId),
    refetchInterval: realTime ? 5000 : 30000, // 5s for real-time
    staleTime: realTime ? 1000 : 10000,
  });

  // WebSocket subscription for real-time updates
  useEffect(() => {
    if (!realTime) return;

    const setupRealtimeUpdates = async () => {
      try {
        const subscriptionId = await web3Events.subscribe('trustchain', 'consensus.metrics', (event) => {
          queryClient.setQueryData(['trustchain', 'consensus-metrics', nodeId], (oldData: FourProofConsensusMetrics | undefined) => {
            if (!oldData) return event.data;
            
            // Merge real-time updates with existing data
            return {
              ...oldData,
              proofs: { ...oldData.proofs, ...event.data.proofs },
              consensus: { ...oldData.consensus, ...event.data.consensus },
              performance: { ...oldData.performance, ...event.data.performance },
            };
          });
        });

        subscriptionRef.current = subscriptionId;
      } catch (error) {
        console.error('Failed to setup real-time consensus metrics:', error);
      }
    };

    setupRealtimeUpdates();

    return () => {
      if (subscriptionRef.current) {
        web3Events.unsubscribe(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
  }, [queryClient, nodeId, realTime]);

  return query;
}
```

---

### **4. Ecosystem Metrics Integration**

#### **Cross-Component State Management**
```typescript
// /ui/frontend/store/ecosystemStore.ts

interface EcosystemMetrics {
  totalAssets: number;
  activeCertificates: number;
  networkThroughput: number; // Gbps
  consensusBlocks: number;
  quantumConnections: number;
  economicRewards: number;
  crossServiceStatus: {
    trustchain: ServiceStatus;
    hypermesh: ServiceStatus;
    caesar: ServiceStatus;
    stoq: ServiceStatus;
    catalog: ServiceStatus;
    ngauge: ServiceStatus;
  };
}

export const useEcosystemStore = create<{
  metrics: EcosystemMetrics;
  updateMetrics: (updates: Partial<EcosystemMetrics>) => void;
  subscribeToUpdates: () => void;
}>((set, get) => ({
  metrics: initialMetrics,
  updateMetrics: (updates) => set(state => ({ 
    metrics: { ...state.metrics, ...updates } 
  })),
  subscribeToUpdates: () => {
    // WebSocket subscription for cross-service metrics
  }
}));
```

#### **Cross-Service Data Aggregation**
```typescript
// /ui/frontend/hooks/api/useEcosystemMetrics.ts

export function useEcosystemMetrics() {
  const queryClient = useQueryClient();
  
  return useQuery({
    queryKey: ['ecosystem', 'metrics'],
    queryFn: async () => {
      // Aggregate data from all services
      const [
        trustchainHealth,
        hypermeshAssets,
        caesarRewards,
        stoqPerformance,
        catalogData,
        ngaugeUsers
      ] = await Promise.allSettled([
        trustChainAPI.getHealthStatus(),
        hypermeshAPI.getAssetSummary(),
        caesarAPI.getRewardsSummary(),
        stoqAPI.getPerformanceMetrics(),
        catalogAPI.getResourceSummary(),
        ngaugeAPI.getUserMetrics()
      ]);

      // Combine and normalize data
      return aggregateEcosystemMetrics({
        trustchainHealth,
        hypermeshAssets,
        caesarRewards,
        stoqPerformance,
        catalogData,
        ngaugeUsers
      });
    },
    refetchInterval: 30000, // 30 seconds
    staleTime: 10000,
  });
}
```

---

## 🔧 **Integration Architecture**

### **TrustChainModule.tsx Enhancement**
```typescript
// Updated routing structure for new components

function TrustChainModule() {
  return (
    <div className="space-y-6">
      <SubNavigation />
      <Routes>
        <Route path="/" element={<TrustChainOverview />} />
        <Route path="/networks" element={<NetworkManagement />} />
        <Route path="/consensus" element={<ConsensusSettings />} />
        <Route path="/security" element={<SecuritySettings />} />
        {/* NEW ROUTES */}
        <Route path="/node-config" element={<NodeConfigurationSettings />} />
        <Route path="/quantum-security" element={<QuantumSecuritySettings />} />
        <Route path="/consensus-metrics" element={<ConsensusMetricsPanel />} />
        <Route path="/ecosystem" element={<EcosystemMetricsDashboard />} />
      </Routes>
    </div>
  );
}
```

### **Enhanced Sub-Navigation**
```typescript
const subNavigation = [
  { name: 'Overview', href: '/trustchain' },
  { name: 'Networks', href: '/trustchain/networks' },
  { name: 'Consensus', href: '/trustchain/consensus' },
  { name: 'Security', href: '/trustchain/security' },
  // NEW NAVIGATION ITEMS
  { name: 'Node Config', href: '/trustchain/node-config' },
  { name: 'Quantum Security', href: '/trustchain/quantum-security' },
  { name: 'Metrics', href: '/trustchain/consensus-metrics' },
  { name: 'Ecosystem', href: '/trustchain/ecosystem' },
];
```

---

## 📊 **Testing Architecture**

### **Component Testing Strategy**
```typescript
// /ui/frontend/components/trustchain/__tests__/NodeConfigurationSettings.test.tsx

describe('NodeConfigurationSettings', () => {
  test('validates IPv6 addresses correctly', async () => {
    render(<NodeConfigurationSettings nodeId="test-node" />);
    
    const ipv6Input = screen.getByLabelText(/IPv6 Address/i);
    await user.type(ipv6Input, 'invalid-ipv6');
    
    expect(screen.getByText(/Invalid IPv6 address/i)).toBeInTheDocument();
  });

  test('updates configuration with validation', async () => {
    const mockUpdate = vi.fn();
    render(<NodeConfigurationSettings onConfigurationChange={mockUpdate} />);
    
    // Test configuration updates
  });

  test('handles real-time configuration changes', async () => {
    // Test WebSocket updates
  });
});
```

### **API Integration Testing**
```typescript
// /ui/frontend/hooks/api/__tests__/useConsensusMetrics.test.tsx

describe('useConsensusMetrics', () => {
  test('fetches four-proof metrics correctly', async () => {
    const { result } = renderHook(() => useConsensusMetrics('test-node'));
    
    await waitFor(() => {
      expect(result.current.data).toHaveProperty('proofs');
      expect(result.current.data.proofs).toHaveProperty('space');
      expect(result.current.data.proofs).toHaveProperty('stake');
      expect(result.current.data.proofs).toHaveProperty('work');
      expect(result.current.data.proofs).toHaveProperty('time');
    });
  });

  test('handles real-time consensus updates', async () => {
    // Test real-time WebSocket updates
  });
});
```

---

## 🚀 **Performance Optimizations**

### **Memoization Strategy**
```typescript
// Optimize re-renders for complex metrics displays
const MemoizedConsensusChart = React.memo(ConsensusChart, (prevProps, nextProps) => {
  return (
    prevProps.data.consensus.currentRound === nextProps.data.consensus.currentRound &&
    prevProps.data.performance.validationTime === nextProps.data.performance.validationTime
  );
});
```

### **Query Optimization**
```typescript
// Selective re-fetching for different data types
export function useOptimizedConsensusMetrics(nodeId?: string) {
  // High-frequency updates for critical metrics
  const realTimeMetrics = useQuery({
    queryKey: ['consensus', 'real-time', nodeId],
    queryFn: () => trustChainAPI.getRealTimeConsensusData(nodeId),
    refetchInterval: 1000, // 1 second
    select: (data) => ({
      currentRound: data.currentRound,
      participationRate: data.participationRate,
      networkSync: data.networkSync
    })
  });

  // Lower-frequency updates for historical data
  const historicalMetrics = useQuery({
    queryKey: ['consensus', 'historical', nodeId],
    queryFn: () => trustChainAPI.getHistoricalConsensusData(nodeId),
    refetchInterval: 60000, // 1 minute
    staleTime: 30000
  });

  return { realTimeMetrics, historicalMetrics };
}
```

---

## 📝 **Implementation Checklist**

### **Phase 1: Core Components (Week 1)**
- [ ] `NodeConfigurationSettings.tsx` with IPv6 validation
- [ ] `QuantumSecuritySettings.tsx` with FALCON-1024 support
- [ ] Basic API hooks for node configuration
- [ ] Component integration with existing TrustChainModule

### **Phase 2: Consensus Metrics (Week 2)**
- [ ] `ConsensusMetricsPanel.tsx` with four-proof display
- [ ] Real-time WebSocket integration for consensus data
- [ ] Historical metrics visualization
- [ ] Performance optimization for real-time updates

### **Phase 3: Enhanced Security (Week 3)**
- [ ] `FalconCertificateDetails.tsx` component
- [ ] Enhanced certificate management with FALCON-1024
- [ ] Quantum security assessment tools
- [ ] Certificate extension management

### **Phase 4: Ecosystem Integration (Week 4)**
- [ ] Cross-service metrics aggregation
- [ ] Ecosystem dashboard component
- [ ] State management optimization
- [ ] Final testing and integration

---

## 🎯 **Success Criteria**

### **Functional Requirements**
- [ ] **Node Configuration**: Real-time node settings with validation
- [ ] **Quantum Security**: FALCON-1024 certificate management
- [ ] **Consensus Metrics**: Four-proof consensus visualization
- [ ] **Real-time Updates**: <500ms latency for live data
- [ ] **API Integration**: Full backend connectivity with fallback

### **Technical Requirements**
- [ ] **Type Safety**: 100% TypeScript coverage
- [ ] **Performance**: <100ms component render times
- [ ] **Testing**: >90% test coverage
- [ ] **Accessibility**: WCAG 2.1 AA compliance
- [ ] **Mobile Responsive**: Works on all screen sizes

### **Integration Requirements**
- [ ] **Backward Compatibility**: Existing TrustChain features preserved
- [ ] **Clean Architecture**: Modular, maintainable code structure
- [ ] **Documentation**: Complete component documentation
- [ ] **Error Handling**: Graceful error states and recovery

---

**Status**: Ready for Implementation  
**Timeline**: 4 weeks for complete UI consolidation  
**Dependencies**: TrustChain API backend, WebSocket infrastructure  
**Risk Mitigation**: Incremental rollout with feature flags