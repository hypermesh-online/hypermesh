// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export function calculateDaysUntilExpiry(validTo: string): number {
  return Math.ceil((new Date(validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
}

export function isExpiringSoon(validTo: string, warningDays: number = 30): boolean {
  return calculateDaysUntilExpiry(validTo) <= warningDays;
}

export function isExpired(validTo: string): boolean {
  return calculateDaysUntilExpiry(validTo) <= 0;
}

export function formatUptime(milliseconds: number): string {
  const days = Math.floor(milliseconds / (1000 * 60 * 60 * 24));
  const hours = Math.floor((milliseconds % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  return `${days}d ${hours}h`;
}