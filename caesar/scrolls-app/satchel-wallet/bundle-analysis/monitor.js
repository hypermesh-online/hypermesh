#!/usr/bin/env node

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.


/**
 * Bundle Size Monitor
 * Tracks bundle size changes and alerts on budget violations
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const CONFIG_PATH = path.join(__dirname, 'performance-config.json');
const BASELINE_PATH = path.join(__dirname, 'baseline-measurement.txt');
const HISTORY_PATH = path.join(__dirname, 'size-history.json');

function loadConfig() {
  return JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
}

function parseSize(sizeStr) {
  const match = sizeStr.match(/(\d+\.?\d*)([KMG]?)B?/);
  if (!match) return 0;
  
  const value = parseFloat(match[1]);
  const unit = match[2];
  
  switch (unit) {
    case 'G': return value * 1024 * 1024 * 1024;
    case 'M': return value * 1024 * 1024;
    case 'K': return value * 1024;
    default: return value;
  }
}

function formatSize(bytes) {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  } else if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)}KB`;
  }
  return `${bytes}B`;
}

function getCurrentBundleSize() {
  try {
    const output = execSync('du -sb dist/', { encoding: 'utf8' });
    const sizeMatch = output.match(/^(\d+)/);
    return sizeMatch ? parseInt(sizeMatch[1]) : 0;
  } catch (error) {
    console.error('Failed to get bundle size:', error.message);
    return 0;
  }
}

function getDetailedSizes() {
  try {
    const output = execSync('ls -l dist/assets/*.js | grep -E "(web3|core|index|react)" | sort -k5 -nr', { encoding: 'utf8' });
    const lines = output.trim().split('\n');
    
    const chunks = {};
    lines.forEach(line => {
      const parts = line.split(/\s+/);
      const size = parseInt(parts[4]);
      const filename = parts[8];
      
      if (filename.includes('web3')) chunks.web3 = size;
      else if (filename.includes('core')) chunks.core = size;
      else if (filename.includes('react')) chunks.react = size;
      else if (filename.includes('index')) {
        const indexKey = `index_${Object.keys(chunks).filter(k => k.startsWith('index')).length + 1}`;
        chunks[indexKey] = size;
      }
    });
    
    return chunks;
  } catch (error) {
    console.error('Failed to get detailed sizes:', error.message);
    return {};
  }
}

function checkBudgets(currentSize, config) {
  const budgets = config.bundleBudgets;
  const violations = [];
  
  const totalLimitBytes = parseSize(budgets.total.limit);
  if (currentSize > totalLimitBytes) {
    violations.push({
      type: 'total',
      current: formatSize(currentSize),
      limit: budgets.total.limit,
      overage: formatSize(currentSize - totalLimitBytes)
    });
  }
  
  return violations;
}

function updateHistory(size, chunks) {
  let history = [];
  if (fs.existsSync(HISTORY_PATH)) {
    history = JSON.parse(fs.readFileSync(HISTORY_PATH, 'utf8'));
  }
  
  const entry = {
    timestamp: new Date().toISOString(),
    totalSize: size,
    chunks,
    formattedSize: formatSize(size)
  };
  
  history.push(entry);
  
  // Keep only last 50 entries
  if (history.length > 50) {
    history = history.slice(-50);
  }
  
  fs.writeFileSync(HISTORY_PATH, JSON.stringify(history, null, 2));
  return entry;
}

function generateReport(current, previous, violations) {
  const report = {
    timestamp: new Date().toISOString(),
    current: {
      size: formatSize(current.totalSize),
      chunks: Object.entries(current.chunks).map(([name, size]) => ({
        name,
        size: formatSize(size)
      }))
    },
    changes: previous ? {
      sizeDelta: formatSize(current.totalSize - previous.totalSize),
      percentage: ((current.totalSize - previous.totalSize) / previous.totalSize * 100).toFixed(2) + '%'
    } : null,
    budgetViolations: violations,
    status: violations.length > 0 ? 'WARNING' : 'OK'
  };
  
  return report;
}

function main() {
  console.log('🔍 Bundle Size Monitor - Caesar Token Satchel Wallet');
  console.log('==================================================');
  
  const config = loadConfig();
  const currentSize = getCurrentBundleSize();
  const currentChunks = getDetailedSizes();
  
  if (currentSize === 0) {
    console.error('❌ Failed to determine bundle size. Run "npm run build" first.');
    process.exit(1);
  }
  
  console.log(`📦 Current Bundle Size: ${formatSize(currentSize)}`);
  
  // Check budgets
  const violations = checkBudgets(currentSize, config);
  
  // Load history
  let previousEntry = null;
  if (fs.existsSync(HISTORY_PATH)) {
    const history = JSON.parse(fs.readFileSync(HISTORY_PATH, 'utf8'));
    previousEntry = history[history.length - 1] || null;
  }
  
  // Update history
  const currentEntry = updateHistory(currentSize, currentChunks);
  
  // Generate report
  const report = generateReport(currentEntry, previousEntry, violations);
  
  // Display results
  console.log('\n📊 Chunk Breakdown:');
  report.current.chunks.forEach(chunk => {
    console.log(`  ${chunk.name}: ${chunk.size}`);
  });
  
  if (report.changes) {
    console.log(`\n📈 Change from last measurement: ${report.changes.sizeDelta} (${report.changes.percentage})`);
  }
  
  if (violations.length > 0) {
    console.log('\n⚠️  Budget Violations:');
    violations.forEach(v => {
      console.log(`  ${v.type}: ${v.current} exceeds ${v.limit} by ${v.overage}`);
    });
  } else {
    console.log('\n✅ All budgets are within limits');
  }
  
  // Save report
  const reportPath = path.join(__dirname, `report-${Date.now()}.json`);
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
  
  console.log(`\n📄 Report saved: ${reportPath}`);
  console.log(`📊 Visual analysis: npm run analyze`);
  
  // Exit with error code if violations exist
  process.exit(violations.length > 0 ? 1 : 0);
}

if (require.main === module) {
  main();
}

module.exports = { parseSize, formatSize, checkBudgets };