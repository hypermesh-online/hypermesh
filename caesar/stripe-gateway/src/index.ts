// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import express, { Application, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import rateLimit from 'express-rate-limit';
import { body, param, query } from 'express-validator';
import multer from 'multer';
import Stripe from 'stripe';

// Import configuration and services
import { config, validateConfig } from '@/config';
import { GatewayController } from '@/controllers/GatewayController';
import { StripeService } from '@/services/StripeService';
import { 
  authenticateToken, 
  requireKYCLevel, 
  requireKYCStatus, 
  checkRiskLevel,
  checkTransactionLimits,
  authenticateWebhook,
  optionalAuth
} from '@/middleware/auth';
import { Logger, loggingMiddleware, errorLoggingMiddleware } from '@/utils/Logger';
import { 
  GatewayError, 
  ErrorCodes, 
  APIResponse, 
  StripeWebhookEvent,
  DocumentType,
  KYCStatus
} from '@/types';

// Initialize logger
const logger = new Logger('app');

// Initialize services
const stripeService = new StripeService();
const gatewayController = new GatewayController();

// Create Express application
const app: Application = express();

/**
 * Security and middleware setup
 */
app.use(helmet({
  contentSecurityPolicy: {
    directives: {
      defaultSrc: ["'self'"],
      styleSrc: ["'self'", "'unsafe-inline'", "https://js.stripe.com"],
      scriptSrc: ["'self'", "https://js.stripe.com"],
      imgSrc: ["'self'", "data:", "https:"],
      connectSrc: ["'self'", "https://api.stripe.com"],
      frameSrc: ["'self'", "https://js.stripe.com", "https://hooks.stripe.com"],
    },
  },
  crossOriginEmbedderPolicy: false,
}));

app.use(cors({
  origin: config.security.corsOrigin,
  credentials: true,
  optionsSuccessStatus: 200,
}));

app.use(compression());

// Rate limiting
const limiter = rateLimit({
  windowMs: config.security.rateLimiting.windowMs,
  max: config.security.rateLimiting.maxRequests,
  message: {
    success: false,
    error: {
      code: ErrorCodes.RATE_LIMIT_EXCEEDED,
      message: 'Too many requests, please try again later',
    },
  },
  standardHeaders: true,
  legacyHeaders: false,
});

app.use('/api/', limiter);

// Body parsing middleware
app.use('/webhooks/stripe', express.raw({ type: 'application/json' })); // Stripe webhooks need raw body
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true, limit: '10mb' }));

// Logging middleware
app.use(loggingMiddleware);

// File upload middleware for KYC documents
const upload = multer({
  storage: multer.memoryStorage(),
  limits: {
    fileSize: 10 * 1024 * 1024, // 10MB limit
  },
  fileFilter: (req, file, cb) => {
    const allowedTypes = ['image/jpeg', 'image/png', 'application/pdf'];
    if (allowedTypes.includes(file.mimetype)) {
      cb(null, true);
    } else {
      cb(new Error('Invalid file type. Only JPEG, PNG, and PDF files are allowed.'));
    }
  },
});

/**
 * Validation middleware
 */
const validateDeposit = [
  body('amount')
    .isFloat({ min: 1, max: 50000 })
    .withMessage('Amount must be between $1 and $50,000'),
  body('currency')
    .isIn(['USD'])
    .withMessage('Only USD is supported'),
  body('paymentMethodId')
    .isString()
    .notEmpty()
    .withMessage('Payment method ID is required'),
  body('destinationChain')
    .isInt({ min: 1 })
    .withMessage('Valid destination chain ID is required'),
  body('destinationAddress')
    .matches(/^0x[a-fA-F0-9]{40}$/)
    .withMessage('Valid Ethereum address is required'),
];

