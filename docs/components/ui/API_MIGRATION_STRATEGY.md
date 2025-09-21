# API Migration Strategy - Legacy to React/TypeScript

## 🎯 **Migration Objective**

Transform the proven legacy `web3API` + `web3Events` system into a modern React/TypeScript equivalent while preserving all functional capabilities and real-time connectivity.

---

## 📋 **Legacy API Architecture Analysis**

### **Current Legacy System (FUNCTIONAL)**
```javascript
// /legacy/interfaces/src/lib/api/index.js
export class Web3API {
  constructor() {
    this.useBackend = false;
    this.connectionStatus = {};
  }
  
  async getSystemStatus() { /* Backend + Mock fallback */ }
  async getByzantineThreats() { /* Real-time threat detection */ }
  async getCertificates() { /* TrustChain integration */ }
  async getComponentStatus(componentId) { /* Per-service status */ }
}

export const web3API = new Web3API();
export { web3Events } from './mock-server.js';
```

### **Key Legacy Features to Preserve**
1. **Automatic Backend/Mock Switching**: Intelligent fallback system
2. **Real-time Updates**: `web3Events` WebSocket-like system
3. **Multi-Service Support**: TrustChain, STOQ, HyperMesh, Caesar, Catalog, NGauge
4. **Connection Status Tracking**: Real-time connectivity monitoring
5. **Error Handling**: Graceful degradation with informative error states

---

## 🔧 **New React/TypeScript API Architecture**

### **Phase 1: Core API Infrastructure**

#### **1.1 API Client Class (TypeScript)**
```typescript
// /src/lib/api/Web3APIClient.ts
export class Web3APIClient {
  private useBackend: boolean = false;
  private connectionStatus: Record<string, boolean> = {};
  private wsConnections: Map<string, WebSocket> = new Map();
  
  constructor() {
    this.initialize();
  }
  
  async initialize(): Promise<void> {
    const connections = await this.checkBackendConnections();
    this.connectionStatus = connections;
    this.useBackend = Object.values(connections).some(Boolean);
    
    if (this.useBackend) {
      this.initializeWebSockets();
    }
  }
  
  // Service-specific methods
  async getSystemStatus(): Promise<SystemStatus>
  async getByzantineThreats(): Promise<ByzantineThreats>
  async getCertificates(): Promise<Certificate[]>
  async getComponentStatus(componentId: string): Promise<ComponentStatus>
}
```

#### **1.2 React Query Integration**
```typescript
// /src/hooks/api/useWeb3API.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { web3APIClient } from '@/lib/api/Web3APIClient';

export const useSystemStatus = () => {
  return useQuery({
    queryKey: ['system-status'],
    queryFn: () => web3APIClient.getSystemStatus(),
    refetchInterval: 5000, // Real-time updates every 5 seconds
  });
};

export const useComponentStatus = (componentId: string) => {
  return useQuery({
    queryKey: ['component-status', componentId],
    queryFn: () => web3APIClient.getComponentStatus(componentId),
    refetchInterval: 3000,
  });
};
```

#### **1.3 Real-time Event System**
```typescript
// /src/lib/api/Web3Events.ts
export class Web3Events {
  private eventTarget: EventTarget;
  private wsConnection: WebSocket | null = null;
  
  constructor() {
    this.eventTarget = new EventTarget();
    this.initializeEventStream();
  }
  
  on(eventType: string, callback: (data: any) => void) {
    this.eventTarget.addEventListener(eventType, (event: any) => {
      callback(event.detail);
    });
  }
  
  emit(eventType: string, data: any) {
    this.eventTarget.dispatchEvent(
      new CustomEvent(eventType, { detail: data })
    );
  }
}

export const web3Events = new Web3Events();
```

### **Phase 2: Service-Specific API Modules**

