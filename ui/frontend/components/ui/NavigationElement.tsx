// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useRef } from 'react';
import { cn } from '@/lib/utils';
import { useKeyboardNavigation } from './KeyboardNavigationProvider';

interface NavigationElementProps {
  children: React.ReactNode;
  id: string;
  order: number;
  onActivate?: () => void;
  className?: string;
  ariaLabel?: string;
  role?: string;
  disabled?: boolean;
}

export function NavigationElement({
  children,
  id,
  order,
  onActivate,
  className,
  ariaLabel,
  role = 'button',
  disabled = false,
}: NavigationElementProps) {
  const elementRef = useRef<HTMLDivElement>(null);
  const {
    isKeyboardNavigation,
    focusedElement,
    setFocusedElement,
    registerNavigationElement,
    unregisterNavigationElement,
  } = useKeyboardNavigation();

  const isFocused = focusedElement === id;

  useEffect(() => {
    if (!disabled) {
      registerNavigationElement(id, order);
    }

    return () => {
      unregisterNavigationElement(id);
    };
  }, [id, order, disabled, registerNavigationElement, unregisterNavigationElement]);

  useEffect(() => {
    if (isFocused && elementRef.current && isKeyboardNavigation) {
      elementRef.current.focus();
    }
  }, [isFocused, isKeyboardNavigation]);

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (disabled) return;

    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onActivate?.();
    }
  };

  const handleClick = () => {
    if (disabled) return;
    setFocusedElement(id);
    onActivate?.();
  };

  const handleFocus = () => {
    if (!disabled) {
      setFocusedElement(id);
    }
  };

  return (
    <div
      ref={elementRef}
      role={role}
      tabIndex={disabled ? -1 : 0}
      aria-label={ariaLabel}
      aria-disabled={disabled}
      onKeyDown={handleKeyDown}
      onClick={handleClick}
      onFocus={handleFocus}
      className={cn(
        'transition-all duration-200',
        !disabled && 'cursor-pointer',
        isFocused && isKeyboardNavigation && 'ring-2 ring-cyan-400 ring-opacity-60 ring-offset-2 ring-offset-black',
        disabled && 'opacity-50 cursor-not-allowed',
        className
      )}
    >
      {children}
    </div>
  );
}
