// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Request } from 'express';
import Stripe from 'stripe';
import { BigNumber } from 'ethers';

// User and Account Types
export interface User {
  id: string;
  email: string;
  walletAddress: string;
  kycStatus: KYCStatus;
  kycLevel: KYCLevel;
  stripeCustomerId?: string;
  stripeConnectAccountId?: string;
  createdAt: Date;
  updatedAt: Date;
  isActive: boolean;
  riskScore: number;
  lastActivity: Date;
  complianceFlags: string[];
}

export enum KYCStatus {
  PENDING = 'PENDING',
  IN_PROGRESS = 'IN_PROGRESS',
  APPROVED = 'APPROVED',
  REJECTED = 'REJECTED',
  EXPIRED = 'EXPIRED',
  UNDER_REVIEW = 'UNDER_REVIEW'
}

export enum KYCLevel {
  LEVEL_0 = 0, // No verification - $100 limit
  LEVEL_1 = 1, // Basic verification - $1,000 limit
  LEVEL_2 = 2, // Enhanced verification - $10,000 limit
  LEVEL_3 = 3  // Full verification - $50,000+ limit
}

// Transaction Types
export interface Transaction {
  id: string;
  userId: string;
  type: TransactionType;
  status: TransactionStatus;
  fiatAmount: number;
  cryptoAmount: string;
  currency: string;
  cryptoCurrency: string;
  exchangeRate: number;
  fees: TransactionFees;
  stripePaymentIntentId?: string;
  stripeTransferId?: string;
  blockchainTxHash?: string;
  layerZeroTxHash?: string;
  sourceChain?: number;
  destinationChain?: number;
  createdAt: Date;
  updatedAt: Date;
  completedAt?: Date;
  metadata: Record<string, any>;
}

export enum TransactionType {
  FIAT_TO_CRYPTO = 'FIAT_TO_CRYPTO',
  CRYPTO_TO_FIAT = 'CRYPTO_TO_FIAT',
  CROSS_CHAIN_BRIDGE = 'CROSS_CHAIN_BRIDGE',
  INTERNAL_TRANSFER = 'INTERNAL_TRANSFER'
}

export enum TransactionStatus {
  PENDING = 'PENDING',
  PROCESSING = 'PROCESSING',
  COMPLETED = 'COMPLETED',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED',
  REFUNDED = 'REFUNDED',
  UNDER_REVIEW = 'UNDER_REVIEW'
}

export interface TransactionFees {
  stripeFee: number;
  gatewayFee: number;
  networkFee: number;
  totalFee: number;
  currency: string;
}

// Stripe Gateway Integration Types
export interface StripeGateway {
  // Fiat onramp functionality
  processFiatDeposit(request: FiatDepositRequest): Promise<GATEMintResponse>;
  
  // Fiat offramp functionality  
  processFiatWithdrawal(request: FiatWithdrawalRequest): Promise<USDTransferResponse>;
  
  // Real-time synchronization
  syncReserves(): Promise<ReserveStatus>;
  validatePegMaintenance(): Promise<PegStatus>;
  
  // Compliance automation
  performKYCValidation(user: UserAccount): Promise<KYCValidationResult>;
  monitorTransactions(): Promise<ComplianceReport>;
}

export interface FiatDepositRequest {
  userId: string;
  amount: number;
  currency: string;
  paymentMethodId: string;
  destinationChain: number;
  destinationAddress: string;
  metadata?: Record<string, any>;
}

export interface GATEMintResponse {
  transactionId: string;
  gateTokensToMint: string;
  estimatedGasPrice: string;
  layerZeroFee: string;
  totalFees: TransactionFees;
  estimatedCompletionTime: number;
  stripePaymentIntent: Stripe.PaymentIntent;
}

export interface FiatWithdrawalRequest {
  userId: string;
  gateAmount: string;
  currency: string;
  bankAccount: BankAccountDetails;
  sourceChain: number;
  metadata?: Record<string, any>;
}

export interface USDTransferResponse {
  transactionId: string;
  fiatAmount: number;
  exchangeRate: number;
  totalFees: TransactionFees;
  estimatedTransferTime: number;
  stripeTransfer: Stripe.Transfer;
}

export interface BankAccountDetails {
  accountNumber: string;
  routingNumber: string;
  accountHolderName: string;
  accountType: 'checking' | 'savings';
  bankName?: string;
  country: string;
}

