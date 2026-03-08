// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import dotenv from 'dotenv';
import { StripeConfig, LayerZeroConfig, ChainConfig } from '@/types';

// Load environment variables
dotenv.config();

export const config = {
  // Server Configuration
  server: {
    port: parseInt(process.env.PORT || '9292', 10),
    environment: process.env.NODE_ENV || 'development',
    apiBaseUrl: process.env.API_BASE_URL || 'http://localhost:9292',
    frontendUrl: process.env.FRONTEND_URL || 'http://localhost:3000',
  },

  // Database Configuration
  database: {
    url: process.env.DATABASE_URL || 'postgresql://user:password@localhost:5432/gateway_stripe',
    redis: {
      url: process.env.REDIS_URL || 'redis://localhost:6379',
      maxRetries: 3,
      retryDelayOnFailover: 100,
    },
  },

  // Stripe Configuration
  stripe: {
    publishableKey: process.env.STRIPE_PUBLISHABLE_KEY!,
    secretKey: process.env.STRIPE_SECRET_KEY!,
    webhookSecret: process.env.STRIPE_WEBHOOK_SECRET!,
    connectClientId: process.env.STRIPE_CONNECT_CLIENT_ID!,
    platformAccountId: process.env.STRIPE_PLATFORM_ACCOUNT_ID!,
    environment: process.env.STRIPE_SECRET_KEY?.startsWith('sk_live') ? 'live' : 'test',
  } as StripeConfig,

  // JWT Configuration
  jwt: {
    secret: process.env.JWT_SECRET!,
    expiresIn: process.env.JWT_EXPIRES_IN || '24h',
    refreshSecret: process.env.REFRESH_TOKEN_SECRET!,
    refreshExpiresIn: process.env.REFRESH_TOKEN_EXPIRES_IN || '30d',
  },

  // LayerZero Configuration
  layerZero: {
    endpointV2Address: process.env.LAYERZERO_ENDPOINT_V2!,
    gateTokenAddress: process.env.GATE_TOKEN_CONTRACT!,
    usdcAddress: process.env.USDC_CONTRACT!,
    supportedChains: [
      {
        chainId: 1,
        name: 'Ethereum',
        rpcUrl: process.env.ETHEREUM_RPC_URL!,
        explorerUrl: 'https://etherscan.io',
        layerZeroChainId: 30101,
        gasMultiplier: 1.2,
        confirmations: 12,
      },
      {
        chainId: 137,
        name: 'Polygon',
        rpcUrl: process.env.POLYGON_RPC_URL!,
        explorerUrl: 'https://polygonscan.com',
        layerZeroChainId: 30109,
        gasMultiplier: 1.5,
        confirmations: 64,
      },
      {
        chainId: 42161,
        name: 'Arbitrum',
        rpcUrl: process.env.ARBITRUM_RPC_URL!,
        explorerUrl: 'https://arbiscan.io',
        layerZeroChainId: 30110,
        gasMultiplier: 1.1,
        confirmations: 1,
      },
      {
        chainId: 10,
        name: 'Optimism',
        rpcUrl: process.env.OPTIMISM_RPC_URL!,
        explorerUrl: 'https://optimistic.etherscan.io',
        layerZeroChainId: 30111,
        gasMultiplier: 1.1,
        confirmations: 1,
      },
      {
        chainId: 8453,
        name: 'Base',
        rpcUrl: process.env.BASE_RPC_URL!,
        explorerUrl: 'https://basescan.org',
        layerZeroChainId: 30184,
        gasMultiplier: 1.1,
        confirmations: 1,
      },
    ] as ChainConfig[],
  } as LayerZeroConfig,

  // Blockchain Configuration
  blockchain: {
    deployerPrivateKey: process.env.DEPLOYER_PRIVATE_KEY!,
    gatewayOperatorKey: process.env.GATEWAY_OPERATOR_KEY!,
    gasLimit: {
      deposit: 300000,
      withdrawal: 400000,
      crossChain: 500000,
    },
    gasPrice: {
      default: '20000000000', // 20 gwei
      priority: '30000000000', // 30 gwei
    },
  },

  // KYC/Compliance Configuration
  kyc: {
    provider: process.env.KYC_PROVIDER || 'jumio',
    jumio: {
      apiToken: process.env.JUMIO_API_TOKEN!,
      apiSecret: process.env.JUMIO_API_SECRET!,
      datacenter: process.env.JUMIO_DATACENTER || 'US',
    },
    sanctionsScreening: {
      apiKey: process.env.SANCTIONS_SCREENING_API_KEY!,
      provider: 'dow-jones',
    },
    riskLevels: {
      low: { maxScore: 25, dailyLimit: 1000 },
      medium: { maxScore: 50, dailyLimit: 5000 },
      high: { maxScore: 75, dailyLimit: 10000 },
      prohibited: { maxScore: 100, dailyLimit: 0 },
    },
  },

  // AWS Configuration
  aws: {
    accessKeyId: process.env.AWS_ACCESS_KEY_ID!,
    secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY!,
    region: process.env.AWS_REGION || 'us-east-1',
    s3: {
      bucketName: process.env.S3_BUCKET_NAME || 'caesar-token-documents',
      documentRetention: 7 * 365 * 24 * 60 * 60 * 1000, // 7 years in ms
    },
  },

  // Security Configuration
  security: {
    bcryptRounds: parseInt(process.env.BCRYPT_ROUNDS || '12', 10),
    corsOrigin: process.env.CORS_ORIGIN?.split(',') || ['http://localhost:3000'],
    enableCors: process.env.ENABLE_CORS === 'true',
    rateLimiting: {
      windowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '900000', 10), // 15 minutes
      maxRequests: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || '100', 10),
    },
    encryption: {
      algorithm: 'aes-256-gcm',
      keyLength: 32,
      ivLength: 16,
    },
  },

  // Transaction Limits
  limits: {
    deposit: {
      min: parseInt(process.env.MIN_DEPOSIT_USD || '1', 10),
      max: parseInt(process.env.MAX_DEPOSIT_USD || '50000', 10),
    },
    withdrawal: {
      min: parseInt(process.env.MIN_WITHDRAWAL_USD || '1', 10),
      max: parseInt(process.env.MAX_WITHDRAWAL_USD || '25000', 10),
    },
    daily: {
      max: parseInt(process.env.DAILY_LIMIT_USD || '100000', 10),
    },
    kycLimits: {
      level0: { daily: 100, monthly: 1000, single: 100 },
      level1: { daily: 1000, monthly: 10000, single: 1000 },
      level2: { daily: 10000, monthly: 100000, single: 10000 },
      level3: { daily: 50000, monthly: 500000, single: 50000 },
    },
  },

  // Fee Structure
  fees: {
    deposit: {
      base: 0.005, // 0.5%
      minimum: 1, // $1 minimum
      maximum: 50, // $50 maximum
    },
    withdrawal: {
      base: 0.01, // 1%
      minimum: 2, // $2 minimum
      maximum: 100, // $100 maximum
      flatFee: 5, // Additional $5 flat fee
    },
    crossChain: {
      base: 0.002, // 0.2%
      minimum: 0.5, // $0.5 minimum
      layerZeroMultiplier: 1.5, // 50% markup on LZ fees
    },
  },

  // Monitoring and Logging
  monitoring: {
    sentryDsn: process.env.SENTRY_DSN,
    logLevel: process.env.LOG_LEVEL || 'info',
    enableMetrics: process.env.ENABLE_METRICS === 'true',
    healthCheck: {
      interval: 30000, // 30 seconds
      timeout: 5000, // 5 seconds
    },
  },

  // Queue Configuration
  queue: {
    redis: {
      host: 'localhost',
      port: 6379,
      db: 1,
    },
    defaultJobOptions: {
      removeOnComplete: 100,
      removeOnFail: 50,
      attempts: 3,
      backoff: {
        type: 'exponential',
        delay: 2000,
      },
    },
    processors: {
      deposit: { concurrency: 5 },
      withdrawal: { concurrency: 3 },
      kyc: { concurrency: 10 },
      compliance: { concurrency: 2 },
      notification: { concurrency: 20 },
    },
  },

  // WebSocket Configuration
  websocket: {
    port: 8443,
    heartbeatInterval: 30000, // 30 seconds
    connectionTimeout: 60000, // 1 minute
    maxConnections: 1000,
  },

  // Compliance Configuration
  compliance: {
    travelRule: {
      threshold: 1000, // $1000 threshold for travel rule
      enabled: true,
    },
    reporting: {
      suspicious: {
        threshold: 10000, // $10,000 threshold for SAR
        enabled: true,
      },
      ctr: {
        threshold: 10000, // $10,000 threshold for CTR
        enabled: true,
      },
    },
    monitoring: {
      velocityThreshold: 50000, // $50,000 in 24h
      patternDetection: true,
      realTimeScreening: true,
    },
  },

  // Circuit Breaker Configuration
  circuitBreaker: {
    thresholds: {
      failureRate: 0.5, // 50% failure rate
      requestVolume: 20,
      timeout: 60000, // 1 minute
    },
    recovery: {
      timeout: 300000, // 5 minutes
      healthCheckInterval: 30000, // 30 seconds
    },
  },

  // Cache Configuration
  cache: {
    defaultTtl: 3600, // 1 hour
    exchangeRates: 300, // 5 minutes
    userProfiles: 1800, // 30 minutes
    transactionStatus: 60, // 1 minute
  },
};

// Validation
export function validateConfig(): void {
  const required = [
    'STRIPE_SECRET_KEY',
    'STRIPE_WEBHOOK_SECRET',
    'JWT_SECRET',
    'DATABASE_URL',
    'LAYERZERO_ENDPOINT_V2',
    'GATE_TOKEN_CONTRACT',
    'ETHEREUM_RPC_URL',
  ];

  const missing = required.filter(key => !process.env[key]);
  
  if (missing.length > 0) {
    throw new Error(`Missing required environment variables: ${missing.join(', ')}`);
  }

  // Additional validations
  if (config.server.port < 1000 || config.server.port > 65535) {
    throw new Error('Invalid port number');
  }

  if (config.security.bcryptRounds < 10 || config.security.bcryptRounds > 15) {
    throw new Error('Bcrypt rounds should be between 10 and 15');
  }

  console.log('✅ Configuration validated successfully');
}

export default config;