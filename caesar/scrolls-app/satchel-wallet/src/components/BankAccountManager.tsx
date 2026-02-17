// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { 
  CreditCard, 
  DollarSign, 
  RefreshCw, 
  Send, 
  Eye, 
  EyeOff,
  AlertTriangle,
  ArrowUpRight,
  ArrowDownLeft,
  Shield,
  Zap,
  TrendingDown
} from 'lucide-react';
import PlaidLink from './PlaidLink';
import { Card, Typography, Button, Flex, Grid, Badge, Animated, Reveal } from '../design-system';

interface BankAccount {
  account_id: string;
  name: string;
  type: string;
  subtype: string;
  mask?: string;
  balances: {
    available: number | null;
    current: number | null;
    iso_currency_code: string;
  };
}

interface Transaction {
  transaction_id: string;
  account_id: string;
  amount: number;
  iso_currency_code: string;
  date: string;
  name: string;
  merchant_name?: string;
  category: string[];
}

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3003';

const BankAccountManager: React.FC = () => {
  const [accounts, setAccounts] = useState<BankAccount[]>([]);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showBalances, setShowBalances] = useState(false);
  const [selectedAccount, setSelectedAccount] = useState<string | null>(null);
  const [transferAmount, setTransferAmount] = useState('');
  const [isConnected, setIsConnected] = useState(false);

  // Check if user has connected accounts
  useEffect(() => {
    checkConnectedAccounts();
  }, []);

  const checkConnectedAccounts = async () => {
    try {
      setLoading(true);
      const response = await fetch(`${API_BASE_URL}/api/plaid/accounts`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ userId: 'default_user' }),
      });

      if (response.ok) {
        const data = await response.json();
        setAccounts(data.accounts);
        setIsConnected(data.accounts.length > 0);
      }
    } catch (err) {
      console.log('No accounts connected yet');
    } finally {
      setLoading(false);
    }
  };

  const handlePlaidSuccess = async (publicToken: string, _metadata: any) => {
    try {
      setLoading(true);
      setError(null);

      const response = await fetch(`${API_BASE_URL}/api/plaid/exchange_public_token`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          public_token: publicToken,
          userId: 'default_user',
        }),
      });

      const data = await response.json();

      if (response.ok) {
        setAccounts(data.accounts);
        setIsConnected(true);
      } else {
        setError(data.error || 'Failed to connect bank account');
      }
    } catch (err) {
      setError('Network error. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const loadTransactions = async () => {
    try {
      setLoading(true);
      const response = await fetch(`${API_BASE_URL}/api/plaid/transactions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ userId: 'default_user', count: 20 }),
      });

      const data = await response.json();
      if (response.ok) {
        setTransactions(data.transactions);
      } else {
        setError(data.error || 'Failed to load transactions');
      }
    } catch (err) {
      setError('Failed to load transactions');
    } finally {
      setLoading(false);
    }
  };

  const handleTransfer = async () => {
    if (!selectedAccount || !transferAmount) {
      setError('Please select an account and enter an amount');
      return;
    }

    try {
      setLoading(true);
      const response = await fetch(`${API_BASE_URL}/api/plaid/transfer`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          userId: 'default_user',
          accountId: selectedAccount,
          amount: parseFloat(transferAmount),
          description: 'Caesar Token Purchase',
        }),
      });

      const data = await response.json();
      if (response.ok) {
        alert('Transfer initiated successfully!');
        setTransferAmount('');
        setSelectedAccount(null);
      } else {
        setError(data.error || 'Transfer failed');
      }
    } catch (err) {
      setError('Transfer failed');
    } finally {
      setLoading(false);
    }
  };

  const formatCurrency = (amount: number, currency: string = 'USD') => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
    }).format(Math.abs(amount));
  };

  if (!isConnected) {
    return (
      <Animated animation="fadeUp">
        <div className="max-w-3xl mx-auto">
          <Card variant="glass" className="text-center p-8">
            {/* Caesar Economic Model Hero */}
            <Animated animation="scaleIn" delay={200}>
              <div className="p-4 rounded-3xl bg-gradient-caesar mx-auto w-24 h-24 flex items-center justify-center mb-6">
                <CreditCard className="text-background-primary" size={36} />
              </div>
            </Animated>
            
            <Typography variant="h1" gradient className="mb-4 font-display">
              Connect Bank Account
            </Typography>
            
            <Typography variant="bodyLarge" color="secondary" className="mb-8 max-w-2xl mx-auto">
              Securely connect your bank account to purchase Caesar tokens directly. 
              Experience the future of utility-based currency with anti-speculation economics.
            </Typography>

            {/* Caesar Benefits */}
            <Card variant="outlined" className="mb-8 p-6 border-caesar-gold/30 bg-caesar-gold/5">
              <Typography variant="h3" color="gold" className="mb-4">
                Why Caesar Tokens?
              </Typography>
              <Grid cols={3} gap="lg" responsive={{ md: 1, lg: 3 }}>
                <Flex direction="col" align="center" gap="sm">
                  <TrendingDown className="text-caesar-gold" size={24} />
                  <Typography variant="bodySmall" color="secondary" className="text-center">
                    Demurrage incentivizes spending over hoarding
                  </Typography>
                </Flex>
                <Flex direction="col" align="center" gap="sm">
                  <Zap className="text-caesar-gold" size={24} />
                  <Typography variant="bodySmall" color="secondary" className="text-center">
                    Utility-focused economics promote real activity
                  </Typography>
                </Flex>
                <Flex direction="col" align="center" gap="sm">
                  <Shield className="text-caesar-gold" size={24} />
                  <Typography variant="bodySmall" color="secondary" className="text-center">
                    Anti-speculation design ensures stability
                  </Typography>
                </Flex>
              </Grid>
            </Card>
            
            <PlaidLink 
              onSuccess={handlePlaidSuccess}
              onError={(error) => setError(error.error_message || 'Connection failed')}
            />
            
            {error && (
              <Animated animation="fadeUp" delay={100}>
                <Card variant="outlined" className="mt-6 p-4 border-semantic-danger/30 bg-semantic-danger/5">
                  <Flex align="start" gap="sm">
                    <AlertTriangle className="text-semantic-danger flex-shrink-0 mt-0.5" size={20} />
                    <div>
                      <Typography variant="bodyMedium" color="danger" weight="medium">
                        Connection Error
                      </Typography>
                      <Typography variant="bodySmall" color="danger" className="mt-1">
                        {error}
                      </Typography>
                    </div>
                  </Flex>
                </Card>
              </Animated>
            )}
          </Card>
        </div>
      </Animated>
    );
  }

  return (
    <div className="max-w-6xl mx-auto space-y-8">
      {/* Header */}
      <Animated animation="slideDown">
        <Flex justify="between" align="center">
          <div>
            <Typography variant="h1" gradient className="font-display">
              Bank Accounts
            </Typography>
            <Typography variant="bodyLarge" color="secondary">
              Traditional banking gateway to Caesar token ecosystem
            </Typography>
          </div>
          <Flex align="center" gap="md">
            <Button
              variant="ghost"
              onClick={() => setShowBalances(!showBalances)}
              icon={showBalances ? <EyeOff size={18} /> : <Eye size={18} />}
            >
              {showBalances ? 'Hide' : 'Show'} Balances
            </Button>
            <Button
              variant="secondary"
              onClick={checkConnectedAccounts}
              loading={loading}
              icon={<RefreshCw size={18} />}
            >
              Refresh
            </Button>
          </Flex>
        </Flex>
      </Animated>

      {/* Connected Accounts */}
      <Grid cols={2} gap="lg" responsive={{ md: 1, lg: 2 }}>
        {accounts.map((account, index) => (
          <Reveal key={account.account_id} animation="slideInLeft" delay={index * 100}>
            <Card variant="glass" className="p-6 hover:bg-white/8 transition-all duration-300">
              <Flex justify="between" align="start" className="mb-4">
                <Flex align="center" gap="md">
                  <div className="p-3 rounded-xl bg-gradient-caesar">
                    <CreditCard className="text-background-primary" size={24} />
                  </div>
                  <div>
                    <Typography variant="h3" weight="semibold">
                      {account.name}
                    </Typography>
                    <Typography variant="bodySmall" color="secondary">
                      {account.type} • {account.subtype}
                      {account.mask && ` • ••••${account.mask}`}
                    </Typography>
                  </div>
                </Flex>
                
                <Badge variant={selectedAccount === account.account_id ? "gold" : "default"}>
                  {selectedAccount === account.account_id ? 'Selected' : 'Available'}
                </Badge>
              </Flex>

              {/* Balance Display */}
              <Card variant="outlined" className="mb-4 p-4">
                <Typography variant="caption" color="secondary" className="mb-2">
                  Available Balance
                </Typography>
                {showBalances && account.balances.available !== null ? (
                  <Typography variant="h2" weight="bold" gradient>
                    {formatCurrency(account.balances.available, account.balances.iso_currency_code)}
                  </Typography>
                ) : (
                  <Typography variant="h2" color="secondary">
                    ••••••••
                  </Typography>
                )}
              </Card>
              
              <Button
                variant={selectedAccount === account.account_id ? "gold" : "primary"}
                fullWidth
                onClick={() => setSelectedAccount(account.account_id)}
                icon={<Send size={18} />}
              >
                {selectedAccount === account.account_id ? 'Selected for Purchase' : 'Select for Caesar Purchase'}
              </Button>
            </Card>
          </Reveal>
        ))}
      </Grid>

      {/* Caesar Purchase Section */}
      {selectedAccount && (
        <Reveal animation="fadeUp">
          <Card variant="glass" className="p-8 bg-gradient-to-br from-caesar-gold/5 via-transparent to-imperial-purple/5">
            <Typography variant="h2" gradient className="mb-6 font-display">
              Purchase Caesar Tokens
            </Typography>
            
            {/* Anti-Speculation Notice */}
            <Card variant="outlined" className="mb-6 p-4 border-caesar-gold/30 bg-caesar-gold/5">
              <Flex align="start" gap="sm">
                <TrendingDown className="text-caesar-gold flex-shrink-0 mt-1" size={20} />
                <div>
                  <Typography variant="bodyMedium" weight="medium" color="gold">
                    Utility-First Currency
                  </Typography>
                  <Typography variant="bodySmall" color="secondary" className="mt-1">
                    Caesar tokens are designed for active use, not speculation. Demurrage encourages spending and economic velocity.
                  </Typography>
                </div>
              </Flex>
            </Card>

            <Grid cols={3} gap="lg" responsive={{ md: 1, lg: 3 }}>
              <div className="lg:col-span-2">
                <Typography variant="bodyMedium" weight="medium" className="mb-3">
                  Purchase Amount (USD)
                </Typography>
                <input
                  type="number"
                  value={transferAmount}
                  onChange={(e) => setTransferAmount(e.target.value)}
                  placeholder="0.00"
                  className="input-field text-lg"
                  min="1"
                  max="10000"
                />
                <Typography variant="caption" color="secondary" className="mt-2 block">
                  Minimum: $1.00 • Maximum: $10,000.00
                </Typography>
              </div>
              
              <div className="flex items-end">
                <Button
                  variant="gold"
                  fullWidth
                  onClick={handleTransfer}
                  loading={loading}
                  disabled={!transferAmount || parseFloat(transferAmount) < 1}
                  icon={loading ? <RefreshCw size={18} /> : <DollarSign size={18} />}
                  size="lg"
                >
                  {loading ? 'Processing...' : 'Buy Caesar Tokens'}
                </Button>
              </div>
            </Grid>

            {transferAmount && parseFloat(transferAmount) >= 1 && (
              <Card variant="terminal" className="mt-6 p-4">
                <Typography variant="bodySmall" family="mono" color="gold">
                  Estimated: {transferAmount} USD → {(parseFloat(transferAmount) / 2.15).toFixed(4)} CAESAR
                </Typography>
                <Typography variant="caption" family="mono" color="secondary">
                  Rate: 1 CAESAR = $2.15 USD (gold-pegged)
                </Typography>
              </Card>
            )}
          </Card>
        </Reveal>
      )}

      {/* Recent Transactions */}
      <Card variant="glass" className="p-6">
        <Flex justify="between" align="center" className="mb-6">
          <Typography variant="h2">Recent Bank Activity</Typography>
          <Button
            variant="ghost"
            onClick={loadTransactions}
            loading={loading}
          >
            Load Transactions
          </Button>
        </Flex>
        
        {transactions.length > 0 ? (
          <div className="space-y-3">
            {transactions.slice(0, 5).map((transaction, index) => (
              <Animated key={transaction.transaction_id} animation="fadeUp" delay={index * 50}>
                <Card variant="outlined" hover className="p-4">
                  <Flex justify="between" align="center">
                    <Flex align="center" gap="md">
                      <div className={`p-2 rounded-lg ${
                        transaction.amount > 0 ? 'bg-semantic-danger/20' : 'bg-semantic-success/20'
                      }`}>
                        {transaction.amount > 0 ? (
                          <ArrowUpRight className="text-semantic-danger" size={18} />
                        ) : (
                          <ArrowDownLeft className="text-semantic-success" size={18} />
                        )}
                      </div>
                      <div>
                        <Typography variant="bodyMedium" weight="medium">
                          {transaction.merchant_name || transaction.name}
                        </Typography>
                        <Typography variant="bodySmall" color="secondary">
                          {transaction.category.join(', ')} • {transaction.date}
                        </Typography>
                      </div>
                    </Flex>
                    <Typography variant="bodyLarge" weight="semibold" className={
                      transaction.amount > 0 ? 'text-semantic-danger' : 'text-semantic-success'
                    }>
                      {transaction.amount > 0 ? '-' : '+'}
                      {formatCurrency(Math.abs(transaction.amount), transaction.iso_currency_code)}
                    </Typography>
                  </Flex>
                </Card>
              </Animated>
            ))}
          </div>
        ) : (
          <div className="text-center py-12">
            <Typography variant="bodyLarge" color="secondary">
              No transactions loaded. Click "Load Transactions" to view recent activity.
            </Typography>
          </div>
        )}
      </Card>

      {error && (
        <Animated animation="fadeUp">
          <Card variant="outlined" className="p-4 border-semantic-danger/30 bg-semantic-danger/5">
            <Flex align="start" gap="sm">
              <AlertTriangle className="text-semantic-danger flex-shrink-0" size={20} />
              <Typography variant="bodyMedium" color="danger">
                {error}
              </Typography>
            </Flex>
          </Card>
        </Animated>
      )}
    </div>
  );
};

export default BankAccountManager;