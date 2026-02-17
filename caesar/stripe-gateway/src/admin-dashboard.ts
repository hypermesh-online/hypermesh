// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import express, { Request, Response, NextFunction } from 'express';
import { StripeService } from '@/services/StripeService';
import { LayerZeroService } from '@/services/LayerZeroService';
import { KYCService } from '@/services/KYCService';
import { Logger } from '@/utils/Logger';
import {
  APIResponse,
  ComplianceReport,
  ReserveStatus,
  PegStatus,
  GatewayError,
  ErrorCodes,
  TransactionStatus,
  KYCStatus,
  RiskLevel,
} from '@/types';

export class AdminDashboardController {
  private logger: Logger;
  private stripeService: StripeService;
  private layerZeroService: LayerZeroService;
  private kycService: KYCService;

  constructor() {
    this.logger = new Logger('AdminDashboard');
    this.stripeService = new StripeService();
    this.layerZeroService = new LayerZeroService();
    this.kycService = new KYCService();
  }

  /**
   * Get comprehensive system dashboard data
   * GET /admin/v1/dashboard
   */
  public getDashboard = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      this.logger.info('Loading admin dashboard');

      const [
        systemHealth,
        transactionMetrics,
        reserveStatus,
        complianceMetrics,
        userMetrics,
        recentAlerts
      ] = await Promise.all([
        this.getSystemHealth(),
        this.getTransactionMetrics(),
        this.layerZeroService.syncReserves(),
        this.getComplianceMetrics(),
        this.getUserMetrics(),
        this.getRecentAlerts(),
      ]);