const validateWithdrawal = [
  body('gateAmount')
    .isString()
    .matches(/^\d+$/)
    .withMessage('Valid CAESAR token amount is required (in wei)'),
  body('currency')
    .isIn(['USD'])
    .withMessage('Only USD is supported'),
  body('sourceChain')
    .isInt({ min: 1 })
    .withMessage('Valid source chain ID is required'),
  body('bankAccount.accountNumber')
    .isString()
    .isLength({ min: 8, max: 17 })
    .withMessage('Valid bank account number is required'),
  body('bankAccount.routingNumber')
    .isString()
    .isLength({ min: 9, max: 9 })
    .withMessage('Valid 9-digit routing number is required'),
  body('bankAccount.accountHolderName')
    .isString()
    .isLength({ min: 2, max: 100 })
    .withMessage('Account holder name is required'),
];

const validateBridge = [
  body('fromChain')
    .isInt({ min: 1 })
    .withMessage('Valid source chain ID is required'),
  body('toChain')
    .isInt({ min: 1 })
    .withMessage('Valid destination chain ID is required'),
  body('amount')
    .isString()
    .matches(/^\d+$/)
    .withMessage('Valid token amount is required (in wei)'),
  body('recipient')
    .matches(/^0x[a-fA-F0-9]{40}$/)
    .withMessage('Valid recipient address is required'),
];

const validatePagination = [
  query('page')
    .optional()
    .isInt({ min: 1 })
    .withMessage('Page must be a positive integer'),
  query('limit')
    .optional()
    .isInt({ min: 1, max: 100 })
    .withMessage('Limit must be between 1 and 100'),
];

/**
 * Public Routes
 */
app.get('/health', gatewayController.healthCheck);

app.get('/api/v1/health', gatewayController.healthCheck);

// Get gas estimates (public endpoint with optional auth for personalization)
app.get('/api/v1/gas-estimates', optionalAuth, gatewayController.getGasEstimates);

// Get reserve status (public endpoint)
app.get('/api/v1/reserves', gatewayController.getReserveStatus);

/**
 * Authentication Required Routes
 */

// Fiat-to-crypto deposits
app.post('/api/v1/deposits',
  authenticateToken,
  requireKYCStatus([KYCStatus.APPROVED]),
  requireKYCLevel(1),
  checkRiskLevel(75),
  validateDeposit,
  checkTransactionLimits(),
  gatewayController.processDeposit
);

// Crypto-to-fiat withdrawals
app.post('/api/v1/withdrawals',
  authenticateToken,
  requireKYCStatus([KYCStatus.APPROVED]),
  requireKYCLevel(2), // Higher requirement for withdrawals
  checkRiskLevel(50),
  validateWithdrawal,
  checkTransactionLimits(),
  gatewayController.processWithdrawal
);

// Cross-chain bridging
app.post('/api/v1/bridge',
  authenticateToken,
  requireKYCStatus([KYCStatus.APPROVED]),
  requireKYCLevel(1),
  checkRiskLevel(75),
  validateBridge,
  gatewayController.bridgeTokens
);

// Transaction management
app.get('/api/v1/transactions/:transactionId',
  authenticateToken,
  param('transactionId').isString().notEmpty(),
  gatewayController.getTransactionStatus
);

app.get('/api/v1/transactions',
  authenticateToken,
  validatePagination,
  gatewayController.getTransactionHistory
);

/**
 * Stripe Connect Routes
 */
app.post('/api/v1/connect/account',
  authenticateToken,
  requireKYCLevel(2),
  gatewayController.createConnectAccount
);

app.post('/api/v1/connect/onboarding',
  authenticateToken,
  body('returnUrl').isURL().withMessage('Valid return URL is required'),
  body('refreshUrl').isURL().withMessage('Valid refresh URL is required'),
  gatewayController.createOnboardingLink
);

app.get('/api/v1/connect/status',
  authenticateToken,
  gatewayController.getConnectAccountStatus
);

/**
 * KYC Routes
 */
app.post('/api/v1/kyc/documents',
  authenticateToken,
  upload.single('document'),
  body('documentType').isIn(Object.values(DocumentType)),
  gatewayController.uploadKYCDocument
);

app.get('/api/v1/kyc/status',
  authenticateToken,
  gatewayController.getKYCStatus
);

/**
 * Webhook Routes
 */
app.post('/webhooks/stripe',
  authenticateWebhook,
  handleStripeWebhook
);

/**
 * Webhook handler function
 */
