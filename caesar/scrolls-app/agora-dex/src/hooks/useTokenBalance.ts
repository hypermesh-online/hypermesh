// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useState, useEffect } from 'react';
import { ethers } from 'ethers';

const ERC20_ABI = [
  'function balanceOf(address owner) view returns (uint256)',
  'function decimals() view returns (uint8)',
  'function symbol() view returns (string)',
  'function name() view returns (string)',
];

export const useTokenBalance = (
  tokenAddress: string | null,
  accountAddress: string | null,
  provider: any
) => {
  const [balance, setBalance] = useState<string>('0');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!tokenAddress || !accountAddress || !provider) {
      setBalance('0');
      return;
    }

    const fetchBalance = async () => {
      setLoading(true);
      setError(null);
      
      try {
        if (tokenAddress === 'ETH') {
          // Native ETH balance
          const ethBalance = await provider.getBalance(accountAddress);
          setBalance(ethers.formatEther(ethBalance));
        } else {
          // ERC20 token balance
          const contract = new ethers.Contract(tokenAddress, ERC20_ABI, provider);
          const tokenBalance = await contract.balanceOf(accountAddress);
          const decimals = await contract.decimals();
          setBalance(ethers.formatUnits(tokenBalance, decimals));
        }
      } catch (err) {
        console.error('Error fetching balance:', err);
        setError('Failed to fetch balance');
        setBalance('0');
      } finally {
        setLoading(false);
      }
    };

    fetchBalance();

    // Refresh balance every 10 seconds
    const interval = setInterval(fetchBalance, 10000);
    return () => clearInterval(interval);
  }, [tokenAddress, accountAddress, provider]);

  return { balance, loading, error };
};