      const response: APIResponse = {
        success: true,
        data: {
          systemHealth,
          transactions: transactionMetrics,
          reserves: reserveStatus,
          compliance: complianceMetrics,
          users: userMetrics,
          alerts: recentAlerts,
          lastUpdated: new Date().toISOString(),
        },
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get detailed transaction analytics
   * GET /admin/v1/analytics/transactions
   */
  public getTransactionAnalytics = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { 
        startDate, 
        endDate, 
        groupBy = 'day',
        chainId 
      } = req.query;

      this.logger.info('Getting transaction analytics', { 
        startDate, 
        endDate, 
        groupBy,
        chainId 
      });

      const analytics = await this.generateTransactionAnalytics({
        startDate: startDate as string,
        endDate: endDate as string,
        groupBy: groupBy as 'hour' | 'day' | 'week' | 'month',
        chainId: chainId ? parseInt(chainId as string) : undefined,
      });

      const response: APIResponse = {
        success: true,
        data: analytics,
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get compliance report
   * GET /admin/v1/compliance/report
   */
  public getComplianceReport = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { startDate, endDate } = req.query;

      this.logger.info('Generating compliance report', { startDate, endDate });

      const report = await this.generateComplianceReport({
        startDate: new Date(startDate as string),
        endDate: new Date(endDate as string),
      });

      const response: APIResponse = {
        success: true,
        data: report,
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get users requiring manual review
   * GET /admin/v1/compliance/pending-reviews
   */
  public getPendingReviews = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { page = '1', limit = '20', riskLevel } = req.query;

      this.logger.info('Getting pending reviews', { page, limit, riskLevel });

      const pendingReviews = await this.getPendingUserReviews({
        page: parseInt(page as string),
        limit: parseInt(limit as string),
        riskLevel: riskLevel as RiskLevel,
      });

      const response: APIResponse = {
        success: true,
        data: pendingReviews.users,
        pagination: {
          page: parseInt(page as string),
          limit: parseInt(limit as string),
          total: pendingReviews.total,
          hasNext: pendingReviews.hasNext,
        },
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Approve or reject user KYC
   * POST /admin/v1/kyc/:userId/decision
   */
  public makeKYCDecision = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { userId } = req.params;
      const { decision, reason, kycLevel } = req.body;
      const adminId = 'admin-user-id'; // Would come from authenticated admin user

      this.logger.info('Making KYC decision', {
        userId,
        decision,
        reason,
        kycLevel,
        adminId
      });

      if (!['approve', 'reject'].includes(decision)) {
        throw new GatewayError(
          ErrorCodes.INVALID_REQUEST,
          'Decision must be either "approve" or "reject"',
          400
        );
      }

      // Update user KYC status
      await this.updateUserKYCStatus({
        userId,
        decision,
        reason,
        kycLevel,
        reviewedBy: adminId,
        reviewedAt: new Date(),
      });

      // Log audit event
      this.logger.audit('KYC_DECISION', adminId, {
        targetUserId: userId,
        decision,
        reason,
        kycLevel,
      });

      const response: APIResponse = {
        success: true,
        data: {
          userId,
          decision,
          processedAt: new Date().toISOString(),
        },
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get reserve management controls
   * GET /admin/v1/reserves/management
   */
  public getReserveManagement = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      this.logger.info('Getting reserve management data');

      const [reserves, pegStatus, chainBalances, recentTransfers] = await Promise.all([
        this.layerZeroService.syncReserves(),
        this.layerZeroService.validatePegMaintenance(),
        this.getChainBalances(),
        this.getRecentReserveTransfers(),
      ]);

      const response: APIResponse = {
        success: true,
        data: {
          overview: {
            totalReserves: reserves.totalUSDCReserves,
            totalSupply: reserves.totalGATESupply,
            pegRatio: reserves.pegRatio,
            healthScore: reserves.reserveHealthScore,
          },
          pegStatus,
          chainBreakdown: chainBalances,
          recentActivity: recentTransfers,
          discrepancies: reserves.discrepancies,
        },
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Emergency controls
   * POST /admin/v1/emergency/:action
   */
  public emergencyAction = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { action } = req.params;
      const { reason } = req.body;
      const adminId = 'admin-user-id';

      this.logger.security('EMERGENCY_ACTION', `Emergency action triggered: ${action}`, {
        action,
        reason,
        adminId,
      });

      let result;
      switch (action) {
        case 'pause-deposits':
          result = await this.pauseDeposits(reason);
          break;
        case 'pause-withdrawals':
          result = await this.pauseWithdrawals(reason);
          break;
        case 'pause-all':
          result = await this.pauseAllOperations(reason);
          break;
        case 'emergency-sync':
          result = await this.emergencyReserveSync();
          break;
        default:
          throw new GatewayError(
            ErrorCodes.INVALID_REQUEST,
            'Invalid emergency action',
            400
          );
      }

      // Log audit event
      this.logger.audit('EMERGENCY_ACTION', adminId, {
        action,
        reason,
        result,
      });

      const response: APIResponse = {
        success: true,
        data: {
          action,
          status: 'executed',
          timestamp: new Date().toISOString(),
          result,
        },
      };

      res.status(200).json(response);

    } catch (error) {
      next(error);
    }
  };

  /**
   * Get system health metrics
   */
  private async getSystemHealth(): Promise<{
    status: string;
    uptime: number;
    services: Record<string, string>;
    performance: Record<string, number>;
  }> {
    const uptime = process.uptime();
    
    // Check service health
    const services = {
      stripe: await this.checkStripeHealth(),
      layerzero: await this.checkLayerZeroHealth(),
      database: await this.checkDatabaseHealth(),
      redis: await this.checkRedisHealth(),
    };

    const allServicesHealthy = Object.values(services).every(status => status === 'healthy');

    return {
      status: allServicesHealthy ? 'healthy' : 'degraded',
      uptime,
      services,
      performance: {
        memoryUsage: process.memoryUsage().heapUsed / 1024 / 1024, // MB
        cpuUsage: process.cpuUsage().user / 1000000, // seconds
      },
    };
  }

  /**
   * Get transaction metrics
   */
  private async getTransactionMetrics(): Promise<{
    today: any;
    thisWeek: any;
    thisMonth: any;
    statusBreakdown: any;
  }> {
    // This would query actual database
    return {
      today: {
        count: 156,
        volume: 487500,
        averageAmount: 3125,
        fees: 2437,
      },
      thisWeek: {
        count: 892,
        volume: 2350000,
        averageAmount: 2634,
        fees: 11750,
      },
      thisMonth: {
        count: 3456,
        volume: 9800000,
        averageAmount: 2836,
        fees: 49000,
      },
      statusBreakdown: {
        [TransactionStatus.COMPLETED]: 3234,
        [TransactionStatus.PENDING]: 156,
        [TransactionStatus.PROCESSING]: 45,
        [TransactionStatus.FAILED]: 21,
      },
    };
  }

  /**
   * Get compliance metrics
   */
  private async getComplianceMetrics(): Promise<{
    kycStatus: Record<string, number>;
    riskDistribution: Record<string, number>;
    alertsThisMonth: number;
    flaggedTransactions: number;
  }> {
    return {
      kycStatus: {
        [KYCStatus.APPROVED]: 2845,
        [KYCStatus.PENDING]: 234,
        [KYCStatus.UNDER_REVIEW]: 45,
        [KYCStatus.REJECTED]: 78,
      },
      riskDistribution: {
        [RiskLevel.LOW]: 2156,
        [RiskLevel.MEDIUM]: 634,
        [RiskLevel.HIGH]: 156,
        [RiskLevel.PROHIBITED]: 12,
      },
      alertsThisMonth: 23,
      flaggedTransactions: 8,
    };
  }

  /**
   * Get user metrics
   */
  private async getUserMetrics(): Promise<{
    total: number;
    active: number;
    newThisMonth: number;
    averageRiskScore: number;
  }> {
    return {
      total: 3456,
      active: 2234,
      newThisMonth: 345,
      averageRiskScore: 23.5,
    };
  }

  /**
   * Get recent alerts
   */
  private async getRecentAlerts(): Promise<any[]> {
    return [
      {
        id: 'alert-1',
        type: 'HIGH_RISK_TRANSACTION',
        severity: 'HIGH',
        message: 'Large withdrawal detected: $25,000',
        userId: 'user-123',
        createdAt: new Date(Date.now() - 3600000).toISOString(),
        status: 'OPEN',
      },
      {
        id: 'alert-2',
        type: 'PEG_DEVIATION',
        severity: 'MEDIUM',
        message: 'Peg ratio deviation: 1.03',
        createdAt: new Date(Date.now() - 7200000).toISOString(),
        status: 'INVESTIGATING',
      },
    ];
  }

  /**
   * Generate transaction analytics
   */
  private async generateTransactionAnalytics(params: {
    startDate: string;
    endDate: string;
    groupBy: 'hour' | 'day' | 'week' | 'month';
    chainId?: number;
  }): Promise<any> {
    // This would query actual database and generate analytics
    return {
      summary: {
        totalTransactions: 1234,
        totalVolume: 5670000,
        averageAmount: 4596,
        totalFees: 28350,
      },
      timeSeries: [
        { period: '2024-01-01', count: 45, volume: 125000 },
        { period: '2024-01-02', count: 67, volume: 198000 },
        // ... more data points
      ],
      chainBreakdown: {
        1: { count: 567, volume: 2100000 }, // Ethereum
        137: { count: 345, volume: 1450000 }, // Polygon
        42161: { count: 234, volume: 980000 }, // Arbitrum
      },
    };
  }

  /**
   * Generate compliance report
   */
  private async generateComplianceReport(params: {
    startDate: Date;
    endDate: Date;
  }): Promise<ComplianceReport> {
    return {
      reportId: `report-${Date.now()}`,
      generatedAt: new Date(),
      period: {
        startDate: params.startDate,
        endDate: params.endDate,
      },
      metrics: {
        totalTransactions: 1234,
        totalVolume: 5670000,
        largeTransactions: 23,
        rapidTransactions: 5,
        crossBorderTransactions: 45,
        flaggedUsers: 8,
        averageRiskScore: 23.5,
      },
      alerts: [],
      suspiciousActivities: [],
    };
  }

  /**
   * Helper methods for service health checks
   */
  private async checkStripeHealth(): Promise<string> {
    try {
      // Would make actual Stripe API call
      return 'healthy';
    } catch {
      return 'unhealthy';
    }
  }

  private async checkLayerZeroHealth(): Promise<string> {
    try {
      // Would check blockchain connections
      return 'healthy';
    } catch {
      return 'unhealthy';
    }
  }

  private async checkDatabaseHealth(): Promise<string> {
    try {
      // Would check database connection
      return 'healthy';
    } catch {
      return 'unhealthy';
    }
  }

  private async checkRedisHealth(): Promise<string> {
    try {
      // Would check Redis connection
      return 'healthy';
    } catch {
      return 'unhealthy';
    }
  }

  /**
   * Other helper methods (simplified implementations)
   */
  private async getPendingUserReviews(params: any): Promise<any> {
    return { users: [], total: 0, hasNext: false };
  }

  private async updateUserKYCStatus(params: any): Promise<void> {
    // Implementation would update database
  }

  private async getChainBalances(): Promise<any> {
    return {};
  }

  private async getRecentReserveTransfers(): Promise<any[]> {
    return [];
  }

  private async pauseDeposits(reason: string): Promise<any> {
    return { paused: true, reason };
  }

  private async pauseWithdrawals(reason: string): Promise<any> {
    return { paused: true, reason };
  }

  private async pauseAllOperations(reason: string): Promise<any> {
    return { paused: true, reason };
  }

  private async emergencyReserveSync(): Promise<any> {
    return await this.layerZeroService.syncReserves();
  }
}

/**
 * Admin dashboard routes
 */
export function setupAdminRoutes(app: express.Application): void {
  const adminController = new AdminDashboardController();

  // Admin authentication middleware (simplified)
  const authenticateAdmin = (req: any, res: any, next: any) => {
    // Implementation would verify admin credentials
    next();
  };

  // Dashboard routes
  app.get('/admin/v1/dashboard', authenticateAdmin, adminController.getDashboard);
  app.get('/admin/v1/analytics/transactions', authenticateAdmin, adminController.getTransactionAnalytics);
  app.get('/admin/v1/compliance/report', authenticateAdmin, adminController.getComplianceReport);
  app.get('/admin/v1/compliance/pending-reviews', authenticateAdmin, adminController.getPendingReviews);
  app.post('/admin/v1/kyc/:userId/decision', authenticateAdmin, adminController.makeKYCDecision);
  app.get('/admin/v1/reserves/management', authenticateAdmin, adminController.getReserveManagement);
  app.post('/admin/v1/emergency/:action', authenticateAdmin, adminController.emergencyAction);
}