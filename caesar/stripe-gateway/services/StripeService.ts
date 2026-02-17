// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import Stripe from 'stripe';
import { config } from '@/config';
import { 
  FiatDepositRequest, 
  GATEMintResponse, 
  FiatWithdrawalRequest, 
  USDTransferResponse,
  TransactionFees,
  BankAccountDetails,
  User,
  GatewayError,
  ErrorCodes
} from '@/types';
import { Logger } from '@/utils/Logger';
import { DecimalJS } from 'decimal.js';

export class StripeService {
  private stripe: Stripe;
  private logger: Logger;

  constructor() {
    this.stripe = new Stripe(config.stripe.secretKey, {
      apiVersion: '2023-10-16',
      typescript: true,
    });
    this.logger = new Logger('StripeService');
  }

  /**
   * Process fiat deposit and initiate CAESAR token minting
   */
  async processFiatDeposit(request: FiatDepositRequest, user: User): Promise<GATEMintResponse> {
    try {
      this.logger.info('Processing fiat deposit', { 
        userId: request.userId, 
        amount: request.amount,
        currency: request.currency 
      });

      // Validate request
      await this.validateDepositRequest(request, user);

      // Calculate fees
      const fees = this.calculateDepositFees(request.amount, request.currency);
      
      // Calculate CAESAR tokens to mint (1:1 with USD after fees)
      const netAmount = request.amount - fees.totalFee;
      const gateTokensToMint = new DecimalJS(netAmount).mul(1e18).toString(); // Convert to wei

      // Create Stripe PaymentIntent
      const paymentIntent = await this.createPaymentIntent({
        amount: Math.round(request.amount * 100), // Convert to cents
        currency: request.currency.toLowerCase(),
        customer: user.stripeCustomerId,
        payment_method: request.paymentMethodId,
        confirmation_method: 'automatic',
        confirm: true,
        metadata: {
          userId: request.userId,
          destinationChain: request.destinationChain.toString(),
          destinationAddress: request.destinationAddress,
          gateTokensToMint,
          transactionType: 'FIAT_TO_CRYPTO',
        },
      });

      // Store transaction in database (would be implemented in TransactionService)
      const transactionId = await this.createDepositTransaction({
        userId: request.userId,
        amount: request.amount,
        currency: request.currency,
        gateTokensToMint,
        fees,
        stripePaymentIntentId: paymentIntent.id,
        destinationChain: request.destinationChain,
        destinationAddress: request.destinationAddress,
        metadata: request.metadata,
      });

      return {
        transactionId,
        gateTokensToMint,
        estimatedGasPrice: '20000000000', // 20 gwei - would be dynamic
        layerZeroFee: '0.001', // Would be calculated from LayerZero
        totalFees: fees,
        estimatedCompletionTime: 300, // 5 minutes
        stripePaymentIntent: paymentIntent,
      };

    } catch (error) {
      this.logger.error('Failed to process fiat deposit', { error, request });
      throw this.handleStripeError(error);
    }
  }

  /**
   * Process fiat withdrawal from CAESAR tokens
   */
  async processFiatWithdrawal(request: FiatWithdrawalRequest, user: User): Promise<USDTransferResponse> {
    try {
      this.logger.info('Processing fiat withdrawal', {
        userId: request.userId,
        gateAmount: request.gateAmount,
        currency: request.currency
      });

      // Validate request
      await this.validateWithdrawalRequest(request, user);

      // Calculate exchange rate (1 GATE = 1 USD minus fees)
      const exchangeRate = 1.0;
      const gateAmountDecimal = new DecimalJS(request.gateAmount).div(1e18);
      const grossFiatAmount = gateAmountDecimal.mul(exchangeRate).toNumber();

      // Calculate fees
      const fees = this.calculateWithdrawalFees(grossFiatAmount, request.currency);
      const netFiatAmount = grossFiatAmount - fees.totalFee;

      // Create connected account if needed
      if (!user.stripeConnectAccountId) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'User must have verified Stripe Connect account for withdrawals',
          400
        );
      }

      // Create bank account for payout if not exists
      const bankAccount = await this.createOrUpdateBankAccount(
        user.stripeConnectAccountId,
        request.bankAccount
      );