#### **2.1 TrustChain API Module**
```typescript
// /src/lib/api/services/TrustChainAPI.ts
export class TrustChainAPI {
  async getCertificates(): Promise<Certificate[]>
  async rotateCertificate(certificateId: string): Promise<void>
  async validateCertificate(certificate: Certificate): Promise<ValidationResult>
  async getDNSRecords(): Promise<DNSRecord[]>
  async manageDomains(): Promise<Domain[]>
}
```

#### **2.2 HyperMesh API Module**
```typescript
// /src/lib/api/services/HyperMeshAPI.ts
export class HyperMeshAPI {
  async getAssets(): Promise<Asset[]>
  async deployAsset(asset: AssetConfig): Promise<DeploymentResult>
  async getNetworkTopology(): Promise<NetworkTopology>
  async getProxyNodes(): Promise<ProxyNode[]>
  async configureResourceSharing(config: ResourceSharingConfig): Promise<void>
  async getMemoryAddressing(): Promise<MemoryAddressingInfo>
}
```

#### **2.3 STOQ Transport API Module**
```typescript
// /src/lib/api/services/STOQAPI.ts
export class STOQAPI {
  async getTunnelStatus(): Promise<TunnelStatus[]>
  async createTunnel(config: TunnelConfig): Promise<Tunnel>
  async getTrafficMetrics(): Promise<TrafficMetrics>
  async getPerformanceData(): Promise<PerformanceData>
  async manageRouting(policy: RoutingPolicy): Promise<void>
}
```

### **Phase 3: React Hooks Integration**

#### **3.1 Service-Specific Hooks**
```typescript
// /src/hooks/api/useTrustChain.ts
export const useCertificates = () => {
  return useQuery({
    queryKey: ['trustchain', 'certificates'],
    queryFn: () => trustChainAPI.getCertificates(),
  });
};

export const useRotateCertificate = () => {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (certificateId: string) => 
      trustChainAPI.rotateCertificate(certificateId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trustchain'] });
    },
  });
};
```

#### **3.2 Real-time Data Hooks**
```typescript
// /src/hooks/api/useRealTimeData.ts
export const useRealTimeSystemStatus = () => {
  const [data, setData] = useState(null);
  
  useEffect(() => {
    const handleUpdate = (data: any) => setData(data);
    web3Events.on('system-update', handleUpdate);
    
    return () => {
      web3Events.off('system-update', handleUpdate);
    };
  }, []);
  
  return data;
};
```

---

## 🔄 **Migration Implementation Plan**

### **Week 1: Foundation Setup**
1. **Create API Client Infrastructure**
   - Implement `Web3APIClient` class with TypeScript
   - Set up connection management and status tracking
   - Implement backend/mock fallback logic

2. **React Query Integration**
   - Configure React Query client in App.tsx
   - Create base query hooks for system status
   - Implement error handling and loading states

3. **Real-time Events System**
   - Implement `Web3Events` class with EventTarget
   - Create WebSocket connection management
   - Set up event subscription system

### **Week 2: Service API Modules**
1. **TrustChain API Implementation**
   - Certificate management functions
   - DNS resolution and caching
   - Domain management operations

2. **HyperMesh API Implementation**
   - Asset management functions
   - Network topology queries
   - Resource sharing configuration

3. **Basic Testing Setup**
   - Unit tests for API client
   - Mock backend responses
   - Connection status validation

### **Week 3: Advanced Features**
1. **STOQ Transport API**
   - Tunnel management functions
   - Traffic analysis queries
   - Performance monitoring

2. **Caesar Economic API**
   - Wallet integration functions
   - Trading interface operations
   - Staking management

3. **NGauge Platform API**
   - User onboarding functions
   - Analytics and metrics
   - Privacy configuration

### **Week 4: Integration & Testing**
1. **Component Integration**
   - Update all existing components to use new API
   - Replace mock data with real API calls
   - Implement real-time data updates

2. **Error Handling & Resilience**
   - Connection failure recovery
   - Automatic retry mechanisms
   - Graceful degradation strategies

3. **Performance Optimization**
   - Query deduplication
   - Caching strategies
   - Background refresh optimization

