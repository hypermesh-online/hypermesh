// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { describe, it, expect } from 'vitest';
import { cn } from '@/lib/utils';

describe('cn utility', () => {
  it('merges class names correctly', () => {
    const result = cn('text-red-500', 'bg-blue-500');
    expect(result).toContain('text-red-500');
    expect(result).toContain('bg-blue-500');
  });

  it('handles conditional classes', () => {
    const isActive = true;
    const result = cn('base-class', isActive && 'active-class');
    expect(result).toContain('base-class');
    expect(result).toContain('active-class');
  });

  it('filters out falsy values', () => {
    const result = cn('base-class', false, null, undefined, 'end-class');
    expect(result).toContain('base-class');
    expect(result).toContain('end-class');
    expect(result).not.toContain('false');
    expect(result).not.toContain('null');
    expect(result).not.toContain('undefined');
  });

  it('resolves Tailwind conflicts by keeping last', () => {
    // tailwind-merge should resolve conflicts
    const result = cn('p-4', 'p-6');
    expect(result).toBe('p-6');
  });

  it('handles empty arguments', () => {
    const result = cn();
    expect(result).toBe('');
  });

  it('handles single class', () => {
    const result = cn('single-class');
    expect(result).toBe('single-class');
  });

  it('merges arrays of classes', () => {
    const result = cn(['class-a', 'class-b']);
    expect(result).toContain('class-a');
    expect(result).toContain('class-b');
  });
});