// Reserve and Peg Management
export interface ReserveStatus {
  totalUSDCReserves: string;
  totalGATESupply: string;
  pegRatio: number;
  reserveHealthScore: number;
  lastSyncTime: Date;
  discrepancies: ReserveDiscrepancy[];
}

export interface ReserveDiscrepancy {
  chain: number;
  expectedBalance: string;
  actualBalance: string;
  difference: string;
  severity: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
}

export interface PegStatus {
  currentPeg: number;
  targetPeg: number;
  deviation: number;
  isWithinThreshold: boolean;
  correctionActions: string[];
  lastCorrection: Date;
}

// KYC and Compliance Types
export interface UserAccount {
  id: string;
  email: string;
  walletAddress: string;
  personalInfo: PersonalInfo;
  documents: KYCDocument[];
  riskAssessment: RiskAssessment;
}

export interface PersonalInfo {
  firstName: string;
  lastName: string;
  dateOfBirth: string;
  nationality: string;
  address: Address;
  phoneNumber: string;
  ssn?: string;
  taxId?: string;
}

export interface Address {
  street: string;
  city: string;
  state: string;
  postalCode: string;
  country: string;
}

export interface KYCDocument {
  id: string;
  type: DocumentType;
  status: DocumentStatus;
  s3Key: string;
  uploadedAt: Date;
  expiresAt?: Date;
  verificationResult?: DocumentVerificationResult;
}

export enum DocumentType {
  GOVERNMENT_ID = 'GOVERNMENT_ID',
  PASSPORT = 'PASSPORT',
  DRIVERS_LICENSE = 'DRIVERS_LICENSE',
  PROOF_OF_ADDRESS = 'PROOF_OF_ADDRESS',
  BANK_STATEMENT = 'BANK_STATEMENT',
  SELFIE = 'SELFIE'
}

export enum DocumentStatus {
  UPLOADED = 'UPLOADED',
  PROCESSING = 'PROCESSING',
  VERIFIED = 'VERIFIED',
  REJECTED = 'REJECTED',
  EXPIRED = 'EXPIRED'
}

export interface DocumentVerificationResult {
  isValid: boolean;
  confidence: number;
  extractedData: Record<string, any>;
  flags: string[];
  provider: string;
  timestamp: Date;
}

export interface RiskAssessment {
  score: number;
  level: RiskLevel;
  factors: RiskFactor[];
  sanctions: SanctionsResult;
  pep: PEPResult;
  lastAssessed: Date;
}

export enum RiskLevel {
  LOW = 'LOW',
  MEDIUM = 'MEDIUM',
  HIGH = 'HIGH',
  PROHIBITED = 'PROHIBITED'
}

export interface RiskFactor {
  type: string;
  description: string;
  impact: number;
  severity: 'LOW' | 'MEDIUM' | 'HIGH';
}

export interface SanctionsResult {
  isMatch: boolean;
  lists: string[];
  confidence: number;
  details?: string;
}

export interface PEPResult {
  isPEP: boolean;
  category?: string;
  details?: string;
}

export interface KYCValidationResult {
  userId: string;
  status: KYCStatus;
  level: KYCLevel;
  approvedLimits: TransactionLimits;
  rejectionReason?: string;
  requiredDocuments: DocumentType[];
  completedAt: Date;
}

export interface TransactionLimits {
  dailyDeposit: number;
  dailyWithdrawal: number;
  monthlyDeposit: number;
  monthlyWithdrawal: number;
  singleTransactionMax: number;
  currency: string;
}

// Compliance Monitoring
export interface ComplianceReport {
  reportId: string;
  generatedAt: Date;
  period: {
    startDate: Date;
    endDate: Date;
  };
  metrics: ComplianceMetrics;
  alerts: ComplianceAlert[];
  suspiciousActivities: SuspiciousActivity[];
}

export interface ComplianceMetrics {
  totalTransactions: number;
  totalVolume: number;
  largeTransactions: number;
  rapidTransactions: number;
  crossBorderTransactions: number;
  flaggedUsers: number;
  averageRiskScore: number;
}

