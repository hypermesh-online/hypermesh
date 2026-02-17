// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from 'hardhat';
import { writeFileSync, mkdirSync, existsSync } from 'fs';
import { join } from 'path';

export interface TestResult {
  testSuite: string;
  testName: string;
  status: 'passed' | 'failed' | 'skipped';
  duration: number;
  gasUsed?: bigint;
  error?: string;
  metrics?: Record<string, any>;
}

export interface PerformanceMetric {
  metric: string;
  value: number | bigint;
  unit: string;
  target?: number | bigint;
  status: 'pass' | 'fail' | 'warning';
  timestamp: number;
}

export interface SecurityFinding {
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info';
  category: string;
  description: string;
  contract?: string;
  function?: string;
  recommendation: string;
  status: 'open' | 'resolved' | 'acknowledged';
}

export interface CoverageReport {
  contract: string;
  statements: { hit: number; total: number; percentage: number };
  branches: { hit: number; total: number; percentage: number };
  functions: { hit: number; total: number; percentage: number };
  lines: { hit: number; total: number; percentage: number };
}

export class TestReporter {
  private results: TestResult[] = [];
  private performanceMetrics: PerformanceMetric[] = [];
  private securityFindings: SecurityFinding[] = [];
  private coverageReports: CoverageReport[] = [];
  private startTime: number = Date.now();

  constructor(private outputDir: string = './test-reports') {
    this.ensureOutputDirectory();
  }

  private ensureOutputDirectory(): void {
    if (!existsSync(this.outputDir)) {
      mkdirSync(this.outputDir, { recursive: true });
    }
  }

  // Test Result Management
  addTestResult(result: TestResult): void {
    this.results.push(result);
  }

  addPerformanceMetric(metric: PerformanceMetric): void {
    this.performanceMetrics.push(metric);
  }

  addSecurityFinding(finding: SecurityFinding): void {
    this.securityFindings.push(finding);
  }

  addCoverageReport(report: CoverageReport): void {
    this.coverageReports.push(report);
  }

  // Report Generation
  generateSummaryReport(): string {
    const totalTests = this.results.length;
    const passedTests = this.results.filter(r => r.status === 'passed').length;
    const failedTests = this.results.filter(r => r.status === 'failed').length;
    const skippedTests = this.results.filter(r => r.status === 'skipped').length;
    const totalDuration = this.results.reduce((sum, r) => sum + r.duration, 0);
    
    const passRate = totalTests > 0 ? ((passedTests / totalTests) * 100).toFixed(2) : '0.00';
    
    const criticalFindings = this.securityFindings.filter(f => f.severity === 'critical').length;
    const highFindings = this.securityFindings.filter(f => f.severity === 'high').length;
    
    const avgCoverage = this.coverageReports.length > 0 
      ? this.coverageReports.reduce((sum, r) => sum + r.statements.percentage, 0) / this.coverageReports.length
      : 0;

    return `
# Caesar Testing Summary Report

## Executive Summary
- **Test Execution Date**: ${new Date().toISOString()}
- **Total Test Duration**: ${(totalDuration / 1000).toFixed(2)} seconds
- **Test Suite Coverage**: Comprehensive (Unit, Integration, E2E, Security, Performance, Stress)

## Test Results Overview
- **Total Tests**: ${totalTests}
- **Passed**: ${passedTests} (${passRate}%)
- **Failed**: ${failedTests}
- **Skipped**: ${skippedTests}

## Code Coverage
- **Average Coverage**: ${avgCoverage.toFixed(2)}%
- **Target**: >95%
- **Status**: ${avgCoverage >= 95 ? '✅ PASS' : avgCoverage >= 80 ? '⚠️  WARNING' : '❌ FAIL'}

## Security Assessment
- **Critical Issues**: ${criticalFindings}
- **High Risk Issues**: ${highFindings}
- **Total Security Findings**: ${this.securityFindings.length}
- **Security Status**: ${criticalFindings === 0 ? (highFindings === 0 ? '✅ SECURE' : '⚠️  REVIEW NEEDED') : '❌ CRITICAL ISSUES'}

## Performance Benchmarks
${this.generatePerformanceSummary()}

## Readiness Assessment
${this.generateReadinessAssessment()}
`;
  }

