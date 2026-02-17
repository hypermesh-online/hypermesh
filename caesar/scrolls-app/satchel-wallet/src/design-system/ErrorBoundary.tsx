// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Caesar Design System - Error Boundary Component
 * Professional error handling component
 */

import React from 'react';
import { cn } from './utils';

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false };
  }
  
  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }
  
  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    console.error('Error caught by boundary:', error, errorInfo);
  }
  
  render(): React.ReactNode {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className={cn(
          'p-8 text-center rounded-2xl',
          'bg-background-glass backdrop-blur-xl border border-white/10'
        )}>
          <h2 className="text-xl font-semibold text-semantic-danger mb-2">
            Something went wrong
          </h2>
          <p className="text-neutral-400">
            Please refresh the page and try again.
          </p>
        </div>
      );
    }
    
    return this.props.children;
  }
}