export interface ComplianceAlert {
  id: string;
  type: AlertType;
  severity: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  userId: string;
  transactionId?: string;
  description: string;
  triggeredAt: Date;
  status: 'OPEN' | 'INVESTIGATING' | 'RESOLVED' | 'FALSE_POSITIVE';
}

export enum AlertType {
  LARGE_TRANSACTION = 'LARGE_TRANSACTION',
  RAPID_TRANSACTIONS = 'RAPID_TRANSACTIONS',
  UNUSUAL_PATTERN = 'UNUSUAL_PATTERN',
  HIGH_RISK_JURISDICTION = 'HIGH_RISK_JURISDICTION',
  SANCTIONS_MATCH = 'SANCTIONS_MATCH',
  VELOCITY_THRESHOLD = 'VELOCITY_THRESHOLD',
  STRUCTURING = 'STRUCTURING'
}

export interface SuspiciousActivity {
  id: string;
  userId: string;
  pattern: string;
  description: string;
  riskScore: number;
  relatedTransactions: string[];
  detectedAt: Date;
  reportedToAuthorities: boolean;
}

// API Request/Response Types
export interface AuthenticatedRequest extends Request {
  user?: User;
  rateLimit?: {
    limit: number;
    current: number;
    remaining: number;
    resetTime: Date;
  };
}

export interface APIResponse<T = any> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: any;
  };
  pagination?: {
    page: number;
    limit: number;
    total: number;
    hasNext: boolean;
  };
  metadata?: Record<string, any>;
}

// WebSocket Types
export interface WebSocketMessage {
  type: string;
  payload: any;
  timestamp: Date;
  userId?: string;
  transactionId?: string;
}

export interface TransactionUpdate {
  transactionId: string;
  status: TransactionStatus;
  progress: number;
  message: string;
  estimatedCompletion?: Date;
  blockchainTxHash?: string;
}

// Configuration Types
export interface StripeConfig {
  publishableKey: string;
  secretKey: string;
  webhookSecret: string;
  connectClientId: string;
  platformAccountId: string;
  environment: 'test' | 'live';
}

export interface LayerZeroConfig {
  endpointV2Address: string;
  gateTokenAddress: string;
  usdcAddress: string;
  supportedChains: ChainConfig[];
}

export interface ChainConfig {
  chainId: number;
  name: string;
  rpcUrl: string;
  explorerUrl: string;
  layerZeroChainId: number;
  gasMultiplier: number;
  confirmations: number;
}

// Error Types
export class GatewayError extends Error {
  constructor(
    public code: string,
    message: string,
    public statusCode: number = 500,
    public details?: any
  ) {
    super(message);
    this.name = 'GatewayError';
  }
}

export enum ErrorCodes {
  INVALID_REQUEST = 'INVALID_REQUEST',
  UNAUTHORIZED = 'UNAUTHORIZED',
  FORBIDDEN = 'FORBIDDEN',
  NOT_FOUND = 'NOT_FOUND',
  RATE_LIMIT_EXCEEDED = 'RATE_LIMIT_EXCEEDED',
  KYC_REQUIRED = 'KYC_REQUIRED',
  INSUFFICIENT_FUNDS = 'INSUFFICIENT_FUNDS',
  TRANSACTION_FAILED = 'TRANSACTION_FAILED',
  COMPLIANCE_VIOLATION = 'COMPLIANCE_VIOLATION',
  SYSTEM_ERROR = 'SYSTEM_ERROR'
}

// Webhook Types
export interface StripeWebhookEvent {
  id: string;
  type: string;
  data: {
    object: any;
  };
  created: number;
  livemode: boolean;
  api_version: string;
}

export interface WebhookProcessor {
  processEvent(event: StripeWebhookEvent): Promise<void>;
}

// Queue Job Types
export interface QueueJob {
  id: string;
  type: JobType;
  data: any;
  priority: number;
  attempts: number;
  delay?: number;
  createdAt: Date;
}

export enum JobType {
  PROCESS_DEPOSIT = 'PROCESS_DEPOSIT',
  PROCESS_WITHDRAWAL = 'PROCESS_WITHDRAWAL',
  SYNC_RESERVES = 'SYNC_RESERVES',
  KYC_VERIFICATION = 'KYC_VERIFICATION',
  COMPLIANCE_CHECK = 'COMPLIANCE_CHECK',
  SEND_NOTIFICATION = 'SEND_NOTIFICATION'
}