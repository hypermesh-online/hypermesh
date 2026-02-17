// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Request, Response, NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { config } from '@/config';
import { AuthenticatedRequest, User, GatewayError, ErrorCodes } from '@/types';
import { Logger } from '@/utils/Logger';

const logger = new Logger('AuthMiddleware');

/**
 * JWT Authentication Middleware
 */
export const authenticateToken = async (
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const authHeader = req.headers.authorization;
    const token = authHeader && authHeader.split(' ')[1]; // Bearer TOKEN

    if (!token) {
      throw new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'Access token required',
        401
      );
    }

    // Verify JWT token
    const decoded = jwt.verify(token, config.jwt.secret) as any;

    // Fetch user from database (simplified)
    const user = await getUserById(decoded.userId);

    if (!user || !user.isActive) {
      throw new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'Invalid or inactive user',
        401
      );
    }

    // Check if user account is locked due to compliance issues
    if (user.complianceFlags && user.complianceFlags.includes('ACCOUNT_LOCKED')) {
      throw new GatewayError(
        ErrorCodes.FORBIDDEN,
        'Account locked due to compliance issues',
        403
      );
    }

    // Attach user to request
    req.user = user;

    // Update last activity
    await updateUserActivity(user.id);

    logger.debug('User authenticated', { userId: user.id });
    next();

  } catch (error) {
    if (error.name === 'JsonWebTokenError') {
      next(new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'Invalid access token',
        401
      ));
    } else if (error.name === 'TokenExpiredError') {
      next(new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'Access token expired',
        401
      ));
    } else {
      next(error);
    }
  }
};

/**
 * KYC Level Requirement Middleware
 */
export const requireKYCLevel = (minLevel: number) => {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction): void => {
    try {
      const user = req.user;

      if (!user) {
        throw new GatewayError(
          ErrorCodes.UNAUTHORIZED,
          'Authentication required',
          401
        );
      }

      if (user.kycLevel < minLevel) {
        throw new GatewayError(
          ErrorCodes.KYC_REQUIRED,
          `KYC Level ${minLevel} or higher required`,
          403,
          {
            currentLevel: user.kycLevel,
            requiredLevel: minLevel,
          }
        );
      }

      next();
    } catch (error) {
      next(error);
    }
  };
};

/**
 * KYC Status Requirement Middleware
 */
export const requireKYCStatus = (allowedStatuses: string[]) => {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction): void => {
    try {
      const user = req.user;

      if (!user) {
        throw new GatewayError(
          ErrorCodes.UNAUTHORIZED,
          'Authentication required',
          401
        );
      }

      if (!allowedStatuses.includes(user.kycStatus)) {
        throw new GatewayError(
          ErrorCodes.KYC_REQUIRED,
          `KYC verification required. Current status: ${user.kycStatus}`,
          403,
          {
            currentStatus: user.kycStatus,
            allowedStatuses,
          }
        );
      }

      next();
    } catch (error) {
      next(error);
    }
  };
};

/**
 * Risk Level Check Middleware
 */
export const checkRiskLevel = (maxRiskScore: number) => {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction): void => {
    try {
      const user = req.user;

      if (!user) {
        throw new GatewayError(
          ErrorCodes.UNAUTHORIZED,
          'Authentication required',
          401
        );
      }

      if (user.riskScore > maxRiskScore) {
        throw new GatewayError(
          ErrorCodes.FORBIDDEN,
          'Transaction not allowed due to risk assessment',
          403,
          {
            riskScore: user.riskScore,
            maxAllowed: maxRiskScore,
          }
        );
      }

      next();
    } catch (error) {
      next(error);
    }
  };
};

/**
 * Transaction Limit Check Middleware
 */
export const checkTransactionLimits = () => {
  return async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user;
      const { amount, currency } = req.body;

      if (!user) {
        throw new GatewayError(
          ErrorCodes.UNAUTHORIZED,
          'Authentication required',
          401
        );
      }

      // Get user's current limits based on KYC level
      const limits = getKYCLimits(user.kycLevel, user.riskScore);

      // Check single transaction limit
      if (amount > limits.singleTransactionMax) {
        throw new GatewayError(
          ErrorCodes.RATE_LIMIT_EXCEEDED,
          'Amount exceeds single transaction limit',
          400,
          {
            amount,
            limit: limits.singleTransactionMax,
            currency,
          }
        );
      }

      // Check daily limits (this would require database queries)
      const dailyVolume = await getUserDailyVolume(user.id);
      const isDeposit = req.path.includes('deposits');
      const dailyLimit = isDeposit ? limits.dailyDeposit : limits.dailyWithdrawal;

      if (dailyVolume + amount > dailyLimit) {
        throw new GatewayError(
          ErrorCodes.RATE_LIMIT_EXCEEDED,
          'Amount exceeds daily transaction limit',
          400,
          {
            amount,
            dailyVolume,
            dailyLimit,
            currency,
          }
        );
      }

      // Check monthly limits
      const monthlyVolume = await getUserMonthlyVolume(user.id);
      const monthlyLimit = isDeposit ? limits.monthlyDeposit : limits.monthlyWithdrawal;

      if (monthlyVolume + amount > monthlyLimit) {
        throw new GatewayError(
          ErrorCodes.RATE_LIMIT_EXCEEDED,
          'Amount exceeds monthly transaction limit',
          400,
          {
            amount,
            monthlyVolume,
            monthlyLimit,
            currency,
          }
        );
      }

      next();
    } catch (error) {
      next(error);
    }
  };
};

