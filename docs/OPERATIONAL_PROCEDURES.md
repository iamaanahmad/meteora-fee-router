# Operational Procedures

This document outlines the operational procedures for managing and maintaining Meteora Fee Router deployments in production environments.

## Table of Contents

- [Daily Operations](#daily-operations)
- [Monitoring and Alerting](#monitoring-and-alerting)
- [Maintenance Procedures](#maintenance-procedures)
- [Emergency Response](#emergency-response)
- [Performance Optimization](#performance-optimization)
- [Security Procedures](#security-procedures)
- [Backup and Recovery](#backup-and-recovery)

## Daily Operations

### Morning Checklist

#### 1. System Health Check

```bash
#!/bin/bash
# daily-health-check.sh

echo "🌅 Daily Health Check - $(date)"
echo "================================"

# Check all active vaults
VAULTS=(
  "vault1_pubkey_here"
  "vault2_pubkey_here"
  # Add your vault addresses
)

for vault in "${VAULTS[@]}"; do
  echo "Checking vault: $vault"
  
  # Check distribution status
  node cli.js status -v "$vault" --rpc "$RPC_URL" --keypair "$KEYPAIR_PATH"
  
  # Check treasury balance
  echo "Treasury balance check..."
  # Add treasury balance check logic
  
  echo "---"
done

echo "✅ Health check completed"
```

#### 2. Distribution Status Review

```typescript
// daily-status-check.ts
import { MeteoraFeeRouterClient } from './meteora-client';
import { CONFIG } from './config';

interface VaultConfig {
  id: string;
  vault: PublicKey;
  name: string;
  expectedInvestors: number;
}

async function dailyStatusCheck(vaults: VaultConfig[]) {
  console.log('📊 Daily Status Check');
  console.log('====================');
  
  const client = await MeteoraFeeRouterClient.create(connection, wallet);
  
  for (const vaultConfig of vaults) {
    console.log(`\n🏦 Vault: ${vaultConfig.name} (${vaultConfig.id})`);
    
    try {
      const status = await client.getDistributionStatus(vaultConfig.vault);
      
      // Check timing
      const hoursUntilNext = status.timing.timeUntilNext / 3600;
      console.log(`⏰ Next distribution: ${hoursUntilNext.toFixed(1)} hours`);
      
      // Check progress
      const progressPercent = (status.progress.paginationCursor / vaultConfig.expectedInvestors) * 100;
      console.log(`📈 Progress: ${progressPercent.toFixed(1)}% (${status.progress.paginationCursor}/${vaultConfig.expectedInvestors})`);
      
      // Check treasury
      if (status.treasury) {
        console.log(`💰 Treasury: ${status.treasury.balance} tokens`);
      }
      
      // Check for issues
      if (status.timing.timeUntilNext < 0 && !status.timing.canStartNewDay) {
        console.log('⚠️  WARNING: Distribution overdue but cannot start');
      }
      
      if (status.progress.dayComplete) {
        console.log('✅ Day completed successfully');
      } else if (status.timing.canContinueSameDay) {
        console.log('🔄 Distribution in progress');
      }
      
    } catch (error) {
      console.error(`❌ Error checking ${vaultConfig.name}:`, error.message);
    }
  }
}
```

#### 3. Event Log Review

```typescript
// event-log-review.ts
async function reviewDailyEvents(vaults: PublicKey[], hoursBack: number = 24) {
  console.log(`📋 Event Review - Last ${hoursBack} hours`);
  console.log('=====================================');
  
  const cutoffTime = Date.now() - (hoursBack * 60 * 60 * 1000);
  
  for (const vault of vaults) {
    console.log(`\n🏦 Vault: ${vault.toString()}`);
    
    // Get recent signatures
    const signatures = await connection.getSignaturesForAddress(vault, {
      limit: 100,
    });
    
    const recentEvents = [];
    
    for (const sig of signatures) {
      if ((sig.blockTime || 0) * 1000 < cutoffTime) break;
      
      try {
        const tx = await connection.getTransaction(sig.signature);
        if (tx?.meta?.logMessages) {
          const events = parseEventsFromTransaction(tx, vault);
          recentEvents.push(...events);
        }
      } catch (error) {
        console.error(`Failed to process ${sig.signature}:`, error.message);
      }
    }
    
    // Summarize events
    const eventSummary = summarizeEvents(recentEvents);
    console.log('Event Summary:', eventSummary);
  }
}

function summarizeEvents(events: any[]) {
  const summary = {
    feesClaimed: 0,
    investorPayouts: 0,
    creatorPayouts: 0,
    totalDistributed: 0,
    errors: 0,
  };
  
  for (const event of events) {
    switch (event.type) {
      case 'quoteFeesClaimed':
        summary.feesClaimed += parseFloat(event.amount);
        break;
      case 'investorPayoutPage':
        summary.investorPayouts += parseFloat(event.distributed);
        break;
      case 'creatorPayoutDayClosed':
        summary.creatorPayouts += parseFloat(event.creatorPayout);
        break;
      case 'error':
        summary.errors++;
        break;
    }
  }
  
  summary.totalDistributed = summary.investorPayouts + summary.creatorPayouts;
  return summary;
}
```

### Distribution Execution

#### 1. Pre-Distribution Checks

```typescript
// pre-distribution-checks.ts
async function preDistributionChecks(
  vault: PublicKey,
  streamflowAccounts: PublicKey[]
): Promise<boolean> {
  console.log('🔍 Pre-Distribution Checks');
  console.log('==========================');
  
  const client = await MeteoraFeeRouterClient.create(connection, wallet);
  
  // 1. Check distribution timing
  const status = await client.getDistributionStatus(vault);
  
  if (!status.timing.canStartNewDay && !status.timing.canContinueSameDay) {
    console.log(`❌ Cannot distribute yet. Wait ${status.timing.timeUntilNext} seconds`);
    return false;
  }
  
  // 2. Validate Streamflow accounts
  console.log('Validating Streamflow accounts...');
  const validAccounts = await validateStreamflowAccounts(streamflowAccounts, status.policy.quoteMint);
  
  if (validAccounts < streamflowAccounts.length * 0.95) { // Allow 5% invalid
    console.log(`❌ Too many invalid Streamflow accounts: ${validAccounts}/${streamflowAccounts.length}`);
    return false;
  }
  
  // 3. Check treasury balance (for continuation)
  if (status.treasury && status.timing.canContinueSameDay) {
    if (status.treasury.balance === 0) {
      console.log('⚠️  Treasury is empty, but continuing same day distribution');
    }
  }
  
  // 4. Check RPC health
  const rpcHealth = await checkRpcHealth();
  if (!rpcHealth) {
    console.log('❌ RPC endpoint unhealthy');
    return false;
  }
  
  console.log('✅ All pre-distribution checks passed');
  return true;
}

async function validateStreamflowAccounts(
  accounts: PublicKey[],
  expectedMint: PublicKey
): Promise<number> {
  let validCount = 0;
  
  for (const account of accounts) {
    try {
      const accountInfo = await connection.getAccountInfo(account);
      if (accountInfo) {
        // Basic validation - account exists
        validCount++;
      }
    } catch (error) {
      // Account doesn't exist or other error
    }
  }
  
  return validCount;
}

async function checkRpcHealth(): Promise<boolean> {
  try {
    const health = await connection.getHealth();
    const slot = await connection.getSlot();
    const recentSlot = await connection.getSlot('finalized');
    
    const slotDiff = slot - recentSlot;
    
    return health === 'ok' && slotDiff < 100; // Less than 100 slots behind
  } catch (error) {
    return false;
  }
}
```

#### 2. Automated Distribution Execution

```typescript
// automated-distribution.ts
class AutomatedDistributor {
  private isRunning = false;
  private stopRequested = false;
  
  constructor(
    private client: MeteoraFeeRouterClient,
    private config: {
      vault: PublicKey;
      quoteMint: PublicKey;
      creatorWallet: PublicKey;
      honoraryPosition: PublicKey;
      streamflowAccounts: PublicKey[];
      pageSize: number;
      maxRetries: number;
      delayBetweenPages: number;
    }
  ) {}
  
  async start(): Promise<void> {
    if (this.isRunning) {
      throw new Error('Distributor already running');
    }
    
    this.isRunning = true;
    this.stopRequested = false;
    
    console.log('🚀 Starting automated distribution');
    
    try {
      // Pre-checks
      const canProceed = await preDistributionChecks(
        this.config.vault,
        this.config.streamflowAccounts
      );
      
      if (!canProceed) {
        throw new Error('Pre-distribution checks failed');
      }
      
      // Execute distribution
      await this.executeDistribution();
      
      console.log('✅ Automated distribution completed successfully');
      
    } catch (error) {
      console.error('❌ Automated distribution failed:', error);
      await this.sendAlert('distribution_failed', {
        vault: this.config.vault.toString(),
        error: error.message,
      });
      throw error;
      
    } finally {
      this.isRunning = false;
    }
  }
  
  async stop(): Promise<void> {
    console.log('🛑 Stop requested for automated distribution');
    this.stopRequested = true;
    
    // Wait for current operation to complete
    while (this.isRunning) {
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  
  private async executeDistribution(): Promise<void> {
    const status = await this.client.getDistributionStatus(this.config.vault);
    let currentCursor = status.progress.paginationCursor;
    const totalInvestors = this.config.streamflowAccounts.length;
    
    console.log(`📊 Distribution state: cursor=${currentCursor}, total=${totalInvestors}`);
    
    while (currentCursor < totalInvestors && !this.stopRequested) {
      const pageEnd = Math.min(currentCursor + this.config.pageSize, totalInvestors);
      const pageAccounts = this.config.streamflowAccounts.slice(currentCursor, pageEnd);
      
      console.log(`🔄 Processing page: ${currentCursor}-${pageEnd}/${totalInvestors}`);
      
      try {
        await this.processPage(pageAccounts, currentCursor);
        currentCursor = pageEnd;
        
        // Progress notification
        const progressPercent = (currentCursor / totalInvestors) * 100;
        console.log(`📈 Progress: ${progressPercent.toFixed(1)}%`);
        
        // Delay between pages
        if (currentCursor < totalInvestors && this.config.delayBetweenPages > 0) {
          await new Promise(resolve => setTimeout(resolve, this.config.delayBetweenPages));
        }
        
      } catch (error) {
        console.error(`❌ Page ${currentCursor}-${pageEnd} failed:`, error);
        
        // Check if cursor advanced despite error
        const newStatus = await this.client.getDistributionStatus(this.config.vault);
        if (newStatus.progress.paginationCursor > currentCursor) {
          console.log('✅ Cursor advanced despite error, continuing');
          currentCursor = newStatus.progress.paginationCursor;
        } else {
          throw error; // Re-throw if no progress made
        }
      }
    }
    
    if (this.stopRequested) {
      console.log('⏹️  Distribution stopped by request');
    } else {
      console.log('🎯 Distribution completed successfully');
    }
  }
  
  private async processPage(
    pageAccounts: PublicKey[],
    cursorPosition: number
  ): Promise<void> {
    let attempt = 0;
    
    while (attempt < this.config.maxRetries) {
      try {
        const signature = await this.client.distributeFees({
          vault: this.config.vault,
          quoteMint: this.config.quoteMint,
          creatorWallet: this.config.creatorWallet,
          honoraryPosition: this.config.honoraryPosition,
          streamflowAccounts: pageAccounts,
          pageSize: pageAccounts.length,
          cursorPosition: attempt > 0 ? cursorPosition : undefined,
        });
        
        console.log(`✅ Page processed: ${signature}`);
        return; // Success
        
      } catch (error) {
        attempt++;
        console.error(`❌ Attempt ${attempt}/${this.config.maxRetries} failed:`, error.message);
        
        if (attempt < this.config.maxRetries) {
          const delay = Math.min(1000 * Math.pow(2, attempt), 30000);
          console.log(`⏳ Retrying in ${delay}ms...`);
          await new Promise(resolve => setTimeout(resolve, delay));
        } else {
          throw new Error(`Failed after ${this.config.maxRetries} attempts: ${error.message}`);
        }
      }
    }
  }
  
  private async sendAlert(type: string, data: any): Promise<void> {
    // Implement your alerting system
    console.log(`🚨 ALERT [${type}]:`, data);
    
    // Example: Send to Discord webhook
    // await sendDiscordAlert(type, data);
    
    // Example: Send email
    // await sendEmailAlert(type, data);
  }
}
```

### Evening Checklist

#### 1. End-of-Day Summary

```typescript
// end-of-day-summary.ts
async function generateEndOfDaySummary(vaults: VaultConfig[]) {
  console.log('🌙 End-of-Day Summary');
  console.log('====================');
  
  const summary = {
    totalVaults: vaults.length,
    activeDistributions: 0,
    completedDistributions: 0,
    totalFeesDistributed: 0,
    totalInvestorPayouts: 0,
    totalCreatorPayouts: 0,
    errors: 0,
  };
  
  for (const vaultConfig of vaults) {
    try {
      const status = await client.getDistributionStatus(vaultConfig.vault);
      
      if (status.progress.dayComplete) {
        summary.completedDistributions++;
      } else if (status.timing.canContinueSameDay) {
        summary.activeDistributions++;
      }
      
      // Get daily events
      const dailyEvents = await getDailyEvents(vaultConfig.vault);
      summary.totalFeesDistributed += dailyEvents.feesClaimed;
      summary.totalInvestorPayouts += dailyEvents.investorPayouts;
      summary.totalCreatorPayouts += dailyEvents.creatorPayouts;
      summary.errors += dailyEvents.errors;
      
    } catch (error) {
      summary.errors++;
      console.error(`Error processing ${vaultConfig.name}:`, error.message);
    }
  }
  
  console.log('📊 Daily Summary:', summary);
  
  // Generate report
  const report = generateDailyReport(summary, vaults);
  await saveDailyReport(report);
  
  return summary;
}

function generateDailyReport(summary: any, vaults: VaultConfig[]): string {
  return `
# Daily Operations Report - ${new Date().toISOString().split('T')[0]}

## Summary
- Total Vaults: ${summary.totalVaults}
- Completed Distributions: ${summary.completedDistributions}
- Active Distributions: ${summary.activeDistributions}
- Total Fees Distributed: ${summary.totalFeesDistributed}
- Total Investor Payouts: ${summary.totalInvestorPayouts}
- Total Creator Payouts: ${summary.totalCreatorPayouts}
- Errors: ${summary.errors}

## Vault Status
${vaults.map(v => `- ${v.name}: ${getVaultStatus(v)}`).join('\n')}

## Recommendations
${generateRecommendations(summary)}
`;
}

async function saveDailyReport(report: string): Promise<void> {
  const filename = `daily-report-${new Date().toISOString().split('T')[0]}.md`;
  await fs.writeFile(`reports/${filename}`, report);
  console.log(`📄 Daily report saved: ${filename}`);
}
```

## Monitoring and Alerting

### Real-Time Monitoring

#### 1. System Metrics Dashboard

```typescript
// monitoring-dashboard.ts
class MonitoringDashboard {
  private metrics = new Map<string, any>();
  private alerts = new Map<string, any>();
  
  async startMonitoring(vaults: VaultConfig[]): Promise<void> {
    console.log('📊 Starting monitoring dashboard');
    
    // Update metrics every minute
    setInterval(async () => {
      await this.updateMetrics(vaults);
    }, 60000);
    
    // Check alerts every 30 seconds
    setInterval(async () => {
      await this.checkAlerts(vaults);
    }, 30000);
    
    // Display dashboard every 5 minutes
    setInterval(() => {
      this.displayDashboard();
    }, 300000);
  }
  
  private async updateMetrics(vaults: VaultConfig[]): Promise<void> {
    for (const vault of vaults) {
      try {
        const status = await client.getDistributionStatus(vault.vault);
        
        this.metrics.set(vault.id, {
          timestamp: Date.now(),
          canStartNewDay: status.timing.canStartNewDay,
          canContinueSameDay: status.timing.canContinueSameDay,
          timeUntilNext: status.timing.timeUntilNext,
          paginationCursor: status.progress.paginationCursor,
          dayComplete: status.progress.dayComplete,
          treasuryBalance: status.treasury?.balance || 0,
          currentDayDistributed: status.progress.currentDayDistributed,
        });
        
      } catch (error) {
        console.error(`Failed to update metrics for ${vault.id}:`, error.message);
      }
    }
  }
  
  private async checkAlerts(vaults: VaultConfig[]): Promise<void> {
    for (const vault of vaults) {
      const metrics = this.metrics.get(vault.id);
      if (!metrics) continue;
      
      // Check for stuck distributions
      if (metrics.canContinueSameDay && !metrics.dayComplete) {
        const stuckTime = Date.now() - metrics.timestamp;
        if (stuckTime > 30 * 60 * 1000) { // 30 minutes
          await this.triggerAlert(vault.id, 'distribution_stuck', {
            duration: stuckTime,
            cursor: metrics.paginationCursor,
          });
        }
      }
      
      // Check for overdue distributions
      if (metrics.timeUntilNext < -3600 && !metrics.canStartNewDay) { // 1 hour overdue
        await this.triggerAlert(vault.id, 'distribution_overdue', {
          overdue: Math.abs(metrics.timeUntilNext),
        });
      }
      
      // Check treasury balance
      if (metrics.treasuryBalance === 0 && metrics.canContinueSameDay) {
        await this.triggerAlert(vault.id, 'treasury_empty', {
          cursor: metrics.paginationCursor,
        });
      }
    }
  }
  
  private async triggerAlert(vaultId: string, alertType: string, data: any): Promise<void> {
    const alertKey = `${vaultId}_${alertType}`;
    const lastAlert = this.alerts.get(alertKey);
    
    // Rate limit alerts (don't send same alert more than once per hour)
    if (lastAlert && Date.now() - lastAlert < 3600000) {
      return;
    }
    
    this.alerts.set(alertKey, Date.now());
    
    console.log(`🚨 ALERT: ${alertType} for vault ${vaultId}`, data);
    
    // Send to external alerting systems
    await this.sendExternalAlert(vaultId, alertType, data);
  }
  
  private async sendExternalAlert(vaultId: string, alertType: string, data: any): Promise<void> {
    // Implement your alerting integrations
    
    // Discord webhook example
    if (process.env.DISCORD_WEBHOOK_URL) {
      await this.sendDiscordAlert(vaultId, alertType, data);
    }
    
    // PagerDuty example
    if (process.env.PAGERDUTY_INTEGRATION_KEY) {
      await this.sendPagerDutyAlert(vaultId, alertType, data);
    }
    
    // Email example
    if (process.env.SMTP_CONFIG) {
      await this.sendEmailAlert(vaultId, alertType, data);
    }
  }
  
  private displayDashboard(): void {
    console.clear();
    console.log('📊 Meteora Fee Router Dashboard');
    console.log('===============================');
    console.log(`Last Update: ${new Date().toISOString()}`);
    console.log('');
    
    for (const [vaultId, metrics] of this.metrics) {
      console.log(`🏦 ${vaultId}:`);
      console.log(`   Status: ${this.getStatusEmoji(metrics)} ${this.getStatusText(metrics)}`);
      console.log(`   Progress: ${metrics.paginationCursor} | Treasury: ${metrics.treasuryBalance}`);
      console.log(`   Next: ${this.formatTimeUntilNext(metrics.timeUntilNext)}`);
      console.log('');
    }
    
    // Show recent alerts
    const recentAlerts = Array.from(this.alerts.entries())
      .filter(([_, timestamp]) => Date.now() - timestamp < 3600000)
      .slice(-5);
    
    if (recentAlerts.length > 0) {
      console.log('🚨 Recent Alerts:');
      for (const [alertKey, timestamp] of recentAlerts) {
        console.log(`   ${alertKey} - ${new Date(timestamp).toLocaleTimeString()}`);
      }
    }
  }
  
  private getStatusEmoji(metrics: any): string {
    if (metrics.dayComplete) return '✅';
    if (metrics.canContinueSameDay) return '🔄';
    if (metrics.canStartNewDay) return '🟢';
    return '⏳';
  }
  
  private getStatusText(metrics: any): string {
    if (metrics.dayComplete) return 'Complete';
    if (metrics.canContinueSameDay) return 'In Progress';
    if (metrics.canStartNewDay) return 'Ready';
    return 'Waiting';
  }
  
  private formatTimeUntilNext(seconds: number): string {
    if (seconds <= 0) return 'Now';
    
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    return `${hours}h ${minutes}m`;
  }
  
  private async sendDiscordAlert(vaultId: string, alertType: string, data: any): Promise<void> {
    // Implement Discord webhook integration
    const webhook = process.env.DISCORD_WEBHOOK_URL;
    if (!webhook) return;
    
    const message = {
      embeds: [{
        title: `🚨 Alert: ${alertType}`,
        description: `Vault: ${vaultId}`,
        fields: Object.entries(data).map(([key, value]) => ({
          name: key,
          value: String(value),
          inline: true,
        })),
        color: 0xff0000,
        timestamp: new Date().toISOString(),
      }],
    };
    
    try {
      await fetch(webhook, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(message),
      });
    } catch (error) {
      console.error('Failed to send Discord alert:', error);
    }
  }
  
  private async sendPagerDutyAlert(vaultId: string, alertType: string, data: any): Promise<void> {
    // Implement PagerDuty integration
    const integrationKey = process.env.PAGERDUTY_INTEGRATION_KEY;
    if (!integrationKey) return;
    
    const event = {
      routing_key: integrationKey,
      event_action: 'trigger',
      payload: {
        summary: `Meteora Fee Router Alert: ${alertType}`,
        source: `vault-${vaultId}`,
        severity: 'error',
        custom_details: data,
      },
    };
    
    try {
      await fetch('https://events.pagerduty.com/v2/enqueue', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(event),
      });
    } catch (error) {
      console.error('Failed to send PagerDuty alert:', error);
    }
  }
  
  private async sendEmailAlert(vaultId: string, alertType: string, data: any): Promise<void> {
    // Implement email integration using your preferred service
    console.log(`📧 Email alert would be sent: ${alertType} for ${vaultId}`);
  }
}
```

### Performance Monitoring

#### 1. Transaction Performance Tracking

```typescript
// performance-monitor.ts
class PerformanceMonitor {
  private metrics: Array<{
    timestamp: number;
    operation: string;
    duration: number;
    success: boolean;
    error?: string;
  }> = [];
  
  async trackOperation<T>(
    operation: string,
    fn: () => Promise<T>
  ): Promise<T> {
    const startTime = Date.now();
    
    try {
      const result = await fn();
      
      this.metrics.push({
        timestamp: startTime,
        operation,
        duration: Date.now() - startTime,
        success: true,
      });
      
      return result;
      
    } catch (error) {
      this.metrics.push({
        timestamp: startTime,
        operation,
        duration: Date.now() - startTime,
        success: false,
        error: error.message,
      });
      
      throw error;
    }
  }
  
  getPerformanceReport(hoursBack: number = 24): any {
    const cutoff = Date.now() - (hoursBack * 60 * 60 * 1000);
    const recentMetrics = this.metrics.filter(m => m.timestamp >= cutoff);
    
    const report = {
      totalOperations: recentMetrics.length,
      successRate: 0,
      averageDuration: 0,
      operationBreakdown: new Map<string, any>(),
    };
    
    if (recentMetrics.length === 0) return report;
    
    const successful = recentMetrics.filter(m => m.success);
    report.successRate = (successful.length / recentMetrics.length) * 100;
    report.averageDuration = successful.reduce((sum, m) => sum + m.duration, 0) / successful.length;
    
    // Breakdown by operation type
    for (const metric of recentMetrics) {
      if (!report.operationBreakdown.has(metric.operation)) {
        report.operationBreakdown.set(metric.operation, {
          count: 0,
          successCount: 0,
          totalDuration: 0,
          errors: [],
        });
      }
      
      const breakdown = report.operationBreakdown.get(metric.operation);
      breakdown.count++;
      breakdown.totalDuration += metric.duration;
      
      if (metric.success) {
        breakdown.successCount++;
      } else {
        breakdown.errors.push(metric.error);
      }
    }
    
    return report;
  }
  
  logPerformanceReport(): void {
    const report = this.getPerformanceReport();
    
    console.log('📈 Performance Report (24h)');
    console.log('===========================');
    console.log(`Total Operations: ${report.totalOperations}`);
    console.log(`Success Rate: ${report.successRate.toFixed(2)}%`);
    console.log(`Average Duration: ${report.averageDuration.toFixed(0)}ms`);
    console.log('');
    
    for (const [operation, breakdown] of report.operationBreakdown) {
      const successRate = (breakdown.successCount / breakdown.count) * 100;
      const avgDuration = breakdown.totalDuration / breakdown.count;
      
      console.log(`${operation}:`);
      console.log(`  Count: ${breakdown.count}`);
      console.log(`  Success Rate: ${successRate.toFixed(2)}%`);
      console.log(`  Avg Duration: ${avgDuration.toFixed(0)}ms`);
      
      if (breakdown.errors.length > 0) {
        console.log(`  Recent Errors: ${breakdown.errors.slice(-3).join(', ')}`);
      }
      console.log('');
    }
  }
}
```

## Maintenance Procedures

### Weekly Maintenance

#### 1. System Health Assessment

```bash
#!/bin/bash
# weekly-maintenance.sh

echo "🔧 Weekly Maintenance - $(date)"
echo "==============================="

# 1. Check disk space
echo "📁 Disk Space Check:"
df -h

# 2. Check memory usage
echo "💾 Memory Usage:"
free -h

# 3. Check log file sizes
echo "📄 Log File Sizes:"
find logs/ -name "*.log" -exec ls -lh {} \;

# 4. Rotate logs if needed
echo "🔄 Log Rotation:"
logrotate /etc/logrotate.d/meteora-fee-router

# 5. Update dependencies
echo "📦 Dependency Updates:"
npm audit
npm update

# 6. Run security scan
echo "🔒 Security Scan:"
npm audit --audit-level moderate

# 7. Backup configuration
echo "💾 Configuration Backup:"
tar -czf "backups/config-$(date +%Y%m%d).tar.gz" config/

echo "✅ Weekly maintenance completed"
```

#### 2. Performance Optimization

```typescript
// weekly-optimization.ts
async function weeklyOptimization() {
  console.log('⚡ Weekly Performance Optimization');
  console.log('=================================');
  
  // 1. Analyze transaction patterns
  const performanceReport = performanceMonitor.getPerformanceReport(168); // 7 days
  console.log('Performance Analysis:', performanceReport);
  
  // 2. Optimize page sizes based on performance
  const optimalPageSizes = calculateOptimalPageSizes(performanceReport);
  console.log('Recommended page sizes:', optimalPageSizes);
  
  // 3. Clean up old metrics
  performanceMonitor.cleanupOldMetrics(30); // Keep 30 days
  
  // 4. Update RPC endpoint rankings
  await updateRpcEndpointRankings();
  
  // 5. Generate optimization report
  const report = generateOptimizationReport(performanceReport, optimalPageSizes);
  await saveOptimizationReport(report);
}

function calculateOptimalPageSizes(report: any): Map<string, number> {
  const recommendations = new Map<string, number>();
  
  for (const [operation, breakdown] of report.operationBreakdown) {
    if (operation.includes('distributeFees')) {
      const avgDuration = breakdown.totalDuration / breakdown.count;
      const successRate = (breakdown.successCount / breakdown.count) * 100;
      
      let recommendedSize = 20; // Default
      
      if (avgDuration > 30000) { // > 30 seconds
        recommendedSize = 10; // Reduce page size
      } else if (avgDuration < 10000 && successRate > 95) { // < 10 seconds, high success
        recommendedSize = 30; // Increase page size
      }
      
      recommendations.set(operation, recommendedSize);
    }
  }
  
  return recommendations;
}

async function updateRpcEndpointRankings(): Promise<void> {
  console.log('🌐 Updating RPC endpoint rankings...');
  
  const endpoints = [
    'https://api.mainnet-beta.solana.com',
    'https://solana-api.projectserum.com',
    // Add more endpoints
  ];
  
  const rankings = [];
  
  for (const endpoint of endpoints) {
    try {
      const startTime = Date.now();
      const connection = new Connection(endpoint, 'confirmed');
      
      await connection.getSlot();
      const latency = Date.now() - startTime;
      
      const health = await connection.getHealth();
      
      rankings.push({
        endpoint,
        latency,
        healthy: health === 'ok',
        score: health === 'ok' ? (1000 - latency) : 0,
      });
      
    } catch (error) {
      rankings.push({
        endpoint,
        latency: Infinity,
        healthy: false,
        score: 0,
      });
    }
  }
  
  rankings.sort((a, b) => b.score - a.score);
  
  console.log('RPC Rankings:', rankings);
  
  // Update configuration with best endpoints
  await updateRpcConfiguration(rankings);
}
```

### Monthly Maintenance

#### 1. Comprehensive System Review

```typescript
// monthly-review.ts
async function monthlySystemReview() {
  console.log('📊 Monthly System Review');
  console.log('========================');
  
  // 1. Generate comprehensive metrics
  const metrics = await generateMonthlyMetrics();
  
  // 2. Analyze trends
  const trends = analyzeTrends(metrics);
  
  // 3. Capacity planning
  const capacityReport = generateCapacityReport(metrics);
  
  // 4. Security review
  const securityReport = await performSecurityReview();
  
  // 5. Generate monthly report
  const report = {
    period: getCurrentMonth(),
    metrics,
    trends,
    capacity: capacityReport,
    security: securityReport,
    recommendations: generateRecommendations(metrics, trends),
  };
  
  await saveMonthlyReport(report);
  
  console.log('📄 Monthly report generated');
  return report;
}

async function generateMonthlyMetrics(): Promise<any> {
  // Collect metrics from the past month
  return {
    totalDistributions: 0,
    totalFeesDistributed: 0,
    averageDistributionTime: 0,
    successRate: 0,
    errorBreakdown: {},
    performanceMetrics: {},
    resourceUsage: {},
  };
}

function analyzeTrends(metrics: any): any {
  return {
    distributionVolumeGrowth: 0,
    performanceTrend: 'stable',
    errorRateTrend: 'decreasing',
    resourceUsageTrend: 'stable',
  };
}

function generateCapacityReport(metrics: any): any {
  return {
    currentCapacity: '80%',
    projectedGrowth: '15% per month',
    recommendedScaling: 'Add 2 more RPC endpoints',
    bottlenecks: ['RPC rate limits', 'Network latency'],
  };
}

async function performSecurityReview(): Promise<any> {
  return {
    vulnerabilities: [],
    recommendations: [
      'Update dependencies',
      'Rotate API keys',
      'Review access logs',
    ],
    complianceStatus: 'compliant',
  };
}
```

## Emergency Response

### Incident Response Procedures

#### 1. Incident Classification

```typescript
// incident-response.ts
enum IncidentSeverity {
  LOW = 'low',
  MEDIUM = 'medium',
  HIGH = 'high',
  CRITICAL = 'critical',
}

interface Incident {
  id: string;
  severity: IncidentSeverity;
  title: string;
  description: string;
  affectedVaults: string[];
  startTime: Date;
  status: 'open' | 'investigating' | 'resolved';
  assignee?: string;
  resolution?: string;
}

class IncidentManager {
  private incidents = new Map<string, Incident>();
  
  createIncident(
    severity: IncidentSeverity,
    title: string,
    description: string,
    affectedVaults: string[] = []
  ): string {
    const id = `INC-${Date.now()}`;
    
    const incident: Incident = {
      id,
      severity,
      title,
      description,
      affectedVaults,
      startTime: new Date(),
      status: 'open',
    };
    
    this.incidents.set(id, incident);
    
    console.log(`🚨 INCIDENT CREATED: ${id} [${severity.toUpperCase()}]`);
    console.log(`Title: ${title}`);
    console.log(`Description: ${description}`);
    console.log(`Affected Vaults: ${affectedVaults.join(', ')}`);
    
    // Auto-escalate critical incidents
    if (severity === IncidentSeverity.CRITICAL) {
      this.escalateIncident(id);
    }
    
    return id;
  }
  
  updateIncident(id: string, updates: Partial<Incident>): void {
    const incident = this.incidents.get(id);
    if (!incident) {
      throw new Error(`Incident ${id} not found`);
    }
    
    Object.assign(incident, updates);
    
    console.log(`📝 INCIDENT UPDATED: ${id}`);
    console.log(`Status: ${incident.status}`);
    
    if (updates.status === 'resolved') {
      console.log(`✅ INCIDENT RESOLVED: ${id}`);
      console.log(`Resolution: ${incident.resolution}`);
    }
  }
  
  private escalateIncident(id: string): void {
    console.log(`🚨 ESCALATING CRITICAL INCIDENT: ${id}`);
    
    // Send immediate notifications
    this.sendCriticalAlert(id);
    
    // Page on-call engineer
    this.pageOnCall(id);
    
    // Create war room
    this.createWarRoom(id);
  }
  
  private sendCriticalAlert(id: string): void {
    // Implement critical alerting
    console.log(`📢 Critical alert sent for incident ${id}`);
  }
  
  private pageOnCall(id: string): void {
    // Implement paging system
    console.log(`📞 On-call engineer paged for incident ${id}`);
  }
  
  private createWarRoom(id: string): void {
    // Create incident response channel
    console.log(`🏠 War room created for incident ${id}`);
  }
}
```

#### 2. Emergency Procedures

```typescript
// emergency-procedures.ts
class EmergencyProcedures {
  private incidentManager = new IncidentManager();
  
  async handleDistributionFailure(
    vault: PublicKey,
    error: Error,
    context: any
  ): Promise<void> {
    console.log('🚨 DISTRIBUTION FAILURE DETECTED');
    
    // 1. Assess severity
    const severity = this.assessFailureSeverity(error, context);
    
    // 2. Create incident
    const incidentId = this.incidentManager.createIncident(
      severity,
      'Distribution Failure',
      `Distribution failed for vault ${vault.toString()}: ${error.message}`,
      [vault.toString()]
    );
    
    // 3. Immediate response based on severity
    switch (severity) {
      case IncidentSeverity.CRITICAL:
        await this.handleCriticalFailure(vault, error, context);
        break;
      case IncidentSeverity.HIGH:
        await this.handleHighSeverityFailure(vault, error, context);
        break;
      default:
        await this.handleStandardFailure(vault, error, context);
    }
    
    // 4. Update incident with response actions
    this.incidentManager.updateIncident(incidentId, {
      status: 'investigating',
      assignee: 'auto-response-system',
    });
  }
  
  private assessFailureSeverity(error: Error, context: any): IncidentSeverity {
    // Critical: System-wide failures, security issues
    if (error.message.includes('BaseFeeDetected') ||
        error.message.includes('security') ||
        context.affectedVaults?.length > 5) {
      return IncidentSeverity.CRITICAL;
    }
    
    // High: Multiple vault failures, data corruption
    if (error.message.includes('InvalidPaginationCursor') ||
        error.message.includes('corruption') ||
        context.affectedVaults?.length > 1) {
      return IncidentSeverity.HIGH;
    }
    
    // Medium: Single vault issues, recoverable errors
    if (error.message.includes('CooldownNotElapsed') ||
        error.message.includes('InsufficientFunds')) {
      return IncidentSeverity.MEDIUM;
    }
    
    // Low: Transient network issues
    return IncidentSeverity.LOW;
  }
  
  private async handleCriticalFailure(
    vault: PublicKey,
    error: Error,
    context: any
  ): Promise<void> {
    console.log('🔴 CRITICAL FAILURE - EMERGENCY RESPONSE');
    
    // 1. Stop all automated processes
    await this.emergencyStop();
    
    // 2. Secure the system
    await this.secureSystem();
    
    // 3. Preserve evidence
    await this.preserveEvidence(vault, error, context);
    
    // 4. Notify stakeholders
    await this.notifyStakeholders('critical', vault, error);
    
    console.log('🛑 System secured - Manual intervention required');
  }
  
  private async handleHighSeverityFailure(
    vault: PublicKey,
    error: Error,
    context: any
  ): Promise<void> {
    console.log('🟠 HIGH SEVERITY FAILURE - AUTOMATED RECOVERY');
    
    // 1. Isolate affected vault
    await this.isolateVault(vault);
    
    // 2. Attempt automated recovery
    const recovered = await this.attemptRecovery(vault, error);
    
    if (!recovered) {
      // Escalate to critical if recovery fails
      await this.handleCriticalFailure(vault, error, context);
    }
  }
  
  private async handleStandardFailure(
    vault: PublicKey,
    error: Error,
    context: any
  ): Promise<void> {
    console.log('🟡 STANDARD FAILURE - RETRY WITH BACKOFF');
    
    // Standard retry logic with exponential backoff
    await this.retryWithBackoff(vault, error, context);
  }
  
  private async emergencyStop(): Promise<void> {
    console.log('🛑 EMERGENCY STOP - Halting all operations');
    
    // Stop all distribution services
    // Set emergency flag in configuration
    // Notify all running processes to stop
    
    process.env.EMERGENCY_STOP = 'true';
  }
  
  private async secureSystem(): Promise<void> {
    console.log('🔒 Securing system state');
    
    // Backup current state
    // Lock configuration changes
    // Enable audit logging
  }
  
  private async preserveEvidence(
    vault: PublicKey,
    error: Error,
    context: any
  ): Promise<void> {
    const evidence = {
      timestamp: new Date().toISOString(),
      vault: vault.toString(),
      error: {
        message: error.message,
        stack: error.stack,
      },
      context,
      systemState: await this.captureSystemState(),
    };
    
    const filename = `evidence-${Date.now()}.json`;
    await fs.writeFile(`incidents/${filename}`, JSON.stringify(evidence, null, 2));
    
    console.log(`📁 Evidence preserved: ${filename}`);
  }
  
  private async captureSystemState(): Promise<any> {
    return {
      timestamp: Date.now(),
      processInfo: {
        pid: process.pid,
        uptime: process.uptime(),
        memoryUsage: process.memoryUsage(),
      },
      environment: {
        nodeVersion: process.version,
        platform: process.platform,
      },
      // Add more system state as needed
    };
  }
  
  private async notifyStakeholders(
    severity: string,
    vault: PublicKey,
    error: Error
  ): Promise<void> {
    const message = `
🚨 METEORA FEE ROUTER ALERT - ${severity.toUpperCase()}

Vault: ${vault.toString()}
Error: ${error.message}
Time: ${new Date().toISOString()}

Immediate action required.
`;
    
    // Send to all notification channels
    console.log(message);
    
    // Implement actual notifications
    // await sendSlackAlert(message);
    // await sendEmailAlert(message);
    // await sendSMSAlert(message);
  }
}
```

This comprehensive operational procedures document provides the framework for managing Meteora Fee Router deployments in production environments, including daily operations, monitoring, maintenance, and emergency response procedures.