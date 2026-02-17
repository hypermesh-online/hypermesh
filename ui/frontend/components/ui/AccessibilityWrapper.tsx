// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useRef } from 'react';
import { cn } from '@/lib/utils';

interface AccessibilityWrapperProps {
  children: React.ReactNode;
  role?: string;
  ariaLabel?: string;
  ariaDescribedBy?: string;
  focusable?: boolean;
  onFocus?: () => void;
  onBlur?: () => void;
  className?: string;
  id?: string;
}

export function AccessibilityWrapper({
  children,
  role,
  ariaLabel,
  ariaDescribedBy,
  focusable = false,
  onFocus,
  onBlur,
  className,
  id,
}: AccessibilityWrapperProps) {
  const elementRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = elementRef.current;
    if (!element) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Handle Enter and Space key for focusable elements
      if (focusable && (event.key === 'Enter' || event.key === ' ')) {
        event.preventDefault();
        const clickEvent = new MouseEvent('click', {
          bubbles: true,
          cancelable: true,
        });
        element.dispatchEvent(clickEvent);
      }
    };

    if (focusable) {
      element.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      if (focusable) {
        element.removeEventListener('keydown', handleKeyDown);
      }
    };
  }, [focusable]);

  return (
    <div
      ref={elementRef}
      id={id}
      role={role}
      aria-label={ariaLabel}
      aria-describedby={ariaDescribedBy}
      tabIndex={focusable ? 0 : undefined}
      onFocus={onFocus}
      onBlur={onBlur}
      className={cn(
        focusable && 'focus:outline-none focus:ring-2 focus:ring-cyan-400 focus:ring-opacity-50 rounded',
        className
      )}
    >
      {children}
    </div>
  );
}
