// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { describe, it, expect } from 'vitest';
import {
  getSystemHealthSummary,
  formatPerformanceMetrics,
  calculateUptimePercentage,
  getCertificateExpiryWarning,
  WEB3_CONFIG
} from '@/lib/api';

describe('API Utility Functions', () => {
  describe('getSystemHealthSummary', () => {
    it('returns unknown status when systemStatus is undefined', () => {
      const result = getSystemHealthSummary(undefined);
      expect(result.status).toBe('unknown');
      expect(result.score).toBe(0);
      expect(result.summary).toBe('System status unavailable');
    });

    it('returns excellent when all services are healthy', () => {
      const status = {
        overall: 'healthy',
        services: {
          trustchain: { name: 'TrustChain', status: 'healthy', responseTime: 10, uptime: 99.9, lastCheck: new Date().toISOString() },
          stoq: { name: 'STOQ', status: 'healthy', responseTime: 15, uptime: 99.8, lastCheck: new Date().toISOString() },
          hypermesh: { name: 'HyperMesh', status: 'healthy', responseTime: 12, uptime: 99.7, lastCheck: new Date().toISOString() },
          integration: { name: 'Integration', status: 'healthy', responseTime: 8, uptime: 99.9, lastCheck: new Date().toISOString() },
        },
        performance: { avgResponseTime: 11.25, errorRate: 0.01, uptime: 99.82, totalRequests: 50000 }
      } as any;

      const result = getSystemHealthSummary(status);
      expect(result.status).toBe('excellent');
      expect(result.score).toBe(100);
      expect(result.summary).toBe('All systems operational');
    });

    it('returns good when 75-89% of services are healthy', () => {
      const status = {
        overall: 'degraded',
        services: {
          trustchain: { name: 'TrustChain', status: 'healthy', responseTime: 10, uptime: 99.9, lastCheck: new Date().toISOString() },
          stoq: { name: 'STOQ', status: 'healthy', responseTime: 15, uptime: 99.8, lastCheck: new Date().toISOString() },
          hypermesh: { name: 'HyperMesh', status: 'healthy', responseTime: 12, uptime: 99.7, lastCheck: new Date().toISOString() },
          integration: { name: 'Integration', status: 'degraded', responseTime: 500, uptime: 95.0, lastCheck: new Date().toISOString() },
        },
        performance: { avgResponseTime: 134.25, errorRate: 0.5, uptime: 98.6, totalRequests: 50000 }
      } as any;

      const result = getSystemHealthSummary(status);
      expect(result.status).toBe('good');
      expect(result.score).toBe(75);
    });

    it('returns critical when most services are down', () => {
      const status = {
        overall: 'critical',
        services: {
          trustchain: { name: 'TrustChain', status: 'offline', responseTime: 0, uptime: 0, lastCheck: new Date().toISOString() },
          stoq: { name: 'STOQ', status: 'offline', responseTime: 0, uptime: 0, lastCheck: new Date().toISOString() },
          hypermesh: { name: 'HyperMesh', status: 'offline', responseTime: 0, uptime: 0, lastCheck: new Date().toISOString() },
          integration: { name: 'Integration', status: 'offline', responseTime: 0, uptime: 0, lastCheck: new Date().toISOString() },
        },
        performance: { avgResponseTime: 0, errorRate: 100, uptime: 0, totalRequests: 0 }
      } as any;

      const result = getSystemHealthSummary(status);
      expect(result.status).toBe('critical');
      expect(result.score).toBe(0);
      expect(result.summary).toBe('Critical system failures detected');
    });
  });

  describe('formatPerformanceMetrics', () => {
    it('returns simulated metrics when undefined', () => {
      const result = formatPerformanceMetrics(undefined);
      expect(result.throughput).toBe('2.95 Gbps');
      expect(result.latency).toBe('35.2 ms');
      expect(result.efficiency).toBe('7.4%');
      expect(result.packetLoss).toBe('0.02%');
    });

    it('formats real metrics correctly', () => {
      const metrics = {
        throughput: {
          upload: 1000,
          download: 2500.7,
          efficiency: 85.3
        },
        latency: {
          rtt: 25.6,
          jitter: 2.1,
          packetLoss: 0.05
        }
      } as any;

      const result = formatPerformanceMetrics(metrics);
      expect(result.throughput).toBe('2500.7 Mbps');
      expect(result.latency).toBe('25.6 ms');
      expect(result.efficiency).toBe('85.3%');
      expect(result.packetLoss).toBe('0.05%');
    });
  });

  describe('calculateUptimePercentage', () => {
    it('formats uptime with 2 decimal places', () => {
      expect(calculateUptimePercentage(99.999)).toBe('100.00%');
      expect(calculateUptimePercentage(99.95)).toBe('99.95%');
      expect(calculateUptimePercentage(0)).toBe('0.00%');
    });
  });

  describe('getCertificateExpiryWarning', () => {
    it('returns critical for certs expiring in 7 days or less', () => {
      const soon = new Date();
      soon.setDate(soon.getDate() + 5);
      expect(getCertificateExpiryWarning(soon.toISOString())).toBe('critical');
    });

    it('returns warning for certs expiring in 8-30 days', () => {
      const mediumSoon = new Date();
      mediumSoon.setDate(mediumSoon.getDate() + 15);
      expect(getCertificateExpiryWarning(mediumSoon.toISOString())).toBe('warning');
    });

    it('returns none for certs valid beyond 30 days', () => {
      const far = new Date();
      far.setDate(far.getDate() + 60);
      expect(getCertificateExpiryWarning(far.toISOString())).toBe('none');
    });

    it('returns critical for already expired certs', () => {
      const past = new Date();
      past.setDate(past.getDate() - 5);
      expect(getCertificateExpiryWarning(past.toISOString())).toBe('critical');
    });
  });

  describe('WEB3_CONFIG', () => {
    it('has valid endpoint configuration', () => {
      expect(WEB3_CONFIG.ENDPOINTS.TRUSTCHAIN).toBeDefined();
      expect(WEB3_CONFIG.ENDPOINTS.STOQ).toBeDefined();
      expect(WEB3_CONFIG.ENDPOINTS.HYPERMESH).toBeDefined();
      expect(WEB3_CONFIG.ENDPOINTS.INTEGRATION).toBeDefined();
    });

    it('has valid performance targets', () => {
      expect(WEB3_CONFIG.PERFORMANCE.TARGET_THROUGHPUT).toBe(40000);
      expect(WEB3_CONFIG.PERFORMANCE.MAX_LATENCY).toBe(100);
      expect(WEB3_CONFIG.PERFORMANCE.MAX_PACKET_LOSS).toBe(1);
      expect(WEB3_CONFIG.PERFORMANCE.MIN_UPTIME).toBe(99.9);
    });

    it('has valid timeout configuration', () => {
      expect(WEB3_CONFIG.TIMEOUTS.API_REQUEST).toBe(5000);
      expect(WEB3_CONFIG.TIMEOUTS.WEBSOCKET_CONNECT).toBe(10000);
      expect(WEB3_CONFIG.TIMEOUTS.PING_INTERVAL).toBe(30000);
      expect(WEB3_CONFIG.TIMEOUTS.RECONNECT_INTERVAL).toBe(5000);
    });

    it('has valid retry configuration', () => {
      expect(WEB3_CONFIG.RETRIES.API_REQUESTS).toBe(3);
      expect(WEB3_CONFIG.RETRIES.WEBSOCKET_CONNECT).toBe(5);
      expect(WEB3_CONFIG.RETRIES.MAX_BACKOFF).toBe(30000);
    });
  });
});