  private generatePerformanceSummary(): string {
    const tpsMetrics = this.performanceMetrics.filter(m => m.metric.includes('TPS'));
    const gasMetrics = this.performanceMetrics.filter(m => m.metric.includes('gas'));
    const latencyMetrics = this.performanceMetrics.filter(m => m.metric.includes('latency') || m.metric.includes('time'));

    let summary = '';
    
    if (tpsMetrics.length > 0) {
      const avgTps = tpsMetrics.reduce((sum, m) => sum + Number(m.value), 0) / tpsMetrics.length;
      summary += `- **Average TPS**: ${avgTps.toFixed(2)} (Target: >10 TPS)\n`;
    }

    if (gasMetrics.length > 0) {
      const avgGas = gasMetrics.reduce((sum, m) => sum + Number(m.value), 0) / gasMetrics.length;
      summary += `- **Average Gas Usage**: ${Math.round(avgGas)} (Target: <100k gas per transaction)\n`;
    }

    if (latencyMetrics.length > 0) {
      const avgLatency = latencyMetrics.reduce((sum, m) => sum + Number(m.value), 0) / latencyMetrics.length;
      summary += `- **Average Response Time**: ${avgLatency.toFixed(2)}ms (Target: <200ms)\n`;
    }

    return summary || '- No performance metrics recorded';
  }

  private generateReadinessAssessment(): string {
    const totalTests = this.results.length;
    const passedTests = this.results.filter(r => r.status === 'passed').length;
    const passRate = totalTests > 0 ? (passedTests / totalTests) : 0;
    
    const criticalFindings = this.securityFindings.filter(f => f.severity === 'critical').length;
    const avgCoverage = this.coverageReports.length > 0 
      ? this.coverageReports.reduce((sum, r) => sum + r.statements.percentage, 0) / this.coverageReports.length
      : 0;

    const performancePass = this.performanceMetrics.filter(m => m.status === 'pass').length;
    const totalPerformanceMetrics = this.performanceMetrics.length;
    const performanceRate = totalPerformanceMetrics > 0 ? (performancePass / totalPerformanceMetrics) : 1;

    const readinessCriteria = [
      { name: 'Test Pass Rate', value: passRate, target: 0.95, weight: 30 },
      { name: 'Code Coverage', value: avgCoverage / 100, target: 0.95, weight: 25 },
      { name: 'Security', value: criticalFindings === 0 ? 1 : 0, target: 1, weight: 25 },
      { name: 'Performance', value: performanceRate, target: 0.8, weight: 20 }
    ];

    let totalScore = 0;
    let maxScore = 0;

    readinessCriteria.forEach(criteria => {
      const score = Math.min(criteria.value / criteria.target, 1) * criteria.weight;
      totalScore += score;
      maxScore += criteria.weight;
    });

    const readinessScore = (totalScore / maxScore) * 100;

    let status = '';
    let recommendation = '';

    if (readinessScore >= 95) {
      status = '🟢 PRODUCTION READY';
      recommendation = 'All criteria met. Ready for production deployment.';
    } else if (readinessScore >= 85) {
      status = '🟡 NEARLY READY';
      recommendation = 'Minor issues to address before production deployment.';
    } else if (readinessScore >= 70) {
      status = '🟠 NEEDS IMPROVEMENT';
      recommendation = 'Significant issues need to be resolved before production.';
    } else {
      status = '🔴 NOT READY';
      recommendation = 'Major issues must be addressed. Not suitable for production.';
    }

    return `
### Overall Readiness Score: ${readinessScore.toFixed(1)}%
### Status: ${status}

**Detailed Breakdown:**
${readinessCriteria.map(c => 
  `- ${c.name}: ${(c.value * 100).toFixed(1)}% (Target: ${(c.target * 100).toFixed(0)}%, Weight: ${c.weight}%)`
).join('\n')}

**Recommendation:** ${recommendation}
`;
  }