async function handleStripeWebhook(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const signature = req.headers['stripe-signature'] as string;
    const payload = req.body;

    // Verify webhook signature
    const stripe = new Stripe(config.stripe.secretKey);
    const event = stripe.webhooks.constructEvent(
      payload,
      signature,
      config.stripe.webhookSecret
    );

    logger.info('Received Stripe webhook', { 
      eventId: event.id, 
      eventType: event.type 
    });

    // Process the webhook event
    await stripeService.processWebhookEvent(event);

    res.status(200).json({ received: true });

  } catch (error) {
    logger.error('Webhook processing failed', { error });
    
    if (error.type === 'StripeSignatureVerificationError') {
      res.status(400).json({
        success: false,
        error: {
          code: ErrorCodes.UNAUTHORIZED,
          message: 'Invalid webhook signature',
        },
      });
    } else {
      next(error);
    }
  }
}

/**
 * Error handling middleware
 */
app.use(errorLoggingMiddleware);

app.use((err: any, req: Request, res: Response, next: NextFunction) => {
  // Handle multer errors
  if (err instanceof multer.MulterError) {
    if (err.code === 'LIMIT_FILE_SIZE') {
      return res.status(400).json({
        success: false,
        error: {
          code: ErrorCodes.INVALID_REQUEST,
          message: 'File too large (max 10MB)',
        },
      });
    }
  }

  // Handle validation errors
  if (err.name === 'ValidationError') {
    return res.status(400).json({
      success: false,
      error: {
        code: ErrorCodes.INVALID_REQUEST,
        message: err.message,
        details: err.details,
      },
    });
  }

  // Handle custom Gateway errors
  if (err instanceof GatewayError) {
    return res.status(err.statusCode).json({
      success: false,
      error: {
        code: err.code,
        message: err.message,
        details: err.details,
      },
    });
  }

  // Handle Stripe errors
  if (err.type && err.type.includes('Stripe')) {
    return res.status(400).json({
      success: false,
      error: {
        code: ErrorCodes.TRANSACTION_FAILED,
        message: 'Payment processing error',
        details: err.code,
      },
    });
  }

  // Handle unexpected errors
  logger.error('Unhandled error', { error: err });
  
  res.status(500).json({
    success: false,
    error: {
      code: ErrorCodes.SYSTEM_ERROR,
      message: config.server.environment === 'production' 
        ? 'An unexpected error occurred' 
        : err.message,
    },
  });
});

/**
 * 404 handler
 */
app.use('*', (req: Request, res: Response) => {
  res.status(404).json({
    success: false,
    error: {
      code: ErrorCodes.NOT_FOUND,
      message: 'Endpoint not found',
    },
  });
});

/**
 * Server startup
 */
async function startServer(): Promise<void> {
  try {
    // Validate configuration
    validateConfig();

    // Start the server
    const server = app.listen(config.server.port, () => {
      logger.info('🚀 Caesar Token Stripe Service started', {
        port: config.server.port,
        environment: config.server.environment,
        version: '1.0.0',
      });
    });

    // Graceful shutdown handling
    const gracefulShutdown = (signal: string) => {
      logger.info(`Received ${signal}. Starting graceful shutdown...`);
      
      server.close(async (err) => {
        if (err) {
          logger.error('Error during server shutdown', { error: err });
          process.exit(1);
        }

        logger.info('Server closed successfully');
        
        // Close database connections, Redis connections, etc.
        // await database.close();
        // await redis.disconnect();
        
        process.exit(0);
      });
    };

    process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));
    process.on('SIGINT', () => gracefulShutdown('SIGINT'));

    // Handle unhandled promise rejections
    process.on('unhandledRejection', (reason, promise) => {
      logger.error('Unhandled Promise Rejection', { reason, promise });
    });

    // Handle uncaught exceptions
    process.on('uncaughtException', (error) => {
      logger.error('Uncaught Exception', { error });
      process.exit(1);
    });

  } catch (error) {
    logger.error('Failed to start server', { error });
    process.exit(1);
  }
}

// Start the server
if (require.main === module) {
  startServer();
}

export default app;