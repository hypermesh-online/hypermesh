// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Application Configuration
 *
 * Centralizes environment-based configuration for the HyperMesh dashboard.
 * Supports both development and production environments with automatic detection.
 */

interface AppConfig {
  stoq: {
    serverAddress: string;
    serverPort: number;
    serverName: string;
  };
  api: {
    baseUrl: string;
  };
  environment: 'development' | 'production' | 'test';
  features: {
    enableDebugLogging: boolean;
    enablePerformanceMonitoring: boolean;
    enableErrorReporting: boolean;
  };
}

/**
 * Get the current application configuration based on environment
 */
export function getConfig(): AppConfig {
  const env = import.meta.env;
  const isDevelopment = env.MODE === 'development';
  const isProduction = env.MODE === 'production';

  // Auto-detect environment or use explicit setting
  const environment = env.VITE_ENVIRONMENT ||
    (isProduction ? 'production' :
     isDevelopment ? 'development' : 'test') as AppConfig['environment'];

  return {
    stoq: {
      // Use environment variables with fallbacks
      serverAddress: env.VITE_STOQ_SERVER_ADDRESS ||
        (isProduction ? 'hypermesh.online' : '::1'),

      serverPort: parseInt(env.VITE_STOQ_SERVER_PORT || '9292'),

      serverName: env.VITE_STOQ_SERVER_NAME || 'hypermesh.online',
    },

    api: {
      baseUrl: env.VITE_API_BASE_URL ||
        (isProduction ? 'https://hypermesh.online' : 'http://localhost:8443'),
    },

    environment,

    features: {
      enableDebugLogging: !isProduction,
      enablePerformanceMonitoring: isProduction,
      enableErrorReporting: isProduction,
    },
  };
}

/**
 * Check if running in production
 */
export function isProduction(): boolean {
  return getConfig().environment === 'production';
}

/**
 * Check if running in development
 */
export function isDevelopment(): boolean {
  return getConfig().environment === 'development';
}

/**
 * Get STOQ connection URL
 */
export function getStoqUrl(): string {
  const config = getConfig();
  return `stoq://${config.stoq.serverAddress}:${config.stoq.serverPort}`;
}

/**
 * Get API base URL
 */
export function getApiUrl(path: string = ''): string {
  const config = getConfig();
  const baseUrl = config.api.baseUrl.replace(/\/$/, ''); // Remove trailing slash
  const cleanPath = path.startsWith('/') ? path : `/${path}`;
  return `${baseUrl}${cleanPath}`;
}

/**
 * Log configuration (only in development)
 */
export function logConfig(): void {
  const config = getConfig();
  if (config.features.enableDebugLogging) {
    console.group('Application Configuration');
    console.log('Environment:', config.environment);
    console.log('STOQ Server:', `${config.stoq.serverAddress}:${config.stoq.serverPort}`);
    console.log('API Base URL:', config.api.baseUrl);
    console.log('Features:', config.features);
    console.groupEnd();
  }
}

// Export singleton config instance
export const config = getConfig();

// Auto-log configuration in development
if (isDevelopment()) {
  logConfig();
}

export default getConfig;