  generateDetailedReport(): string {
    return `
# Caesar Detailed Testing Report

${this.generateSummaryReport()}

## Detailed Test Results

### Test Suites Breakdown
${this.generateTestSuiteBreakdown()}

### Performance Metrics Detail
${this.generateDetailedPerformanceReport()}

### Security Findings Detail
${this.generateDetailedSecurityReport()}

### Code Coverage Detail
${this.generateDetailedCoverageReport()}

## Recommendations
${this.generateRecommendations()}

## Test Execution Log
${this.generateExecutionLog()}
`;
  }

  private generateTestSuiteBreakdown(): string {
    const suiteMap = new Map<string, TestResult[]>();
    
    this.results.forEach(result => {
      if (!suiteMap.has(result.testSuite)) {
        suiteMap.set(result.testSuite, []);
      }
      suiteMap.get(result.testSuite)!.push(result);
    });

    let breakdown = '';
    suiteMap.forEach((tests, suite) => {
      const passed = tests.filter(t => t.status === 'passed').length;
      const failed = tests.filter(t => t.status === 'failed').length;
      const total = tests.length;
      const passRate = total > 0 ? ((passed / total) * 100).toFixed(1) : '0.0';
      
      breakdown += `
#### ${suite}
- **Tests**: ${total}
- **Passed**: ${passed} (${passRate}%)
- **Failed**: ${failed}
- **Status**: ${failed === 0 ? '✅ PASS' : '❌ ISSUES DETECTED'}
`;

      if (failed > 0) {
        const failedTests = tests.filter(t => t.status === 'failed');
        breakdown += '\n**Failed Tests:**\n';
        failedTests.forEach(test => {
          breakdown += `- ${test.testName}: ${test.error || 'Unknown error'}\n`;
        });
      }
    });

    return breakdown;
  }

  private generateDetailedPerformanceReport(): string {
    if (this.performanceMetrics.length === 0) {
      return 'No performance metrics recorded.';
    }

    const metricsByCategory = new Map<string, PerformanceMetric[]>();
    
    this.performanceMetrics.forEach(metric => {
      const category = this.categorizeMetric(metric.metric);
      if (!metricsByCategory.has(category)) {
        metricsByCategory.set(category, []);
      }
      metricsByCategory.get(category)!.push(metric);
    });

    let report = '';
    metricsByCategory.forEach((metrics, category) => {
      report += `\n#### ${category}\n`;
      metrics.forEach(metric => {
        const status = metric.status === 'pass' ? '✅' : metric.status === 'warning' ? '⚠️' : '❌';
        const target = metric.target ? ` (Target: ${metric.target}${metric.unit})` : '';
        report += `- ${metric.metric}: ${metric.value}${metric.unit}${target} ${status}\n`;
      });
    });

    return report;
  }

  private categorizeMetric(metricName: string): string {
    if (metricName.includes('TPS') || metricName.includes('throughput')) return 'Throughput';
    if (metricName.includes('gas') || metricName.includes('Gas')) return 'Gas Efficiency';
    if (metricName.includes('time') || metricName.includes('latency') || metricName.includes('Time')) return 'Response Time';
    if (metricName.includes('memory') || metricName.includes('Memory')) return 'Memory Usage';
    if (metricName.includes('cross-chain') || metricName.includes('Cross')) return 'Cross-Chain Performance';
    return 'General';
  }

