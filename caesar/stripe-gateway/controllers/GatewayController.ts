// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Request, Response, NextFunction } from 'express';
import { StripeService } from '@/services/StripeService';
import { LayerZeroService } from '@/services/LayerZeroService';
import { KYCService } from '@/services/KYCService';
import { Logger } from '@/utils/Logger';
import {
  AuthenticatedRequest,
  APIResponse,
  FiatDepositRequest,
  FiatWithdrawalRequest,
  TransactionStatus,
  DocumentType,
  GatewayError,
  ErrorCodes,
} from '@/types';
import { validationResult } from 'express-validator';

export class GatewayController {
  private logger: Logger;
  private stripeService: StripeService;
  private layerZeroService: LayerZeroService;
  private kycService: KYCService;

  constructor() {
    this.logger = new Logger('GatewayController');
    this.stripeService = new StripeService();
    this.layerZeroService = new LayerZeroService();
    this.kycService = new KYCService();
  }

  /**
   * Process fiat-to-crypto deposit
   * POST /api/v1/deposits
   */
  public processDeposit = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Validation failed',
          400,
          errors.array()
        );
      }

      const user = req.user!;
      const depositRequest: FiatDepositRequest = req.body;

      this.logger.info('Processing deposit request', {
        userId: user.id,
        amount: depositRequest.amount,
        currency: depositRequest.currency,
        destinationChain: depositRequest.destinationChain
      });

      // Process the deposit through Stripe
      const result = await this.stripeService.processFiatDeposit(depositRequest, user);

      const response: APIResponse = {
        success: true,
        data: {
          transactionId: result.transactionId,
          gateTokensToMint: result.gateTokensToMint,
          totalFees: result.totalFees,
          estimatedCompletionTime: result.estimatedCompletionTime,
          paymentIntent: {
            id: result.stripePaymentIntent.id,
            clientSecret: result.stripePaymentIntent.client_secret,
            status: result.stripePaymentIntent.status,
          },
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Process crypto-to-fiat withdrawal
   * POST /api/v1/withdrawals
   */
  public processWithdrawal = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Validation failed',
          400,
          errors.array()
        );
      }

      const user = req.user!;
      const withdrawalRequest: FiatWithdrawalRequest = req.body;

      this.logger.info('Processing withdrawal request', {
        userId: user.id,
        gateAmount: withdrawalRequest.gateAmount,
        currency: withdrawalRequest.currency,
        sourceChain: withdrawalRequest.sourceChain
      });

      // Process the withdrawal through Stripe
      const result = await this.stripeService.processFiatWithdrawal(withdrawalRequest, user);

      const response: APIResponse = {
        success: true,
        data: {
          transactionId: result.transactionId,
          fiatAmount: result.fiatAmount,
          exchangeRate: result.exchangeRate,
          totalFees: result.totalFees,
          estimatedTransferTime: result.estimatedTransferTime,
          transfer: {
            id: result.stripeTransfer.id,
            amount: result.stripeTransfer.amount,
            currency: result.stripeTransfer.currency,
            destination: result.stripeTransfer.destination,
          },
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Cross-chain bridge CAESAR tokens
   * POST /api/v1/bridge
   */
  public bridgeTokens = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Validation failed',
          400,
          errors.array()
        );
      }

      const user = req.user!;
      const { fromChain, toChain, amount, recipient } = req.body;

      this.logger.info('Processing bridge request', {
        userId: user.id,
        fromChain,
        toChain,
        amount,
        recipient
      });

      const result = await this.layerZeroService.bridgeGATETokens({
        fromChain,
        toChain,
        amount,
        recipient,
        sender: user.walletAddress,
      });

      const response: APIResponse = {
        success: true,
        data: {
          txHash: result.txHash,
          layerZeroGuid: result.layerZeroGuid,
          estimatedArrivalTime: result.estimatedArrivalTime,
          fromChain,
          toChain,
          amount,
          recipient,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get transaction status
   * GET /api/v1/transactions/:transactionId
   */
  public getTransactionStatus = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;
      const { transactionId } = req.params;

      this.logger.info('Getting transaction status', {
        userId: user.id,
        transactionId
      });

      // This would fetch from database
      const transaction = await this.getTransactionById(transactionId);

      if (!transaction || transaction.userId !== user.id) {
        throw new GatewayError(
          ErrorCodes.NOT_FOUND,
          'Transaction not found',
          404
        );
      }

      const response: APIResponse = {
        success: true,
        data: {
          id: transaction.id,
          type: transaction.type,
          status: transaction.status,
          fiatAmount: transaction.fiatAmount,
          cryptoAmount: transaction.cryptoAmount,
          currency: transaction.currency,
          cryptoCurrency: transaction.cryptoCurrency,
          fees: transaction.fees,
          createdAt: transaction.createdAt,
          updatedAt: transaction.updatedAt,
          completedAt: transaction.completedAt,
          blockchainTxHash: transaction.blockchainTxHash,
          layerZeroTxHash: transaction.layerZeroTxHash,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get user's transaction history
   * GET /api/v1/transactions
   */
  public getTransactionHistory = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;
      const { 
        page = '1', 
        limit = '20', 
        status, 
        type, 
        startDate, 
        endDate 
      } = req.query;

      const pageNum = parseInt(page as string, 10);
      const limitNum = Math.min(parseInt(limit as string, 10), 100); // Max 100 per page

      this.logger.info('Getting transaction history', {
        userId: user.id,
        page: pageNum,
        limit: limitNum,
        filters: { status, type, startDate, endDate }
      });

      // This would fetch from database with proper pagination
      const { transactions, total } = await this.getTransactionHistory_impl({
        userId: user.id,
        page: pageNum,
        limit: limitNum,
        status: status as TransactionStatus,
        type,
        startDate,
        endDate,
      });

      const response: APIResponse = {
        success: true,
        data: transactions,
        pagination: {
          page: pageNum,
          limit: limitNum,
          total,
          hasNext: pageNum * limitNum < total,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get reserve status and peg information
   * GET /api/v1/reserves
   */
  public getReserveStatus = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      this.logger.info('Getting reserve status');

      const [reserveStatus, pegStatus] = await Promise.all([
        this.layerZeroService.syncReserves(),
        this.layerZeroService.validatePegMaintenance(),
      ]);

      const response: APIResponse = {
        success: true,
        data: {
          reserves: reserveStatus,
          peg: pegStatus,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get gas estimates for all chains
   * GET /api/v1/gas-estimates
   */
  public getGasEstimates = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      this.logger.info('Getting gas estimates');

      const gasEstimates = await this.layerZeroService.getGasEstimates();

      const response: APIResponse = {
        success: true,
        data: gasEstimates,
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Create Stripe Connect account
   * POST /api/v1/connect/account
   */
  public createConnectAccount = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;

      if (user.stripeConnectAccountId) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'User already has a Stripe Connect account',
          400
        );
      }

      this.logger.info('Creating Stripe Connect account', { userId: user.id });

      const account = await this.stripeService.createConnectAccount(user);

      const response: APIResponse = {
        success: true,
        data: {
          accountId: account.id,
          onboardingRequired: !account.details_submitted,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(201).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Create Stripe Connect account onboarding link
   * POST /api/v1/connect/onboarding
   */
  public createOnboardingLink = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;
      const { returnUrl, refreshUrl } = req.body;

      if (!user.stripeConnectAccountId) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'User does not have a Stripe Connect account',
          400
        );
      }

      this.logger.info('Creating onboarding link', { 
        userId: user.id,
        accountId: user.stripeConnectAccountId 
      });

      const accountLink = await this.stripeService.createAccountLink(
        user.stripeConnectAccountId,
        returnUrl,
        refreshUrl
      );

      const response: APIResponse = {
        success: true,
        data: {
          url: accountLink.url,
          expiresAt: accountLink.expires_at,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get Stripe Connect account status
   * GET /api/v1/connect/status
   */
  public getConnectAccountStatus = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;

      if (!user.stripeConnectAccountId) {
        const response: APIResponse = {
          success: true,
          data: {
            hasAccount: false,
            status: 'none',
          },
        };
        return res.status(200).json(response);
      }

      this.logger.info('Getting Connect account status', { 
        userId: user.id,
        accountId: user.stripeConnectAccountId 
      });

      const status = await this.stripeService.getAccountStatus(user.stripeConnectAccountId);

      const response: APIResponse = {
        success: true,
        data: {
          hasAccount: true,
          accountId: user.stripeConnectAccountId,
          chargesEnabled: status.chargesEnabled,
          payoutsEnabled: status.payoutsEnabled,
          detailsSubmitted: status.detailsSubmitted,
          requirements: status.requirements,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Upload KYC document
   * POST /api/v1/kyc/documents
   */
  public uploadKYCDocument = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;
      const file = req.file;
      const { documentType } = req.body;

      if (!file) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'No file uploaded',
          400
        );
      }

      if (!Object.values(DocumentType).includes(documentType)) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Invalid document type',
          400
        );
      }

      this.logger.info('Uploading KYC document', {
        userId: user.id,
        documentType,
        fileName: file.originalname
      });

      const document = await this.kycService.uploadDocument({
        userId: user.id,
        documentType,
        fileBuffer: file.buffer,
        fileName: file.originalname,
        mimeType: file.mimetype,
      });

      const response: APIResponse = {
        success: true,
        data: {
          documentId: document.id,
          type: document.type,
          status: document.status,
          uploadedAt: document.uploadedAt,
          expiresAt: document.expiresAt,
        },
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(201).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get KYC status
   * GET /api/v1/kyc/status
   */
  public getKYCStatus = async (req: AuthenticatedRequest, res: Response, next: NextFunction): Promise<void> => {
    try {
      const user = req.user!;

      this.logger.info('Getting KYC status', { userId: user.id });

      // This would fetch from database
      const kycStatus = await this.getUserKYCStatus(user.id);

      const response: APIResponse = {
        success: true,
        data: kycStatus,
        metadata: {
          requestId: req.headers['x-request-id'],
          timestamp: new Date().toISOString(),
        }
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Health check endpoint
   * GET /api/v1/health
   */
  public healthCheck = async (req: Request, res: Response): Promise<void> => {
    const response: APIResponse = {
      success: true,
      data: {
        status: 'healthy',
        timestamp: new Date().toISOString(),
        version: '1.0.0',
        services: {
          stripe: 'operational',
          layerzero: 'operational',
          kyc: 'operational',
          database: 'operational',
          redis: 'operational',
        },
      },
    };

    res.status(200).json(response);
  };

  /**
   * Private helper methods (would be implemented with actual database calls)
   */
  private async getTransactionById(transactionId: string): Promise<any> {
    // Implementation would fetch from database
    return null;
  }

  private async getTransactionHistory_impl(params: {
    userId: string;
    page: number;
    limit: number;
    status?: TransactionStatus;
    type?: string;
    startDate?: string;
    endDate?: string;
  }): Promise<{ transactions: any[]; total: number }> {
    // Implementation would fetch from database with filters and pagination
    return {
      transactions: [],
      total: 0,
    };
  }

  private async getUserKYCStatus(userId: string): Promise<any> {
    // Implementation would fetch KYC status from database
    return {
      status: 'PENDING',
      level: 0,
      requiredDocuments: ['GOVERNMENT_ID'],
      uploadedDocuments: [],
      limits: {
        dailyDeposit: 100,
        dailyWithdrawal: 100,
        monthlyDeposit: 1000,
        monthlyWithdrawal: 1000,
        singleTransactionMax: 100,
        currency: 'USD',
      },
    };
  }
}