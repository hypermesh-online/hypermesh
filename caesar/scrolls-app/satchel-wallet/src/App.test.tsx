// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import App from './App';
import AppWagmiProvider from './providers/WagmiProvider';
import { ToastProvider } from './providers/ToastProvider';
import { ErrorBoundary } from './components/ErrorStates';

const renderWithProviders = (component: React.ReactElement) => {
  return render(
    <ErrorBoundary>
      <AppWagmiProvider>
        <ToastProvider>
          {component}
        </ToastProvider>
      </AppWagmiProvider>
    </ErrorBoundary>
  );
};

describe('App', () => {
  it('renders wallet interface when not connected', () => {
    renderWithProviders(<App />);
    expect(screen.getByRole('heading', { name: 'Wallet' })).toBeInTheDocument();
    // Should show wallet not connected state
    expect(screen.getByText('Wallet Not Connected')).toBeInTheDocument();
    expect(screen.getByText('Please connect your wallet to continue.')).toBeInTheDocument();
  });

  it('navigates between tabs', () => {
    renderWithProviders(<App />);
    
    // Click on Cards tab
    fireEvent.click(screen.getByRole('button', { name: /cards/i }));
    expect(screen.getByText('Add Money')).toBeInTheDocument();
    
    // Click on Activity tab
    fireEvent.click(screen.getByRole('button', { name: /activity/i }));
    expect(screen.getByText('Recent Activity')).toBeInTheDocument();
    
    // Click back to Wallet tab (use getAllByRole and select the navigation one)
    const walletButtons = screen.getAllByRole('button', { name: /wallet/i });
    const walletNavButton = walletButtons.find(button => 
      button.querySelector('span')?.textContent === 'Wallet'
    );
    fireEvent.click(walletNavButton!);
    expect(screen.getByText('Wallet Not Connected')).toBeInTheDocument();
  });

  it('shows connect wallet button', () => {
    renderWithProviders(<App />);
    expect(screen.getByText('Connect Wallet')).toBeInTheDocument();
  });
});