  private generateDetailedSecurityReport(): string {
    if (this.securityFindings.length === 0) {
      return '✅ No security issues found.';
    }

    const findingsByCategory = new Map<string, SecurityFinding[]>();
    
    this.securityFindings.forEach(finding => {
      if (!findingsByCategory.has(finding.category)) {
        findingsByCategory.set(finding.category, []);
      }
      findingsByCategory.get(finding.category)!.push(finding);
    });

    let report = '';
    findingsByCategory.forEach((findings, category) => {
      report += `\n#### ${category}\n`;
      findings.forEach((finding, index) => {
        const severityIcon = {
          'critical': '🔴',
          'high': '🟠',
          'medium': '🟡',
          'low': '🟢',
          'info': 'ℹ️'
        }[finding.severity];
        
        const statusIcon = finding.status === 'resolved' ? '✅' : finding.status === 'acknowledged' ? '⚠️' : '🔍';
        
        report += `
**${index + 1}. ${finding.description}**
- **Severity**: ${severityIcon} ${finding.severity.toUpperCase()}
- **Contract**: ${finding.contract || 'N/A'}
- **Function**: ${finding.function || 'N/A'}
- **Status**: ${statusIcon} ${finding.status.toUpperCase()}
- **Recommendation**: ${finding.recommendation}
`;
      });
    });

    return report;
  }

  private generateDetailedCoverageReport(): string {
    if (this.coverageReports.length === 0) {
      return 'No coverage data available.';
    }

    let report = '';
    this.coverageReports.forEach(coverage => {
      const overallCoverage = coverage.statements.percentage;
      const status = overallCoverage >= 95 ? '✅' : overallCoverage >= 80 ? '⚠️' : '❌';
      
      report += `
#### ${coverage.contract}
- **Overall Coverage**: ${overallCoverage.toFixed(2)}% ${status}
- **Statements**: ${coverage.statements.hit}/${coverage.statements.total} (${coverage.statements.percentage.toFixed(2)}%)
- **Branches**: ${coverage.branches.hit}/${coverage.branches.total} (${coverage.branches.percentage.toFixed(2)}%)
- **Functions**: ${coverage.functions.hit}/${coverage.functions.total} (${coverage.functions.percentage.toFixed(2)}%)
- **Lines**: ${coverage.lines.hit}/${coverage.lines.total} (${coverage.lines.percentage.toFixed(2)}%)
`;
    });

    return report;
  }

  private generateRecommendations(): string {
    const recommendations = [];

    // Test failure recommendations
    const failedTests = this.results.filter(r => r.status === 'failed').length;
    if (failedTests > 0) {
      recommendations.push(`🔧 **Fix ${failedTests} failing tests** - Address failing test cases before deployment.`);
    }

    // Coverage recommendations
    const avgCoverage = this.coverageReports.length > 0 
      ? this.coverageReports.reduce((sum, r) => sum + r.statements.percentage, 0) / this.coverageReports.length
      : 0;
    
    if (avgCoverage < 95) {
      recommendations.push(`📊 **Improve code coverage** - Current coverage is ${avgCoverage.toFixed(2)}%, target is >95%.`);
    }

    // Security recommendations
    const criticalFindings = this.securityFindings.filter(f => f.severity === 'critical').length;
    const highFindings = this.securityFindings.filter(f => f.severity === 'high').length;
    
    if (criticalFindings > 0) {
      recommendations.push(`🚨 **URGENT: Resolve ${criticalFindings} critical security issues** - Critical vulnerabilities must be fixed immediately.`);
    }
    
    if (highFindings > 0) {
      recommendations.push(`🔐 **Address ${highFindings} high-risk security issues** - High-risk vulnerabilities should be resolved before production.`);
    }

    // Performance recommendations
    const failedPerformance = this.performanceMetrics.filter(m => m.status === 'fail').length;
    if (failedPerformance > 0) {
      recommendations.push(`⚡ **Optimize performance** - ${failedPerformance} performance metrics are below target.`);
    }

    // General recommendations
    if (recommendations.length === 0) {
      recommendations.push('✅ **All criteria met** - The system is ready for production deployment.');
    }

    return recommendations.map(rec => `- ${rec}`).join('\n');
  }

