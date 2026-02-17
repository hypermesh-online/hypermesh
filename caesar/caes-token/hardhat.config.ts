// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { HardhatUserConfig } from 'hardhat/config';
import '@nomicfoundation/hardhat-toolbox';
import 'dotenv/config';

const config: HardhatUserConfig = {
  solidity: {
    version: '0.8.22',
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
      viaIR: true,
    },
  },
  networks: {
    // Ethereum Mainnet
    ethereum: {
      url: process.env.ETH_RPC_URL || 'https://mainnet.infura.io/v3/YOUR_INFURA_KEY',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 1,
    },
    // Ethereum Sepolia Testnet
    sepolia: {
      url: process.env.ETH_TESTNET_RPC || 'https://sepolia.infura.io/v3/YOUR_INFURA_KEY',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 11155111,
    },
    // Polygon Mainnet
    polygon: {
      url: process.env.POLYGON_RPC_URL || 'https://polygon-mainnet.infura.io/v3/YOUR_INFURA_KEY',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 137,
    },
    // Polygon Mumbai Testnet
    mumbai: {
      url: process.env.POLYGON_MUMBAI_RPC || 'https://polygon-mumbai.infura.io/v3/YOUR_INFURA_KEY',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 80001,
    },
    // BNB Smart Chain
    bsc: {
      url: 'https://bsc-dataseed1.binance.org/',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 56,
    },
    // BNB Smart Chain Testnet
    bscTestnet: {
      url: 'https://data-seed-prebsc-1-s1.binance.org:8545/',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 97,
    },
    // Arbitrum One
    arbitrum: {
      url: 'https://arb1.arbitrum.io/rpc',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 42161,
    },
    // Arbitrum Sepolia
    arbitrumSepolia: {
      url: 'https://sepolia-rollup.arbitrum.io/rpc',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 421614,
    },
    // Optimism
    optimism: {
      url: 'https://mainnet.optimism.io',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 10,
    },
    // Optimism Sepolia
    optimismSepolia: {
      url: 'https://sepolia.optimism.io',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 11155420,
    },
    // Base
    base: {
      url: 'https://mainnet.base.org',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 8453,
    },
    // Base Sepolia
    baseSepolia: {
      url: 'https://sepolia.base.org',
      accounts: process.env.MNEMONIC ? 
        { mnemonic: process.env.MNEMONIC } : 
        [],
      chainId: 84532,
    },
  },
  etherscan: {
    apiKey: {
      mainnet: process.env.ETHERSCAN_API_KEY || '',
      sepolia: process.env.ETHERSCAN_API_KEY || '',
      polygon: process.env.POLYGONSCAN_API_KEY || '',
      polygonMumbai: process.env.POLYGONSCAN_API_KEY || '',
      bsc: process.env.BSCSCAN_API_KEY || '',
      bscTestnet: process.env.BSCSCAN_API_KEY || '',
      arbitrumOne: process.env.ARBISCAN_API_KEY || '',
      arbitrumSepolia: process.env.ARBISCAN_API_KEY || '',
      optimisticEthereum: process.env.OPTIMISM_API_KEY || '',
      optimisticSepolia: process.env.OPTIMISM_API_KEY || '',
      base: process.env.BASESCAN_API_KEY || '',
      baseSepolia: process.env.BASESCAN_API_KEY || '',
    },
  },
  gasReporter: {
    enabled: process.env.REPORT_GAS !== undefined,
    currency: 'USD',
  },
};

export default config;