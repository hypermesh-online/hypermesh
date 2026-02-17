// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Shield, AlertTriangle, CheckCircle, FileText } from 'lucide-react';
import { cn } from '@/lib/utils';

interface SecurityAuditResult {
  overallScore: number;
  vulnerabilities: Array<{
    severity: 'high' | 'medium' | 'low';
    category: string;
    description: string;
    recommendation: string;
  }>;
  compliance: {
    quantumResistant: boolean;
    pciCompliant: boolean;
    fipsApproved: boolean;
  };
  recommendations: string[];
}

interface TestCertResult {
  success: boolean;
  certificateDetails: {
    algorithm: string;
    keySize: number;
    validFrom: Date;
    validTo: Date;
    fingerprint: string;
  };
  verificationTests: {
    signatureValid: boolean;
    chainValid: boolean;
    quantumSafe: boolean;
    ocspValid: boolean;
  };
}

interface SecurityAuditResultsProps {
  auditResults?: SecurityAuditResult;
  testCertResults?: TestCertResult;
}

export function SecurityAuditResults({ 
  auditResults, 
  testCertResults 
}: SecurityAuditResultsProps) {
  if (!auditResults && !testCertResults) {
    return null;
  }

  return (
    <div className="space-y-6">
      {/* Security Audit Results */}
      {auditResults && (
        <Card className="mt-6">
          <CardHeader>
            <CardTitle className="flex items-center space-x-2">
              <Shield className="h-5 w-5" />
              <span>Security Audit Results</span>
              <Badge 
                variant={auditResults.overallScore >= 90 ? "default" : 
                        auditResults.overallScore >= 70 ? "secondary" : "destructive"}
              >
                {auditResults.overallScore}% Score
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {auditResults.vulnerabilities.length > 0 && (
                <div>
                  <h4 className="font-medium mb-2">Vulnerabilities Found</h4>
                  <div className="space-y-2">
                    {auditResults.vulnerabilities.map((vuln, index) => (
                      <div key={index} className="flex items-start space-x-2 p-3 border rounded-lg">
                        <AlertTriangle className={cn(
                          "h-4 w-4 mt-0.5",
                          vuln.severity === 'high' ? 'text-red-600' :
                          vuln.severity === 'medium' ? 'text-yellow-600' : 'text-blue-600'
                        )} />
                        <div className="flex-1">
                          <div className="flex items-center space-x-2">
                            <span className="font-medium">{vuln.category}</span>
                            <Badge variant={
                              vuln.severity === 'high' ? 'destructive' :
                              vuln.severity === 'medium' ? 'secondary' : 'default'
                            }>
                              {vuln.severity}
                            </Badge>
                          </div>
                          <p className="text-sm text-muted-foreground mt-1">{vuln.description}</p>
                          <p className="text-sm text-blue-600 mt-1">{vuln.recommendation}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <div>
                <h4 className="font-medium mb-2">Compliance Status</h4>
                <div className="grid grid-cols-3 gap-4 text-sm">
                  <div className="flex items-center justify-between">
                    <span>Quantum Resistant</span>
                    {auditResults.compliance.quantumResistant ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex items-center justify-between">
                    <span>PCI Compliant</span>
                    {auditResults.compliance.pciCompliant ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex items-center justify-between">
                    <span>FIPS Approved</span>
                    {auditResults.compliance.fipsApproved ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Test Certificate Results */}
      {testCertResults && (
        <Card className="mt-6">
          <CardHeader>
            <CardTitle className="flex items-center space-x-2">
              <FileText className="h-5 w-5" />
              <span>Test Certificate Results</span>
              {testCertResults.success ? (
                <CheckCircle className="h-5 w-5 text-green-600" />
              ) : (
                <AlertTriangle className="h-5 w-5 text-red-600" />
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <h4 className="font-medium">Certificate Details</h4>
                <div className="text-sm space-y-1">
                  <div className="flex justify-between">
                    <span>Algorithm:</span>
                    <span className="font-medium">{testCertResults.certificateDetails.algorithm}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Key Size:</span>
                    <span className="font-medium">{testCertResults.certificateDetails.keySize} bits</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Valid From:</span>
                    <span className="font-medium">
                      {testCertResults.certificateDetails.validFrom.toLocaleDateString()}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span>Valid To:</span>
                    <span className="font-medium">
                      {testCertResults.certificateDetails.validTo.toLocaleDateString()}
                    </span>
                  </div>
                </div>
              </div>
              <div className="space-y-2">
                <h4 className="font-medium">Verification Tests</h4>
                <div className="text-sm space-y-1">
                  <div className="flex justify-between">
                    <span>Signature Valid:</span>
                    {testCertResults.verificationTests.signatureValid ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex justify-between">
                    <span>Chain Valid:</span>
                    {testCertResults.verificationTests.chainValid ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex justify-between">
                    <span>Quantum-Safe:</span>
                    {testCertResults.verificationTests.quantumSafe ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="flex justify-between">
                    <span>OCSP Valid:</span>
                    {testCertResults.verificationTests.ocspValid ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}