// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { createContext, useContext, useEffect, useState } from 'react';

interface KeyboardNavigationContextType {
  isKeyboardNavigation: boolean;
  focusedElement: string | null;
  setFocusedElement: (elementId: string | null) => void;
  navigateToNext: () => void;
  navigateToPrevious: () => void;
  registerNavigationElement: (elementId: string, order: number) => void;
  unregisterNavigationElement: (elementId: string) => void;
}

const KeyboardNavigationContext = createContext<KeyboardNavigationContextType | null>(null);

interface NavigationElement {
  id: string;
  order: number;
}

interface KeyboardNavigationProviderProps {
  children: React.ReactNode;
}

export function KeyboardNavigationProvider({ children }: KeyboardNavigationProviderProps) {
  const [isKeyboardNavigation, setIsKeyboardNavigation] = useState(false);
  const [focusedElement, setFocusedElement] = useState<string | null>(null);
  const [navigationElements, setNavigationElements] = useState<NavigationElement[]>([]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Detect keyboard navigation
      if (event.key === 'Tab' || event.key === 'ArrowUp' || event.key === 'ArrowDown' || 
          event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
        setIsKeyboardNavigation(true);
      }

      // Handle arrow key navigation
      if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
        event.preventDefault();
        navigateToNext();
      } else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
        event.preventDefault();
        navigateToPrevious();
      }

      // Handle Escape key to exit navigation
      if (event.key === 'Escape') {
        setFocusedElement(null);
        setIsKeyboardNavigation(false);
      }
    };

    const handleMouseDown = () => {
      setIsKeyboardNavigation(false);
    };

    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleMouseDown);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleMouseDown);
    };
  }, [navigationElements, focusedElement]);

  const navigateToNext = () => {
    const sortedElements = [...navigationElements].sort((a, b) => a.order - b.order);
    if (sortedElements.length === 0) return;

    if (!focusedElement) {
      setFocusedElement(sortedElements[0].id);
      return;
    }

    const currentIndex = sortedElements.findIndex(el => el.id === focusedElement);
    const nextIndex = (currentIndex + 1) % sortedElements.length;
    setFocusedElement(sortedElements[nextIndex].id);
  };

  const navigateToPrevious = () => {
    const sortedElements = [...navigationElements].sort((a, b) => a.order - b.order);
    if (sortedElements.length === 0) return;

    if (!focusedElement) {
      setFocusedElement(sortedElements[sortedElements.length - 1].id);
      return;
    }

    const currentIndex = sortedElements.findIndex(el => el.id === focusedElement);
    const prevIndex = currentIndex === 0 ? sortedElements.length - 1 : currentIndex - 1;
    setFocusedElement(sortedElements[prevIndex].id);
  };

  const registerNavigationElement = (elementId: string, order: number) => {
    setNavigationElements(prev => {
      const filtered = prev.filter(el => el.id !== elementId);
      return [...filtered, { id: elementId, order }];
    });
  };

  const unregisterNavigationElement = (elementId: string) => {
    setNavigationElements(prev => prev.filter(el => el.id !== elementId));
  };

  const value: KeyboardNavigationContextType = {
    isKeyboardNavigation,
    focusedElement,
    setFocusedElement,
    navigateToNext,
    navigateToPrevious,
    registerNavigationElement,
    unregisterNavigationElement,
  };

  return (
    <KeyboardNavigationContext.Provider value={value}>
      {children}
    </KeyboardNavigationContext.Provider>
  );
}

export function useKeyboardNavigation() {
  const context = useContext(KeyboardNavigationContext);
  if (!context) {
    throw new Error('useKeyboardNavigation must be used within a KeyboardNavigationProvider');
  }
  return context;
}