  private generateExecutionLog(): string {
    const totalDuration = Date.now() - this.startTime;
    
    let log = `**Test Execution Started**: ${new Date(this.startTime).toISOString()}\n`;
    log += `**Total Execution Time**: ${(totalDuration / 1000).toFixed(2)} seconds\n\n`;
    
    // Group by test suite
    const suiteMap = new Map<string, TestResult[]>();
    this.results.forEach(result => {
      if (!suiteMap.has(result.testSuite)) {
        suiteMap.set(result.testSuite, []);
      }
      suiteMap.get(result.testSuite)!.push(result);
    });

    suiteMap.forEach((tests, suite) => {
      log += `**${suite}**\n`;
      tests.forEach(test => {
        const status = test.status === 'passed' ? '✅' : test.status === 'failed' ? '❌' : '⏭️';
        const gas = test.gasUsed ? ` (${test.gasUsed} gas)` : '';
        log += `  ${status} ${test.testName} - ${test.duration}ms${gas}\n`;
      });
      log += '\n';
    });

    return log;
  }

  // Save reports to files
  saveReports(): void {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    
    // Save summary report
    const summaryReport = this.generateSummaryReport();
    writeFileSync(
      join(this.outputDir, `caesar-token-test-summary-${timestamp}.md`),
      summaryReport
    );

    // Save detailed report
    const detailedReport = this.generateDetailedReport();
    writeFileSync(
      join(this.outputDir, `caesar-token-test-detailed-${timestamp}.md`),
      detailedReport
    );

    // Save JSON data for programmatic access
    const jsonData = {
      timestamp: new Date().toISOString(),
      results: this.results,
      performanceMetrics: this.performanceMetrics,
      securityFindings: this.securityFindings,
      coverageReports: this.coverageReports,
      summary: {
        totalTests: this.results.length,
        passedTests: this.results.filter(r => r.status === 'passed').length,
        failedTests: this.results.filter(r => r.status === 'failed').length,
        averageCoverage: this.coverageReports.length > 0 
          ? this.coverageReports.reduce((sum, r) => sum + r.statements.percentage, 0) / this.coverageReports.length
          : 0,
        criticalSecurityIssues: this.securityFindings.filter(f => f.severity === 'critical').length,
        executionDuration: Date.now() - this.startTime
      }
    };

    writeFileSync(
      join(this.outputDir, `caesar-token-test-data-${timestamp}.json`),
      JSON.stringify(jsonData, (key, value) => 
        typeof value === 'bigint' ? value.toString() : value, 2
      )
    );

    console.log(`📊 Test reports saved to ${this.outputDir}/`);
    console.log(`📋 Summary: caesar-token-test-summary-${timestamp}.md`);
    console.log(`📖 Detailed: caesar-token-test-detailed-${timestamp}.md`);
    console.log(`📄 Data: caesar-token-test-data-${timestamp}.json`);
  }

  // Utility methods for test integration
  recordTestStart(testSuite: string, testName: string): string {
    const testId = `${testSuite}::${testName}::${Date.now()}`;
    return testId;
  }

  recordTestEnd(testId: string, status: 'passed' | 'failed' | 'skipped', duration: number, error?: string, gasUsed?: bigint): void {
    const [testSuite, testName] = testId.split('::');
    
    this.addTestResult({
      testSuite,
      testName,
      status,
      duration,
      gasUsed,
      error
    });
  }

  // Performance recording helpers
  recordThroughput(operation: string, tps: number, target: number = 10): void {
    this.addPerformanceMetric({
      metric: `${operation} Throughput`,
      value: tps,
      unit: 'TPS',
      target,
      status: tps >= target ? 'pass' : tps >= target * 0.8 ? 'warning' : 'fail',
      timestamp: Date.now()
    });
  }

  recordGasUsage(operation: string, gas: bigint, target: bigint = BigInt(100000)): void {
    this.addPerformanceMetric({
      metric: `${operation} Gas Usage`,
      value: gas,
      unit: 'gas',
      target,
      status: gas <= target ? 'pass' : gas <= target * 120n / 100n ? 'warning' : 'fail',
      timestamp: Date.now()
    });
  }

  recordResponseTime(operation: string, timeMs: number, target: number = 200): void {
    this.addPerformanceMetric({
      metric: `${operation} Response Time`,
      value: timeMs,
      unit: 'ms',
      target,
      status: timeMs <= target ? 'pass' : timeMs <= target * 1.5 ? 'warning' : 'fail',
      timestamp: Date.now()
    });
  }

