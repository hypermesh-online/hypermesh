// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Security Monitoring Dashboard - Real-time security health monitoring
 *
 * Comprehensive security dashboard integrating:
 * - TrustChain certificate authority monitoring
 * - Byzantine fault detection and threat analysis
 * - Network security metrics and alerts
 * - Certificate transparency and audit logs
 * - Post-quantum cryptography status (FALCON-1024, Kyber)
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import {
  useCertificates,
  useTrustHierarchy,
  useByzantineDetections,
  useSystemStatus,
  useValidateCertificate
} from '@/lib/api';
import { Shield, AlertTriangle, Network, Eye } from 'lucide-react';
import {
  AlertsTab,
  CertificatesTab,
  ThreatsTab,
  AuditTab
} from './security-monitoring-dashboard';
import type { SecurityAlert, SecurityMetrics } from './security-monitoring-dashboard';

export function SecurityMonitoringDashboard() {
  const { systemStatus } = useSystemStatus(true);
  const { certificates, isLoading: certsLoading } = useCertificates();
  const { data: trustHierarchy } = useTrustHierarchy();
  const { data: byzantineDetections } = useByzantineDetections();
  const [validatingCertId, setValidatingCertId] = React.useState<string | null>(null);
  const validateCertificate = useValidateCertificate(validatingCertId || '');

  const securityMetrics = React.useMemo((): SecurityMetrics => {
    if (!certificates || !systemStatus) {
      return { certificateHealth: 0, networkSecurity: 0, threatLevel: 'medium', activeThreats: 0, lastAudit: 'Never', auditScore: 0 };
    }

    const activeCerts = certificates.filter(c => c.status === 'active');
    const expiringSoon = certificates.filter(c => {
      const daysUntilExpiry = Math.ceil((new Date(c.validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
      return daysUntilExpiry <= 30 && daysUntilExpiry > 0;
    });
    const revokedCerts = certificates.filter(c => c.status === 'revoked');

    const certificateHealth = certificates.length > 0 ? (activeCerts.length / certificates.length) * 100 : 0;
    const healthyServices = Object.values(systemStatus.services).filter(s => s.status === 'healthy').length;
    const totalServices = Object.values(systemStatus.services).length;
    const networkSecurity = totalServices > 0 ? (healthyServices / totalServices) * 100 : 0;
    const activeThreats = (byzantineDetections?.length || 0) + expiringSoon.length + revokedCerts.length;

    let threatLevel: SecurityMetrics['threatLevel'] = 'low';
    if (activeThreats >= 10) threatLevel = 'critical';
    else if (activeThreats >= 5) threatLevel = 'high';
    else if (activeThreats >= 2) threatLevel = 'medium';

    return {
      certificateHealth, networkSecurity, threatLevel, activeThreats,
      lastAudit: new Date().toLocaleString(),
      auditScore: Math.min(certificateHealth, networkSecurity)
    };
  }, [certificates, systemStatus, byzantineDetections]);

  const securityAlerts = React.useMemo((): SecurityAlert[] => {
    const alerts: SecurityAlert[] = [];

    if (certificates) {
      certificates.forEach(cert => {
        const daysUntilExpiry = Math.ceil((new Date(cert.validTo).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
        if (daysUntilExpiry <= 7 && daysUntilExpiry > 0) {
          alerts.push({
            id: `cert-expiry-${cert.id}`, type: 'certificate',
            severity: daysUntilExpiry <= 1 ? 'critical' : 'high',
            title: 'Certificate Expiring Soon',
            description: `Certificate ${cert.subject} expires in ${daysUntilExpiry} days`,
            timestamp: new Date().toISOString(), resolved: false
          });
        }
      });
    }

    if (byzantineDetections) {
      byzantineDetections.forEach(detection => {
        alerts.push({
          id: `byzantine-${detection.nodeId}`, type: 'byzantine', severity: detection.severity,
          title: 'Byzantine Behavior Detected',
          description: `Malicious behavior detected from node ${detection.nodeId?.slice(0, 8)}...`,
          timestamp: detection.detectedAt, resolved: detection.status === 'resolved'
        });
      });
    }

    if (systemStatus) {
      Object.entries(systemStatus.services).forEach(([serviceKey, service]) => {
        if (service.status !== 'healthy') {
          alerts.push({
            id: `service-${serviceKey}`, type: 'network',
            severity: service.status === 'critical' ? 'critical' : 'medium',
            title: 'Service Health Issue',
            description: `Service ${service.name} is ${service.status}`,
            timestamp: new Date().toISOString(), resolved: false
          });
        }
      });
    }

    return alerts.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
  }, [certificates, byzantineDetections, systemStatus]);

  const handleCertificateValidation = () => {
    if (!certificates || certificates.length === 0) { alert('No certificates available for validation'); return; }
    setValidatingCertId(certificates[0].id);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-red-400 to-orange-600 bg-clip-text text-transparent mb-2">
          Security Monitoring Center
        </h1>
        <p className="text-gray-400 max-w-3xl mx-auto">
          Real-time security monitoring with TrustChain certificate authority, Byzantine fault detection,
          and post-quantum cryptography status. Monitor threats and maintain system security integrity.
        </p>
      </div>

      {/* Security Overview Metrics */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Certificate Health</CardTitle>
            <Shield className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{securityMetrics.certificateHealth.toFixed(1)}%</div>
            <p className="text-xs text-gray-400">Active certificates</p>
            <Progress value={securityMetrics.certificateHealth} className="mt-2 h-1" />
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Network Security</CardTitle>
            <Network className="h-4 w-4 text-blue-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-400">{securityMetrics.networkSecurity.toFixed(1)}%</div>
            <p className="text-xs text-gray-400">Healthy services</p>
            <Progress value={securityMetrics.networkSecurity} className="mt-2 h-1" />
          </CardContent>
        </Card>

        <Card className={cn(
          "bg-black/40 backdrop-blur-lg",
          securityMetrics.threatLevel === 'critical' ? 'border-red-500/30' :
          securityMetrics.threatLevel === 'high' ? 'border-orange-500/30' :
          securityMetrics.threatLevel === 'medium' ? 'border-yellow-500/30' :
          'border-green-500/30'
        )}>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Threat Level</CardTitle>
            <AlertTriangle className={cn(
              'h-4 w-4',
              securityMetrics.threatLevel === 'critical' ? 'text-red-400' :
              securityMetrics.threatLevel === 'high' ? 'text-orange-400' :
              securityMetrics.threatLevel === 'medium' ? 'text-yellow-400' :
              'text-green-400'
            )} />
          </CardHeader>
          <CardContent>
            <div className={cn(
              'text-2xl font-bold capitalize',
              securityMetrics.threatLevel === 'critical' ? 'text-red-400' :
              securityMetrics.threatLevel === 'high' ? 'text-orange-400' :
              securityMetrics.threatLevel === 'medium' ? 'text-yellow-400' :
              'text-green-400'
            )}>
              {securityMetrics.threatLevel}
            </div>
            <p className="text-xs text-gray-400">{securityMetrics.activeThreats} active threats</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Audit Score</CardTitle>
            <Eye className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{securityMetrics.auditScore.toFixed(0)}%</div>
            <p className="text-xs text-gray-400">Security compliance</p>
            <Progress value={securityMetrics.auditScore} className="mt-2 h-1" />
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="alerts" className="space-y-6">
        <TabsList className="grid w-full grid-cols-4 bg-black/40">
          <TabsTrigger value="alerts" className="data-[state=active]:bg-red-500/20">Security Alerts</TabsTrigger>
          <TabsTrigger value="certificates" className="data-[state=active]:bg-red-500/20">Certificates</TabsTrigger>
          <TabsTrigger value="threats" className="data-[state=active]:bg-red-500/20">Threat Detection</TabsTrigger>
          <TabsTrigger value="audit" className="data-[state=active]:bg-red-500/20">Audit Logs</TabsTrigger>
        </TabsList>

        <TabsContent value="alerts" className="space-y-6">
          <AlertsTab securityAlerts={securityAlerts} />
        </TabsContent>

        <TabsContent value="certificates" className="space-y-6">
          <CertificatesTab
            certificates={certificates}
            certsLoading={certsLoading}
            systemStatus={systemStatus}
            onValidate={handleCertificateValidation}
            isValidating={validateCertificate.isLoading}
          />
        </TabsContent>

        <TabsContent value="threats" className="space-y-6">
          <ThreatsTab byzantineDetections={byzantineDetections} />
        </TabsContent>

        <TabsContent value="audit" className="space-y-6">
          <AuditTab securityMetrics={securityMetrics} certificateCount={certificates?.length || 0} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
