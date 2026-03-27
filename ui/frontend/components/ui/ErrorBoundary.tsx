// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { Component, type ReactNode } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from './card';
import { Button } from './button';
import { AlertTriangle } from 'lucide-react';

interface ErrorBoundaryProps {
  children: ReactNode;
  module?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(
      `[ErrorBoundary${this.props.module ? `:${this.props.module}` : ''}]`,
      error,
      info.componentStack,
    );
  }

  private handleRetry = () => {
    this.setState({ hasError: false, error: undefined });
  };

  render() {
    if (this.state.hasError) {
      return (
        <Card className="m-4 border-red-500/30 bg-red-500/5">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-red-400">
              <AlertTriangle className="h-5 w-5" />
              {this.props.module
                ? `${this.props.module} Error`
                : 'Something went wrong'}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-gray-400 mb-4">
              {this.state.error?.message || 'An unexpected error occurred'}
            </p>
            <Button variant="outline" size="sm" onClick={this.handleRetry}>
              Retry
            </Button>
          </CardContent>
        </Card>
      );
    }
    return this.props.children;
  }
}