/**
 * API Key Authentication (for webhook endpoints)
 */
export const authenticateAPIKey = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const apiKey = req.headers['x-api-key'] as string;

    if (!apiKey) {
      throw new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'API key required',
        401
      );
    }

    // In production, this would validate against stored API keys
    if (!isValidAPIKey(apiKey)) {
      throw new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'Invalid API key',
        401
      );
    }

    next();
  } catch (error) {
    next(error);
  }
};

/**
 * Webhook Authentication (for Stripe webhooks)
 */
export const authenticateWebhook = (req: Request, res: Response, next: NextFunction): void => {
  try {
    const signature = req.headers['stripe-signature'] as string;

    if (!signature) {
      throw new GatewayError(
        ErrorCodes.UNAUTHORIZED,
        'Webhook signature required',
        401
      );
    }

    // Stripe signature verification would be done in the webhook handler
    // This middleware just ensures the header exists
    next();
  } catch (error) {
    next(error);
  }
};

/**
 * Optional Authentication (for public endpoints that can benefit from auth)
 */
export const optionalAuth = async (
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const authHeader = req.headers.authorization;
    const token = authHeader && authHeader.split(' ')[1];

    if (token) {
      const decoded = jwt.verify(token, config.jwt.secret) as any;
      const user = await getUserById(decoded.userId);

      if (user && user.isActive) {
        req.user = user;
        await updateUserActivity(user.id);
      }
    }

    next();
  } catch (error) {
    // Don't fail on optional auth errors, just continue without user
    next();
  }
};

/**
 * Helper functions (would be implemented with actual database calls)
 */
async function getUserById(userId: string): Promise<User | null> {
  // Implementation would fetch user from database
  // This is a simplified mock
  return {
    id: userId,
    email: 'user@example.com',
    walletAddress: '0x123...',
    kycStatus: 'APPROVED' as any,
    kycLevel: 2 as any,
    stripeCustomerId: 'cus_123',
    stripeConnectAccountId: 'acct_123',
    createdAt: new Date(),
    updatedAt: new Date(),
    isActive: true,
    riskScore: 25,
    lastActivity: new Date(),
    complianceFlags: [],
  };
}

async function updateUserActivity(userId: string): Promise<void> {
  // Implementation would update user's last activity in database
  logger.debug('Updated user activity', { userId });
}

function getKYCLimits(kycLevel: number, riskScore: number): {
  dailyDeposit: number;
  dailyWithdrawal: number;
  monthlyDeposit: number;
  monthlyWithdrawal: number;
  singleTransactionMax: number;
} {
  const baseLimits = config.limits.kycLimits[`level${kycLevel}` as keyof typeof config.limits.kycLimits] || 
                     config.limits.kycLimits.level0;

  // Apply risk-based multipliers
  let multiplier = 1.0;
  if (riskScore > 75) multiplier = 0.5;
  else if (riskScore > 50) multiplier = 0.8;

  return {
    dailyDeposit: Math.floor(baseLimits.daily * multiplier),
    dailyWithdrawal: Math.floor(baseLimits.daily * multiplier * 0.8),
    monthlyDeposit: Math.floor(baseLimits.monthly * multiplier),
    monthlyWithdrawal: Math.floor(baseLimits.monthly * multiplier * 0.8),
    singleTransactionMax: Math.floor(baseLimits.single * multiplier),
  };
}

async function getUserDailyVolume(userId: string): Promise<number> {
  // Implementation would calculate user's daily transaction volume
  return 0;
}

async function getUserMonthlyVolume(userId: string): Promise<number> {
  // Implementation would calculate user's monthly transaction volume
  return 0;
}

function isValidAPIKey(apiKey: string): boolean {
  // Implementation would validate API key against database
  // This is a simplified check
  return apiKey.startsWith('gw_') && apiKey.length === 32;
}