      // Create transfer to connected account
      const transfer = await this.stripe.transfers.create({
        amount: Math.round(netFiatAmount * 100), // Convert to cents
        currency: request.currency.toLowerCase(),
        destination: user.stripeConnectAccountId,
        metadata: {
          userId: request.userId,
          gateAmount: request.gateAmount,
          sourceChain: request.sourceChain.toString(),
          transactionType: 'CRYPTO_TO_FIAT',
        },
      });

      // Create payout to bank account
      await this.stripe.payouts.create({
        amount: Math.round(netFiatAmount * 100),
        currency: request.currency.toLowerCase(),
        method: 'instant', // or 'standard' based on user preference
        metadata: {
          transferId: transfer.id,
          userId: request.userId,
        },
      }, {
        stripeAccount: user.stripeConnectAccountId,
      });

      // Store transaction in database
      const transactionId = await this.createWithdrawalTransaction({
        userId: request.userId,
        gateAmount: request.gateAmount,
        fiatAmount: grossFiatAmount,
        currency: request.currency,
        fees,
        stripeTransferId: transfer.id,
        sourceChain: request.sourceChain,
        bankAccount: request.bankAccount,
        metadata: request.metadata,
      });

      return {
        transactionId,
        fiatAmount: netFiatAmount,
        exchangeRate,
        totalFees: fees,
        estimatedTransferTime: 86400, // 24 hours for bank transfer
        stripeTransfer: transfer,
      };

    } catch (error) {
      this.logger.error('Failed to process fiat withdrawal', { error, request });
      throw this.handleStripeError(error);
    }
  }

  /**
   * Create Stripe Connect account for user
   */
  async createConnectAccount(user: User): Promise<Stripe.Account> {
    try {
      const account = await this.stripe.accounts.create({
        type: 'express',
        country: 'US', // Would be dynamic based on user location
        email: user.email,
        capabilities: {
          card_payments: { requested: true },
          transfers: { requested: true },
        },
        business_type: 'individual',
        metadata: {
          userId: user.id,
          walletAddress: user.walletAddress,
        },
      });

      // Update user record with Stripe Connect account ID
      await this.updateUserStripeConnectId(user.id, account.id);

      return account;
    } catch (error) {
      this.logger.error('Failed to create Stripe Connect account', { error, userId: user.id });
      throw this.handleStripeError(error);
    }
  }

  /**
   * Create account link for Connect onboarding
   */
  async createAccountLink(accountId: string, returnUrl: string, refreshUrl: string): Promise<Stripe.AccountLink> {
    try {
      return await this.stripe.accountLinks.create({
        account: accountId,
        refresh_url: refreshUrl,
        return_url: returnUrl,
        type: 'account_onboarding',
      });
    } catch (error) {
      this.logger.error('Failed to create account link', { error, accountId });
      throw this.handleStripeError(error);
    }
  }

  /**
   * Get account status and capabilities
   */
  async getAccountStatus(accountId: string): Promise<{
    chargesEnabled: boolean;
    payoutsEnabled: boolean;
    detailsSubmitted: boolean;
    requirements: Stripe.Account.Requirements;
  }> {
    try {
      const account = await this.stripe.accounts.retrieve(accountId);
      
      return {
        chargesEnabled: account.charges_enabled || false,
        payoutsEnabled: account.payouts_enabled || false,
        detailsSubmitted: account.details_submitted || false,
        requirements: account.requirements || {},
      };
    } catch (error) {
      this.logger.error('Failed to get account status', { error, accountId });
      throw this.handleStripeError(error);
    }
  }

  /**
   * Process webhook events
   */
  async processWebhookEvent(event: Stripe.Event): Promise<void> {
    try {
      this.logger.info('Processing webhook event', { type: event.type, id: event.id });

      switch (event.type) {
        case 'payment_intent.succeeded':
          await this.handlePaymentIntentSucceeded(event.data.object as Stripe.PaymentIntent);
          break;
        
        case 'payment_intent.payment_failed':
          await this.handlePaymentIntentFailed(event.data.object as Stripe.PaymentIntent);
          break;
        
        case 'transfer.created':
          await this.handleTransferCreated(event.data.object as Stripe.Transfer);
          break;
        
        case 'payout.paid':
          await this.handlePayoutPaid(event.data.object as Stripe.Payout);
          break;
        
        case 'account.updated':
          await this.handleAccountUpdated(event.data.object as Stripe.Account);
          break;
        
        default:
          this.logger.info('Unhandled webhook event type', { type: event.type });
      }
    } catch (error) {
      this.logger.error('Failed to process webhook event', { error, eventId: event.id });
      throw error;
    }
  }

  /**
   * Calculate deposit fees
   */
  private calculateDepositFees(amount: number, currency: string): TransactionFees {
    const basePercentage = config.fees.deposit.base;
    const minimumFee = config.fees.deposit.minimum;
    const maximumFee = config.fees.deposit.maximum;

    const percentageFee = amount * basePercentage;
    const gatewayFee = Math.max(minimumFee, Math.min(percentageFee, maximumFee));
    
    // Stripe fee (2.9% + $0.30 for cards)
    const stripeFee = (amount * 0.029) + 0.30;
    
    // Network fee (estimated)
    const networkFee = 0.50;

    const totalFee = gatewayFee + stripeFee + networkFee;

    return {
      stripeFee,
      gatewayFee,
      networkFee,
      totalFee,
      currency,
    };
  }

  /**
   * Calculate withdrawal fees
   */
  private calculateWithdrawalFees(amount: number, currency: string): TransactionFees {
    const basePercentage = config.fees.withdrawal.base;
    const minimumFee = config.fees.withdrawal.minimum;
    const maximumFee = config.fees.withdrawal.maximum;
    const flatFee = config.fees.withdrawal.flatFee;

    const percentageFee = amount * basePercentage;
    const gatewayFee = Math.max(minimumFee, Math.min(percentageFee, maximumFee)) + flatFee;
    
    // Stripe payout fee
    const stripeFee = 0.25; // $0.25 for standard bank transfer
    
    // Network fee for burning CAESAR tokens
    const networkFee = 1.00;

    const totalFee = gatewayFee + stripeFee + networkFee;

    return {
      stripeFee,
      gatewayFee,
      networkFee,
      totalFee,
      currency,
    };
  }

  /**
   * Validate deposit request
   */
  private async validateDepositRequest(request: FiatDepositRequest, user: User): Promise<void> {
    // Check minimum/maximum limits
    if (request.amount < config.limits.deposit.min) {
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        `Minimum deposit amount is $${config.limits.deposit.min}`,
        400
      );
    }

    if (request.amount > config.limits.deposit.max) {
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        `Maximum deposit amount is $${config.limits.deposit.max}`,
        400
      );
    }

    // Check KYC requirements
    const kycLimits = this.getKYCLimits(user.kycLevel);
    if (request.amount > kycLimits.single) {
      throw new GatewayError(
        ErrorCodes.KYC_REQUIRED,
        'Higher KYC level required for this transaction amount',
        403
      );
    }

    // Check daily limits
    const todayVolume = await this.getDailyVolume(user.id);
    if (todayVolume + request.amount > kycLimits.daily) {
      throw new GatewayError(
        ErrorCodes.RATE_LIMIT_EXCEEDED,
        'Daily transaction limit exceeded',
        429
      );
    }
  }

  /**
   * Validate withdrawal request
   */
  private async validateWithdrawalRequest(request: FiatWithdrawalRequest, user: User): Promise<void> {
    const gateAmountDecimal = new DecimalJS(request.gateAmount).div(1e18);
    const fiatAmount = gateAmountDecimal.toNumber();

    // Check minimum/maximum limits
    if (fiatAmount < config.limits.withdrawal.min) {
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        `Minimum withdrawal amount is $${config.limits.withdrawal.min}`,
        400
      );
    }

    if (fiatAmount > config.limits.withdrawal.max) {
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        `Maximum withdrawal amount is $${config.limits.withdrawal.max}`,
        400
      );
    }

    // Check KYC requirements
    const kycLimits = this.getKYCLimits(user.kycLevel);
    if (fiatAmount > kycLimits.single) {
      throw new GatewayError(
        ErrorCodes.KYC_REQUIRED,
        'Higher KYC level required for this transaction amount',
        403
      );
    }
  }

  /**
   * Webhook handlers
   */
  private async handlePaymentIntentSucceeded(paymentIntent: Stripe.PaymentIntent): Promise<void> {
    const { userId, gateTokensToMint, destinationChain, destinationAddress } = paymentIntent.metadata;
    
    // Trigger CAESAR token minting process
    await this.triggerGATEMinting({
      userId,
      amount: gateTokensToMint,
      destinationChain: parseInt(destinationChain),
      destinationAddress,
      stripePaymentIntentId: paymentIntent.id,
    });
  }

  private async handlePaymentIntentFailed(paymentIntent: Stripe.PaymentIntent): Promise<void> {
    // Update transaction status to failed
    // Trigger refund if necessary
    // Send notification to user
    this.logger.error('Payment intent failed', { paymentIntentId: paymentIntent.id });
  }

  private async handleTransferCreated(transfer: Stripe.Transfer): Promise<void> {
    // Update transaction status
    // Trigger CAESAR token burning process
    const { userId, gateAmount, sourceChain } = transfer.metadata;
    
    await this.triggerGATEBurning({
      userId,
      amount: gateAmount,
      sourceChain: parseInt(sourceChain),
      stripeTransferId: transfer.id,
    });
  }

  private async handlePayoutPaid(payout: Stripe.Payout): Promise<void> {
    // Mark withdrawal as completed
    // Send confirmation to user
    this.logger.info('Payout completed', { payoutId: payout.id });
  }

  private async handleAccountUpdated(account: Stripe.Account): Promise<void> {
    // Update user account status
    // Check if onboarding is complete
    // Enable/disable features based on account status
    this.logger.info('Account updated', { accountId: account.id });
  }

  /**
   * Helper methods (would be implemented with actual database/service calls)
   */
  private async createPaymentIntent(params: Stripe.PaymentIntentCreateParams): Promise<Stripe.PaymentIntent> {
    return this.stripe.paymentIntents.create(params);
  }

  private async createOrUpdateBankAccount(accountId: string, bankDetails: BankAccountDetails): Promise<Stripe.ExternalAccount> {
    // Implementation would handle bank account creation/update
    return {} as Stripe.ExternalAccount;
  }

  private async createDepositTransaction(params: any): Promise<string> {
    // Implementation would store transaction in database
    return 'txn_' + Date.now();
  }

  private async createWithdrawalTransaction(params: any): Promise<string> {
    // Implementation would store transaction in database
    return 'txn_' + Date.now();
  }

  private async updateUserStripeConnectId(userId: string, accountId: string): Promise<void> {
    // Implementation would update user record
  }

  private async getDailyVolume(userId: string): Promise<number> {
    // Implementation would calculate user's daily volume
    return 0;
  }

  private getKYCLimits(kycLevel: any): { daily: number; single: number } {
    return config.limits.kycLimits[`level${kycLevel}`] || config.limits.kycLimits.level0;
  }

  private async triggerGATEMinting(params: any): Promise<void> {
    // Implementation would trigger LayerZero minting
    this.logger.info('Triggering GATE minting', params);
  }

  private async triggerGATEBurning(params: any): Promise<void> {
    // Implementation would trigger LayerZero burning
    this.logger.info('Triggering GATE burning', params);
  }

  private handleStripeError(error: any): GatewayError {
    if (error.type === 'StripeError') {
      switch (error.code) {
        case 'card_declined':
          return new GatewayError(ErrorCodes.TRANSACTION_FAILED, 'Payment was declined', 400);
        case 'insufficient_funds':
          return new GatewayError(ErrorCodes.INSUFFICIENT_FUNDS, 'Insufficient funds', 400);
        case 'processing_error':
          return new GatewayError(ErrorCodes.TRANSACTION_FAILED, 'Processing error occurred', 500);
        default:
          return new GatewayError(ErrorCodes.SYSTEM_ERROR, error.message, 500);
      }
    }
    
    return new GatewayError(ErrorCodes.SYSTEM_ERROR, 'An unexpected error occurred', 500);
  }
}