// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import winston from 'winston';
import { config } from '@/config';

// Custom log levels
const levels = {
  error: 0,
  warn: 1,
  info: 2,
  http: 3,
  debug: 4,
};

// Colors for different log levels
const colors = {
  error: 'red',
  warn: 'yellow',
  info: 'green',
  http: 'magenta',
  debug: 'white',
};

winston.addColors(colors);

// Define log format
const logFormat = winston.format.combine(
  winston.format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss' }),
  winston.format.errors({ stack: true }),
  winston.format.json(),
  winston.format.printf((info) => {
    const { timestamp, level, message, service, ...meta } = info;
    
    let logMessage = `${timestamp} [${level.toUpperCase()}]`;
    if (service) logMessage += ` [${service}]`;
    logMessage += `: ${message}`;
    
    // Add metadata if present
    if (Object.keys(meta).length > 0) {
      logMessage += ` ${JSON.stringify(meta)}`;
    }
    
    return logMessage;
  })
);

// Console format for development
const consoleFormat = winston.format.combine(
  winston.format.colorize({ all: true }),
  winston.format.timestamp({ format: 'HH:mm:ss' }),
  winston.format.printf((info) => {
    const { timestamp, level, message, service, ...meta } = info;
    
    let logMessage = `${timestamp} ${level}`;
    if (service) logMessage += ` [${service}]`;
    logMessage += `: ${message}`;
    
    // Add metadata for debugging
    if (Object.keys(meta).length > 0 && config.server.environment === 'development') {
      logMessage += `\n${JSON.stringify(meta, null, 2)}`;
    }
    
    return logMessage;
  })
);

// Create winston logger
const winstonLogger = winston.createLogger({
  level: config.monitoring.logLevel,
  levels,
  format: logFormat,
  defaultMeta: {
    service: 'caesar-token-stripe',
    environment: config.server.environment,
  },
  transports: [
    // Console transport
    new winston.transports.Console({
      format: config.server.environment === 'production' ? logFormat : consoleFormat,
    }),
    
    // File transports for production
    ...(config.server.environment === 'production' ? [
      new winston.transports.File({
        filename: 'logs/error.log',
        level: 'error',
        maxsize: 5242880, // 5MB
        maxFiles: 10,
      }),
      new winston.transports.File({
        filename: 'logs/combined.log',
        maxsize: 5242880, // 5MB
        maxFiles: 10,
      }),
    ] : []),
  ],
  
  // Don't exit on handled exceptions
  exitOnError: false,
});

/**
 * Enhanced Logger class with service-specific context
 */
export class Logger {
  private service: string;

  constructor(service: string) {
    this.service = service;
  }

  private formatMessage(message: string, meta?: Record<string, any>): [string, Record<string, any>] {
    const formattedMeta = {
      ...meta,
      service: this.service,
      timestamp: new Date().toISOString(),
    };

    return [message, formattedMeta];
  }

  debug(message: string, meta?: Record<string, any>): void {
    const [msg, formattedMeta] = this.formatMessage(message, meta);
    winstonLogger.debug(msg, formattedMeta);
  }

  info(message: string, meta?: Record<string, any>): void {
    const [msg, formattedMeta] = this.formatMessage(message, meta);
    winstonLogger.info(msg, formattedMeta);
  }

  warn(message: string, meta?: Record<string, any>): void {
    const [msg, formattedMeta] = this.formatMessage(message, meta);
    winstonLogger.warn(msg, formattedMeta);
  }

  error(message: string, meta?: Record<string, any>): void {
    const [msg, formattedMeta] = this.formatMessage(message, meta);
    winstonLogger.error(msg, formattedMeta);
  }

  http(message: string, meta?: Record<string, any>): void {
    const [msg, formattedMeta] = this.formatMessage(message, meta);
    winstonLogger.http(msg, formattedMeta);
  }

  // Specialized logging methods for different contexts
  
  transaction(transactionId: string, message: string, meta?: Record<string, any>): void {
    this.info(message, {
      ...meta,
      transactionId,
      context: 'transaction',
    });
  }

  security(event: string, message: string, meta?: Record<string, any>): void {
    this.warn(message, {
      ...meta,
      securityEvent: event,
      context: 'security',
    });
  }

  compliance(event: string, message: string, meta?: Record<string, any>): void {
    this.info(message, {
      ...meta,
      complianceEvent: event,
      context: 'compliance',
    });
  }

  performance(operation: string, duration: number, meta?: Record<string, any>): void {
    this.info(`Performance: ${operation} completed in ${duration}ms`, {
      ...meta,
      operation,
      duration,
      context: 'performance',
    });
  }

  audit(action: string, userId: string, meta?: Record<string, any>): void {
    this.info(`Audit: ${action}`, {
      ...meta,
      action,
      userId,
      context: 'audit',
    });
  }

  // Method to log structured API requests/responses
  apiLog(method: string, path: string, statusCode: number, duration: number, meta?: Record<string, any>): void {
    const level = statusCode >= 400 ? 'warn' : 'info';
    const message = `${method} ${path} ${statusCode} - ${duration}ms`;

    this[level](message, {
      ...meta,
      httpMethod: method,
      httpPath: path,
      httpStatusCode: statusCode,
      duration,
      context: 'api',
    });
  }

  // Method to log errors with stack traces
  exception(error: Error, message?: string, meta?: Record<string, any>): void {
    this.error(message || error.message, {
      ...meta,
      error: {
        name: error.name,
        message: error.message,
        stack: error.stack,
      },
      context: 'exception',
    });
  }
}

// Export singleton logger for general use
export const logger = new Logger('app');

// Export logging middleware for Express
export const loggingMiddleware = (req: any, res: any, next: any) => {
  const start = Date.now();
  const reqLogger = new Logger('http');

  // Log incoming request
  reqLogger.http('Incoming request', {
    method: req.method,
    path: req.path,
    userAgent: req.get('User-Agent'),
    ip: req.ip,
    userId: req.user?.id,
  });

  // Override res.end to capture response
  const originalEnd = res.end;
  res.end = function(...args: any[]) {
    const duration = Date.now() - start;
    
    reqLogger.apiLog(
      req.method,
      req.path,
      res.statusCode,
      duration,
      {
        userId: req.user?.id,
        userAgent: req.get('User-Agent'),
        ip: req.ip,
      }
    );

    originalEnd.apply(this, args);
  };

  next();
};

// Export error logging middleware for Express
export const errorLoggingMiddleware = (err: any, req: any, res: any, next: any) => {
  const errorLogger = new Logger('error-handler');

  errorLogger.exception(err, 'Unhandled error in request', {
    method: req.method,
    path: req.path,
    userId: req.user?.id,
    userAgent: req.get('User-Agent'),
    ip: req.ip,
    requestId: req.headers['x-request-id'],
  });

  next(err);
};

export default Logger;