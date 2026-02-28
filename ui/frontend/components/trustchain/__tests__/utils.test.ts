// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { describe, it, expect } from 'vitest';
import { getStatusColor, getTypeColor, getTrustLevelColor } from '../utils/statusHelpers';
import { calculateDaysUntilExpiry, isExpiringSoon, isExpired, formatUptime } from '../utils/dateFormatters';
import { algorithmInfo, commonExtensions } from '../utils/algorithmInfo';

describe('TrustChain Utility Functions', () => {
  describe('statusHelpers', () => {
    describe('getStatusColor', () => {
      it('returns correct color for Connected status', () => {
        const result = getStatusColor('Connected');
        expect(result).toContain('text-green-400');
        expect(result).toContain('bg-green-500/20');
      });

      it('returns correct color for Connecting status', () => {
        const result = getStatusColor('Connecting');
        expect(result).toContain('text-yellow-400');
      });

      it('returns correct color for Disconnected status', () => {
        const result = getStatusColor('Disconnected');
        expect(result).toContain('text-gray-400');
      });

      it('returns correct color for Error status', () => {
        const result = getStatusColor('Error');
        expect(result).toContain('text-red-400');
      });

      it('returns correct color for online status', () => {
        const result = getStatusColor('online');
        expect(result).toContain('text-green-400');
      });

      it('returns correct color for active status', () => {
        const result = getStatusColor('active');
        expect(result).toContain('text-green-400');
      });

      it('returns correct color for expired status', () => {
        const result = getStatusColor('expired');
        expect(result).toContain('text-red-400');
      });

      it('returns correct color for revoked status', () => {
        const result = getStatusColor('revoked');
        expect(result).toContain('text-red-400');
      });

      it('returns fallback color for unknown status', () => {
        const result = getStatusColor('unknown-status');
        expect(result).toContain('bg-gray-500/20');
        expect(result).toContain('text-gray-400');
      });
    });

    describe('getTypeColor', () => {
      it('returns correct color for Public type', () => {
        const result = getTypeColor('Public');
        expect(result).toContain('text-cyan-400');
      });

      it('returns correct color for P2P type', () => {
        const result = getTypeColor('P2P');
        expect(result).toContain('text-purple-400');
      });

      it('returns correct color for Federated type', () => {
        const result = getTypeColor('Federated');
        expect(result).toContain('text-blue-400');
      });

      it('returns fallback color for unknown type', () => {
        const result = getTypeColor('Unknown');
        expect(result).toContain('text-gray-400');
      });
    });

    describe('getTrustLevelColor', () => {
      it('returns green for root level', () => {
        expect(getTrustLevelColor('root')).toBe('text-green-400');
      });

      it('returns blue for intermediate level', () => {
        expect(getTrustLevelColor('intermediate')).toBe('text-blue-400');
      });

      it('returns purple for end-entity level', () => {
        expect(getTrustLevelColor('end-entity')).toBe('text-purple-400');
      });

      it('returns gray for unknown level', () => {
        expect(getTrustLevelColor('unknown')).toBe('text-gray-400');
      });
    });
  });

  describe('dateFormatters', () => {
    describe('calculateDaysUntilExpiry', () => {
      it('returns positive days for future date', () => {
        const futureDate = new Date();
        futureDate.setDate(futureDate.getDate() + 30);
        const result = calculateDaysUntilExpiry(futureDate.toISOString());
        expect(result).toBeGreaterThanOrEqual(29);
        expect(result).toBeLessThanOrEqual(31);
      });

      it('returns negative or zero days for past date', () => {
        const pastDate = new Date();
        pastDate.setDate(pastDate.getDate() - 5);
        const result = calculateDaysUntilExpiry(pastDate.toISOString());
        expect(result).toBeLessThanOrEqual(0);
      });

      it('returns approximately 0 for today', () => {
        const today = new Date();
        const result = calculateDaysUntilExpiry(today.toISOString());
        expect(result).toBeLessThanOrEqual(1);
        expect(result).toBeGreaterThanOrEqual(0);
      });
    });

    describe('isExpiringSoon', () => {
      it('returns true for date within warning threshold', () => {
        const soonDate = new Date();
        soonDate.setDate(soonDate.getDate() + 15);
        expect(isExpiringSoon(soonDate.toISOString(), 30)).toBe(true);
      });

      it('returns false for date beyond warning threshold', () => {
        const laterDate = new Date();
        laterDate.setDate(laterDate.getDate() + 60);
        expect(isExpiringSoon(laterDate.toISOString(), 30)).toBe(false);
      });

      it('returns true for already expired date', () => {
        const pastDate = new Date();
        pastDate.setDate(pastDate.getDate() - 5);
        expect(isExpiringSoon(pastDate.toISOString())).toBe(true);
      });

      it('uses default 30-day warning threshold', () => {
        const within30 = new Date();
        within30.setDate(within30.getDate() + 20);
        expect(isExpiringSoon(within30.toISOString())).toBe(true);

        const beyond30 = new Date();
        beyond30.setDate(beyond30.getDate() + 45);
        expect(isExpiringSoon(beyond30.toISOString())).toBe(false);
      });
    });

    describe('isExpired', () => {
      it('returns true for past date', () => {
        const pastDate = new Date();
        pastDate.setDate(pastDate.getDate() - 1);
        expect(isExpired(pastDate.toISOString())).toBe(true);
      });

      it('returns false for future date', () => {
        const futureDate = new Date();
        futureDate.setDate(futureDate.getDate() + 30);
        expect(isExpired(futureDate.toISOString())).toBe(false);
      });
    });

    describe('formatUptime', () => {
      it('formats days and hours correctly', () => {
        // 2 days and 5 hours in milliseconds
        const ms = (2 * 24 * 60 * 60 * 1000) + (5 * 60 * 60 * 1000);
        expect(formatUptime(ms)).toBe('2d 5h');
      });

      it('formats zero uptime', () => {
        expect(formatUptime(0)).toBe('0d 0h');
      });

      it('formats less than a day', () => {
        const ms = 12 * 60 * 60 * 1000; // 12 hours
        expect(formatUptime(ms)).toBe('0d 12h');
      });

      it('formats large values', () => {
        const ms = 365 * 24 * 60 * 60 * 1000; // 365 days
        expect(formatUptime(ms)).toBe('365d 0h');
      });
    });
  });

  describe('algorithmInfo', () => {
    it('contains FALCON-1024 algorithm info', () => {
      const falcon = algorithmInfo['FALCON-1024'];
      expect(falcon).toBeDefined();
      expect(falcon.name).toBe('FALCON-1024');
      expect(falcon.type).toBe('Post-Quantum Digital Signature');
      expect(falcon.security).toBe('NIST Level 5');
      expect(falcon.keySize).toBe('1024 bits');
      expect(falcon.description).toContain('quantum-resistant');
    });

    it('contains Kyber-768 algorithm info', () => {
      const kyber = algorithmInfo['Kyber-768'];
      expect(kyber).toBeDefined();
      expect(kyber.name).toBe('Kyber-768');
      expect(kyber.type).toBe('Post-Quantum Key Encapsulation');
      expect(kyber.security).toBe('NIST Level 3');
    });

    it('contains RSA-2048 algorithm info', () => {
      const rsa = algorithmInfo['RSA-2048'];
      expect(rsa).toBeDefined();
      expect(rsa.security).toBe('Legacy');
      expect(rsa.description).toContain('not quantum-resistant');
    });

    it('has color and bgColor for all algorithms', () => {
      for (const [, info] of Object.entries(algorithmInfo)) {
        expect(info.color).toBeTruthy();
        expect(info.bgColor).toBeTruthy();
      }
    });
  });

  describe('commonExtensions', () => {
    it('maps standard OIDs to human-readable names', () => {
      expect(commonExtensions['2.5.29.15']).toBe('Key Usage');
      expect(commonExtensions['2.5.29.37']).toBe('Extended Key Usage');
      expect(commonExtensions['2.5.29.17']).toBe('Subject Alternative Name');
      expect(commonExtensions['2.5.29.19']).toBe('Basic Constraints');
      expect(commonExtensions['2.5.29.14']).toBe('Subject Key Identifier');
      expect(commonExtensions['2.5.29.35']).toBe('Authority Key Identifier');
    });

    it('contains Authority Information Access OID', () => {
      expect(commonExtensions['1.3.6.1.5.5.7.1.1']).toBe('Authority Information Access');
    });

    it('contains Certificate Policies OID', () => {
      expect(commonExtensions['2.5.29.32']).toBe('Certificate Policies');
    });
  });
});