  // Security recording helpers
  recordSecurityFinding(
    severity: 'critical' | 'high' | 'medium' | 'low' | 'info',
    category: string,
    description: string,
    recommendation: string,
    contract?: string,
    functionName?: string
  ): void {
    this.addSecurityFinding({
      severity,
      category,
      description,
      recommendation,
      contract,
      function: functionName,
      status: 'open'
    });
  }

  // Coverage recording helpers
  recordCoverage(
    contract: string,
    statements: { hit: number; total: number },
    branches: { hit: number; total: number },
    functions: { hit: number; total: number },
    lines: { hit: number; total: number }
  ): void {
    this.addCoverageReport({
      contract,
      statements: { ...statements, percentage: (statements.hit / statements.total) * 100 },
      branches: { ...branches, percentage: (branches.hit / branches.total) * 100 },
      functions: { ...functions, percentage: (functions.hit / functions.total) * 100 },
      lines: { ...lines, percentage: (lines.hit / lines.total) * 100 }
    });
  }

  // Generate production readiness checklist
  generateProductionChecklist(): string {
    return `
# Caesar Production Readiness Checklist

## ✅ Testing Requirements
- [ ] Unit tests pass (>95% success rate)
- [ ] Integration tests pass (>95% success rate)  
- [ ] End-to-end tests pass (>90% success rate)
- [ ] Security audit completed
- [ ] Performance benchmarks met
- [ ] Stress testing completed

## ✅ Security Requirements
- [ ] No critical security vulnerabilities
- [ ] No high-risk security vulnerabilities
- [ ] Access controls verified
- [ ] Cross-chain security validated
- [ ] Emergency procedures tested

## ✅ Performance Requirements
- [ ] Transaction throughput >10 TPS
- [ ] Average gas usage <100k per transaction
- [ ] API response time <200ms (95th percentile)
- [ ] Cross-chain finality <3 seconds
- [ ] System uptime >99.9%

## ✅ Economic Model Validation
- [ ] Demurrage system validated under stress
- [ ] Anti-speculation mechanisms tested
- [ ] Stability pool operations verified
- [ ] Fiat backing maintained at 100%
- [ ] Emergency controls functional

## ✅ Compliance Requirements
- [ ] KYC/AML workflows tested
- [ ] Transaction monitoring active
- [ ] Regulatory reporting ready
- [ ] Data privacy compliance verified
- [ ] Audit trail complete

## ✅ Operational Readiness
- [ ] Monitoring systems deployed
- [ ] Alerting configured
- [ ] Incident response procedures documented
- [ ] Recovery procedures tested
- [ ] Support procedures established

## 🎯 Go-Live Criteria
All above requirements must be met with green status (✅) before production deployment.

**Deployment Approval Required From:**
- [ ] Technical Lead
- [ ] Security Lead  
- [ ] Product Manager
- [ ] Compliance Officer
`;
  }
}

// Export singleton instance
export const testReporter = new TestReporter();

// Helper functions for easy integration
export function startTest(testSuite: string, testName: string): string {
  return testReporter.recordTestStart(testSuite, testName);
}

export function endTest(testId: string, status: 'passed' | 'failed' | 'skipped', duration: number, error?: string, gasUsed?: bigint): void {
  testReporter.recordTestEnd(testId, status, duration, error, gasUsed);
}

export function recordPerformance(operation: string, tps?: number, gas?: bigint, timeMs?: number): void {
  if (tps !== undefined) testReporter.recordThroughput(operation, tps);
  if (gas !== undefined) testReporter.recordGasUsage(operation, gas);
  if (timeMs !== undefined) testReporter.recordResponseTime(operation, timeMs);
}

export function recordSecurity(
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info',
  category: string,
  description: string,
  recommendation: string,
  contract?: string,
  functionName?: string
): void {
  testReporter.recordSecurityFinding(severity, category, description, recommendation, contract, functionName);
}

export function generateReports(): void {
  testReporter.saveReports();
  console.log(testReporter.generateSummaryReport());
}