---

## 🧪 **Testing Strategy**

### **Unit Testing**
```typescript
// /src/lib/api/__tests__/Web3APIClient.test.ts
describe('Web3APIClient', () => {
  test('should fallback to mock data when backend unavailable', async () => {
    const client = new Web3APIClient();
    const status = await client.getSystemStatus();
    expect(status).toHaveProperty('components');
  });
  
  test('should handle WebSocket connection errors gracefully', () => {
    // Test WebSocket error handling
  });
});
```

### **Integration Testing**
```typescript
// /src/hooks/api/__tests__/useSystemStatus.test.tsx
describe('useSystemStatus', () => {
  test('should fetch system status and handle updates', async () => {
    const { result } = renderHook(() => useSystemStatus());
    await waitFor(() => {
      expect(result.current.data).toBeDefined();
    });
  });
});
```

### **E2E Testing**
```typescript
// /src/__tests__/e2e/api-integration.test.ts
describe('API Integration E2E', () => {
  test('should display real-time system updates', async () => {
    render(<DashboardHome />);
    // Simulate backend data update
    // Verify UI reflects changes
  });
});
```

---

## 📊 **Migration Validation Checklist**

### **Functional Parity**
- [ ] **System Status**: Real-time system health monitoring
- [ ] **Component Status**: Individual service status tracking  
- [ ] **Byzantine Detection**: Real-time threat detection and alerts
- [ ] **Certificate Management**: TrustChain certificate operations
- [ ] **Asset Management**: HyperMesh asset deployment and tracking
- [ ] **Transport Monitoring**: STOQ transport performance tracking

### **Technical Requirements**
- [ ] **Backend Connectivity**: Successful connection to all 6 services
- [ ] **Mock Fallback**: Graceful fallback when backend unavailable
- [ ] **Real-time Updates**: WebSocket or equivalent real-time data
- [ ] **Error Handling**: Comprehensive error states and recovery
- [ ] **Performance**: <100ms API response times
- [ ] **Type Safety**: Full TypeScript implementation

### **User Experience**
- [ ] **Loading States**: Appropriate loading indicators
- [ ] **Error Messages**: User-friendly error messages
- [ ] **Offline Support**: Functional with limited connectivity
- [ ] **Data Freshness**: Clear indication of data age
- [ ] **Connection Status**: Visible connection health indicators

---

## 🚀 **Success Metrics**

### **API Performance**
- **Response Time**: <100ms for cached queries
- **Real-time Latency**: <500ms for live updates
- **Uptime**: 99.9% API availability
- **Error Rate**: <1% failed requests

### **Development Experience**
- **Type Safety**: 100% TypeScript coverage
- **Test Coverage**: >90% code coverage
- **Documentation**: Complete API documentation
- **Maintainability**: Clear separation of concerns

### **User Experience**
- **Loading Performance**: <2s initial page load
- **Data Freshness**: Real-time updates within 5s
- **Error Recovery**: Automatic retry and recovery
- **Offline Functionality**: Basic functionality without backend

---

## 📝 **Implementation Notes**

### **Critical Success Factors**
1. **Preserve Legacy Functionality**: Every feature must have equivalent in new system
2. **Maintain Real-time Capability**: WebSocket or equivalent real-time updates
3. **Ensure Backward Compatibility**: APIs must support existing backend contracts
4. **Implement Robust Testing**: Comprehensive testing before legacy system retirement

### **Risk Mitigation**
1. **Parallel Implementation**: Build new system alongside legacy system
2. **Incremental Migration**: Migrate components one service at a time
3. **Feature Flags**: Enable progressive rollout of new API system
4. **Rollback Plan**: Ability to quickly revert to legacy system if needed

---

**Status**: Ready for Implementation  
**Timeline**: 4 weeks for complete API migration  
**Dependencies**: Backend service availability, WebSocket infrastructure  
**Success Criteria**: 100% functional parity with legacy system