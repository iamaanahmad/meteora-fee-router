# Troubleshooting Guide

This guide helps diagnose and resolve common issues when working with the Meteora Fee Router program.

## Table of Contents

- [Common Error Codes](#common-error-codes)
- [Initialization Issues](#initialization-issues)
- [Distribution Problems](#distribution-problems)
- [Account Issues](#account-issues)
- [Timing and Pagination](#timing-and-pagination)
- [Performance Issues](#performance-issues)
- [Debugging Tools](#debugging-tools)
- [Recovery Procedures](#recovery-procedures)

## Common Error Codes

### Error 6000: InvalidQuoteMint

**Symptoms:**
- Initialization fails with "Invalid quote mint configuration"
- Pool validation errors during setup

**Causes:**
- Quote mint doesn't match pool configuration
- Incorrect token order in pool
- Wrong mint address provided

**Solutions:**
```typescript
// Verify quote mint matches pool configuration
const poolInfo = await connection.getAccountInfo(poolAddress);
// Check that quoteMint matches one of the pool's token mints

// Ensure correct token order
const quoteVault = await connection.getTokenAccountBalance(quoteVaultAddress);
console.log("Quote vault mint:", quoteVault.value.mint);
```

### Error 6001: BaseFeeDetected

**Symptoms:**
- Distribution fails with "Base fees detected"
- Quote-only enforcement failure

**Causes:**
- DAMM V2 position is accruing base token fees
- Incorrect tick range configuration
- Pool price movement causing base fee accrual

**Solutions:**
```typescript
// Check position configuration
const positionInfo = await dammProgram.account.position.fetch(positionAddress);
console.log("Position tick range:", {
  tickLowerIndex: positionInfo.tickLowerIndex,
  tickUpperIndex: positionInfo.tickUpperIndex,
});

// Verify current pool price
const poolInfo = await dammProgram.account.pool.fetch(poolAddress);
console.log("Current pool price:", poolInfo.sqrtPriceX64);

// Ensure position is configured for quote-only accrual
```

### Error 6002: CooldownNotElapsed

**Symptoms:**
- Distribution fails with "24-hour cooldown not elapsed"
- Cannot start new distribution cycle

**Causes:**
- Attempting distribution before 24 hours have passed
- Incorrect timestamp calculation
- Clock synchronization issues

**Solutions:**
```typescript
// Check current distribution status
const progress = await program.account.distributionProgress.fetch(progressPda);
const currentTime = Math.floor(Date.now() / 1000);
const timeUntilNext = progress.lastDistributionTs.toNumber() + 86400 - currentTime;

console.log("Time until next distribution:", timeUntilNext, "seconds");

if (timeUntilNext > 0) {
  console.log("Wait until:", new Date((currentTime + timeUntilNext) * 1000));
}
```

### Error 6005: InvalidPaginationCursor

**Symptoms:**
- Distribution fails with cursor validation error
- Pagination state inconsistency

**Causes:**
- Cursor position ahead of expected value
- Concurrent distribution attempts
- State corruption from failed transactions

**Solutions:**
```typescript
// Reset cursor to current position
const progress = await program.account.distributionProgress.fetch(progressPda);
console.log("Current cursor:", progress.paginationCursor);

// Use explicit cursor for retry
await client.distributeFees({
  // ... other params
  cursorPosition: progress.paginationCursor, // Use current cursor
});

// Or reset cursor if needed (requires admin action)
```

### Error 6007: StreamflowValidationFailed

**Symptoms:**
- Distribution fails during Streamflow account processing
- Invalid stream data errors

**Causes:**
- Streamflow account doesn't exist
- Stream is closed or expired
- Incorrect stream mint
- Corrupted stream data

**Solutions:**
```typescript
// Validate Streamflow accounts before distribution
async function validateStreamflowAccounts(
  streamAccounts: PublicKey[],
  expectedMint: PublicKey
) {
  for (const [index, streamAccount] of streamAccounts.entries()) {
    try {
      const accountInfo = await connection.getAccountInfo(streamAccount);
      if (!accountInfo) {
        console.error(`Stream ${index} does not exist:`, streamAccount.toString());
        continue;
      }

      // Parse stream data (implement based on Streamflow structure)
      const streamData = parseStreamflowData(accountInfo.data);
      
      if (!streamData.mint.equals(expectedMint)) {
        console.error(`Stream ${index} has wrong mint:`, {
          expected: expectedMint.toString(),
          actual: streamData.mint.toString(),
        });
      }

      if (streamData.end_time < Date.now() / 1000) {
        console.warn(`Stream ${index} is expired`);
      }

    } catch (error) {
      console.error(`Error validating stream ${index}:`, error);
    }
  }
}
```

## Initialization Issues

### Problem: Account Already Exists

**Symptoms:**
- Initialization fails with "account already exists"
- PDA collision errors

**Solutions:**
```typescript
// Check if accounts already exist
const [policyPda] = MeteoraUtils.derivePolicyConfigPDA(programId, vault);
const policyInfo = await connection.getAccountInfo(policyPda);

if (policyInfo) {
  console.log("Policy config already exists, skipping initialization");
  // Use existing accounts instead of initializing
} else {
  // Proceed with initialization
}
```

### Problem: Insufficient Funds for Rent

**Symptoms:**
- Initialization fails with insufficient funds
- Account creation errors

**Solutions:**
```typescript
// Calculate required rent
const rent = await connection.getMinimumBalanceForRentExemption(
  8 + PolicyConfig.INIT_SPACE
);
console.log("Required rent for policy config:", rent / 1e9, "SOL");

// Check payer balance
const payerBalance = await connection.getBalance(payer.publicKey);
console.log("Payer balance:", payerBalance / 1e9, "SOL");

if (payerBalance < rent * 2) { // Need rent for both accounts
  throw new Error("Insufficient SOL for account creation");
}
```

### Problem: Invalid Pool Configuration

**Symptoms:**
- Pool validation fails during initialization
- DAMM V2 integration errors

**Solutions:**
```typescript
// Validate pool accounts before initialization
async function validatePoolAccounts(poolAccounts: {
  pool: PublicKey;
  poolConfig: PublicKey;
  quoteVault: PublicKey;
  baseVault: PublicKey;
}) {
  // Check pool exists
  const poolInfo = await connection.getAccountInfo(poolAccounts.pool);
  if (!poolInfo) {
    throw new Error("Pool account does not exist");
  }

  // Check vault ownership
  const quoteVaultInfo = await connection.getTokenAccountBalance(poolAccounts.quoteVault);
  const baseVaultInfo = await connection.getTokenAccountBalance(poolAccounts.baseVault);
  
  console.log("Pool vaults:", {
    quote: {
      mint: quoteVaultInfo.value.mint,
      amount: quoteVaultInfo.value.uiAmount,
    },
    base: {
      mint: baseVaultInfo.value.mint,
      amount: baseVaultInfo.value.uiAmount,
    },
  });
}
```

## Distribution Problems

### Problem: No Fees to Distribute

**Symptoms:**
- Distribution completes but no tokens are transferred
- Zero fee amounts in events

**Causes:**
- No trading activity in the pool
- Position not accruing fees
- Fees already claimed

**Solutions:**
```typescript
// Check position fee status
async function checkPositionFees(positionAddress: PublicKey) {
  const positionInfo = await dammProgram.account.position.fetch(positionAddress);
  
  console.log("Position fees:", {
    feeOwedA: positionInfo.feeOwedA.toString(),
    feeOwedB: positionInfo.feeOwedB.toString(),
    liquidity: positionInfo.liquidity.toString(),
  });

  // Check if position is in range
  const poolInfo = await dammProgram.account.pool.fetch(positionInfo.pool);
  const inRange = positionInfo.tickLowerIndex <= poolInfo.tickCurrent &&
                  poolInfo.tickCurrent < positionInfo.tickUpperIndex;
  
  console.log("Position in range:", inRange);
}
```

### Problem: Partial Distribution Failure

**Symptoms:**
- Some investor pages process successfully
- Later pages fail with various errors
- Inconsistent distribution state

**Solutions:**
```typescript
// Implement robust pagination with recovery
async function distributeWithRecovery(
  client: MeteoraFeeRouterClient,
  params: DistributeParams,
  maxRetries: number = 3
) {
  const status = await client.getDistributionStatus(params.vault);
  let currentCursor = status.progress.paginationCursor;
  const totalInvestors = params.streamflowAccounts.length;

  while (currentCursor < totalInvestors) {
    const pageEnd = Math.min(currentCursor + params.pageSize!, totalInvestors);
    const pageAccounts = params.streamflowAccounts.slice(currentCursor, pageEnd);

    let success = false;
    let attempt = 0;

    while (!success && attempt < maxRetries) {
      try {
        await client.distributeFees({
          ...params,
          streamflowAccounts: pageAccounts,
          pageSize: pageAccounts.length,
          cursorPosition: currentCursor, // Explicit cursor for recovery
        });

        console.log(`✅ Page ${currentCursor}-${pageEnd} completed`);
        success = true;
        currentCursor = pageEnd;

      } catch (error) {
        attempt++;
        console.error(`❌ Page ${currentCursor}-${pageEnd} attempt ${attempt} failed:`, error);

        if (attempt < maxRetries) {
          // Check if cursor moved (partial success)
          const newStatus = await client.getDistributionStatus(params.vault);
          if (newStatus.progress.paginationCursor > currentCursor) {
            console.log("Cursor advanced, continuing from new position");
            currentCursor = newStatus.progress.paginationCursor;
            success = true; // Skip retry, cursor moved
          } else {
            // Wait before retry
            await new Promise(resolve => setTimeout(resolve, 2000 * attempt));
          }
        } else {
          throw new Error(`Failed after ${maxRetries} attempts: ${error}`);
        }
      }
    }
  }
}
```

## Account Issues

### Problem: Treasury ATA Not Found

**Symptoms:**
- Distribution fails with "Treasury ATA not found"
- Token account errors

**Solutions:**
```typescript
// Create treasury ATA if it doesn't exist
async function ensureTreasuryATA(
  connection: Connection,
  quoteMint: PublicKey,
  positionOwnerPda: PublicKey,
  payer: PublicKey
): Promise<PublicKey> {
  const treasuryAta = await getAssociatedTokenAddress(
    quoteMint,
    positionOwnerPda,
    true // Allow PDA owner
  );

  const accountInfo = await connection.getAccountInfo(treasuryAta);
  
  if (!accountInfo) {
    console.log("Creating treasury ATA:", treasuryAta.toString());
    
    const createIx = createAssociatedTokenAccountInstruction(
      payer,
      treasuryAta,
      positionOwnerPda,
      quoteMint
    );

    const transaction = new Transaction().add(createIx);
    await sendAndConfirmTransaction(connection, transaction, [/* payer keypair */]);
  }

  return treasuryAta;
}
```

### Problem: Creator ATA Issues

**Symptoms:**
- Creator payout fails
- ATA creation errors for creator

**Solutions:**
```typescript
// Validate and create creator ATA
async function ensureCreatorATA(
  connection: Connection,
  quoteMint: PublicKey,
  creatorWallet: PublicKey,
  payer: PublicKey
): Promise<PublicKey> {
  const creatorAta = await getAssociatedTokenAddress(quoteMint, creatorWallet);
  
  const accountInfo = await connection.getAccountInfo(creatorAta);
  
  if (!accountInfo) {
    console.log("Creating creator ATA:", creatorAta.toString());
    
    // Check if creator wallet exists
    const creatorInfo = await connection.getAccountInfo(creatorWallet);
    if (!creatorInfo) {
      throw new Error("Creator wallet does not exist");
    }

    const createIx = createAssociatedTokenAccountInstruction(
      payer,
      creatorAta,
      creatorWallet,
      quoteMint
    );

    const transaction = new Transaction().add(createIx);
    await sendAndConfirmTransaction(connection, transaction, [/* payer keypair */]);
  }

  return creatorAta;
}
```

## Timing and Pagination

### Problem: Clock Synchronization Issues

**Symptoms:**
- Timing validation fails unexpectedly
- Inconsistent cooldown behavior

**Solutions:**
```typescript
// Use on-chain clock for timing validation
async function getOnChainTime(connection: Connection): Promise<number> {
  const slot = await connection.getSlot();
  const blockTime = await connection.getBlockTime(slot);
  
  if (!blockTime) {
    throw new Error("Could not get block time");
  }
  
  return blockTime;
}

// Compare with local time
const localTime = Math.floor(Date.now() / 1000);
const onChainTime = await getOnChainTime(connection);
const timeDiff = Math.abs(localTime - onChainTime);

if (timeDiff > 60) { // More than 1 minute difference
  console.warn(`Clock drift detected: ${timeDiff} seconds`);
}
```

### Problem: Pagination State Corruption

**Symptoms:**
- Cursor position doesn't match expected value
- Duplicate or skipped investor processing

**Solutions:**
```typescript
// Validate pagination state before processing
async function validatePaginationState(
  program: Program,
  progressPda: PublicKey,
  expectedInvestorCount: number
) {
  const progress = await program.account.distributionProgress.fetch(progressPda);
  
  console.log("Pagination state:", {
    cursor: progress.paginationCursor,
    dayComplete: progress.dayComplete,
    expectedTotal: expectedInvestorCount,
  });

  // Check for invalid states
  if (progress.paginationCursor > expectedInvestorCount) {
    console.error("Cursor beyond expected range");
    // May need admin intervention to reset
  }

  if (progress.dayComplete && progress.paginationCursor < expectedInvestorCount) {
    console.error("Day marked complete but cursor not at end");
    // Inconsistent state detected
  }
}
```

## Performance Issues

### Problem: Transaction Timeouts

**Symptoms:**
- Transactions fail with timeout errors
- Slow confirmation times

**Solutions:**
```typescript
// Optimize transaction settings
const connection = new Connection(rpcUrl, {
  commitment: 'confirmed', // Faster than 'finalized'
  confirmTransactionInitialTimeout: 90000, // 90 seconds
  disableRetryOnRateLimit: false,
});

// Use priority fees for faster processing
const priorityFeeIx = ComputeBudgetProgram.setComputeUnitPrice({
  microLamports: 1000, // Adjust based on network conditions
});

const transaction = new Transaction()
  .add(priorityFeeIx)
  .add(distributionIx);
```

### Problem: Compute Budget Exceeded

**Symptoms:**
- Transactions fail with compute budget errors
- Large page sizes cause failures

**Solutions:**
```typescript
// Reduce page size for complex operations
const optimalPageSize = Math.min(pageSize, 15); // Reduce from default 20

// Add compute budget instruction
const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
  units: 400000, // Increase compute budget
});

const transaction = new Transaction()
  .add(computeBudgetIx)
  .add(distributionIx);
```

### Problem: Rate Limiting

**Symptoms:**
- RPC errors about rate limits
- 429 HTTP status codes

**Solutions:**
```typescript
// Implement exponential backoff
async function executeWithBackoff<T>(
  operation: () => Promise<T>,
  maxRetries: number = 5
): Promise<T> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      if (error.message?.includes('429') && attempt < maxRetries) {
        const delay = Math.min(1000 * Math.pow(2, attempt), 30000);
        console.log(`Rate limited, waiting ${delay}ms...`);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }
      throw error;
    }
  }
  throw new Error('Max retries exceeded');
}

// Use multiple RPC endpoints
const rpcEndpoints = [
  'https://api.mainnet-beta.solana.com',
  'https://solana-api.projectserum.com',
  // Add more endpoints
];

let currentEndpointIndex = 0;

function getConnection(): Connection {
  const endpoint = rpcEndpoints[currentEndpointIndex];
  currentEndpointIndex = (currentEndpointIndex + 1) % rpcEndpoints.length;
  return new Connection(endpoint, 'confirmed');
}
```

## Debugging Tools

### Transaction Analysis

```typescript
// Analyze failed transactions
async function analyzeFailedTransaction(
  connection: Connection,
  signature: string
) {
  try {
    const transaction = await connection.getTransaction(signature, {
      commitment: 'confirmed',
      maxSupportedTransactionVersion: 0,
    });

    if (!transaction) {
      console.log("Transaction not found");
      return;
    }

    console.log("Transaction analysis:", {
      slot: transaction.slot,
      blockTime: transaction.blockTime,
      fee: transaction.meta?.fee,
      computeUnitsConsumed: transaction.meta?.computeUnitsConsumed,
      err: transaction.meta?.err,
      logs: transaction.meta?.logMessages,
    });

    // Analyze logs for specific errors
    const logs = transaction.meta?.logMessages || [];
    const errorLogs = logs.filter(log => 
      log.includes('Error') || log.includes('failed') || log.includes('insufficient')
    );

    if (errorLogs.length > 0) {
      console.log("Error logs:", errorLogs);
    }

  } catch (error) {
    console.error("Failed to analyze transaction:", error);
  }
}
```

### Account State Inspection

```typescript
// Inspect account states for debugging
async function inspectAccountStates(
  program: Program,
  vault: PublicKey
) {
  const [policyPda] = MeteoraUtils.derivePolicyConfigPDA(program.programId, vault);
  const [progressPda] = MeteoraUtils.deriveDistributionProgressPDA(program.programId, vault);
  const [ownerPda] = MeteoraUtils.derivePositionOwnerPDA(program.programId, vault);

  try {
    const policy = await program.account.policyConfig.fetch(policyPda);
    console.log("Policy Config:", {
      vault: policy.vault.toString(),
      quoteMint: policy.quoteMint.toString(),
      creatorWallet: policy.creatorWallet.toString(),
      investorFeeShareBps: policy.investorFeeShareBps,
      dailyCapLamports: policy.dailyCapLamports?.toString(),
      minPayoutLamports: policy.minPayoutLamports.toString(),
      y0TotalAllocation: policy.y0TotalAllocation.toString(),
    });
  } catch (error) {
    console.error("Failed to fetch policy config:", error);
  }

  try {
    const progress = await program.account.distributionProgress.fetch(progressPda);
    console.log("Distribution Progress:", {
      vault: progress.vault.toString(),
      lastDistributionTs: progress.lastDistributionTs.toNumber(),
      currentDayDistributed: progress.currentDayDistributed.toString(),
      carryOverDust: progress.carryOverDust.toString(),
      paginationCursor: progress.paginationCursor,
      dayComplete: progress.dayComplete,
    });
  } catch (error) {
    console.error("Failed to fetch distribution progress:", error);
  }

  // Check PDA account info
  const ownerInfo = await program.provider.connection.getAccountInfo(ownerPda);
  console.log("Position Owner PDA:", {
    exists: !!ownerInfo,
    lamports: ownerInfo?.lamports,
    owner: ownerInfo?.owner.toString(),
  });
}
```

### Event Monitoring for Debugging

```typescript
// Set up comprehensive event monitoring
function setupDebugEventMonitoring(program: Program) {
  const listeners: number[] = [];

  // Monitor all events
  listeners.push(program.addEventListener('honoraryPositionInitialized', (event) => {
    console.log('🎯 Position Initialized:', event);
  }));

  listeners.push(program.addEventListener('quoteFeesClaimed', (event) => {
    console.log('💰 Fees Claimed:', event);
  }));

  listeners.push(program.addEventListener('investorPayoutPage', (event) => {
    console.log('👥 Investor Page:', event);
  }));

  listeners.push(program.addEventListener('creatorPayoutDayClosed', (event) => {
    console.log('🎯 Creator Payout:', event);
  }));

  // Return cleanup function
  return async () => {
    for (const listener of listeners) {
      await program.removeEventListener(listener);
    }
  };
}
```

## Recovery Procedures

### Reset Pagination State

```typescript
// Admin function to reset pagination (requires program upgrade or admin instruction)
async function resetPaginationState(
  program: Program,
  vault: PublicKey,
  newCursor: number = 0
) {
  // This would require an admin instruction in the program
  // For now, document the manual recovery process
  
  console.log("Manual recovery steps:");
  console.log("1. Wait for current day to complete (24+ hours)");
  console.log("2. Start new distribution cycle");
  console.log("3. Monitor cursor advancement");
  console.log("4. If issues persist, contact program administrators");
}
```

### Emergency Stop Procedures

```typescript
// Emergency procedures for critical issues
async function emergencyStopProcedures(vault: PublicKey) {
  console.log("🚨 EMERGENCY PROCEDURES 🚨");
  console.log("Vault:", vault.toString());
  console.log("");
  console.log("1. STOP all automated distribution processes");
  console.log("2. Document current state:");
  
  // Document current state
  await inspectAccountStates(program, vault);
  
  console.log("3. Contact program administrators with:");
  console.log("   - Vault address");
  console.log("   - Error details");
  console.log("   - Account state dump");
  console.log("   - Recent transaction signatures");
  
  console.log("4. DO NOT attempt manual fixes without admin approval");
}
```

### Data Recovery

```typescript
// Recover distribution data from events
async function recoverDistributionData(
  connection: Connection,
  programId: PublicKey,
  vault: PublicKey,
  fromSlot?: number
) {
  console.log("Recovering distribution data from events...");
  
  const signatures = await connection.getSignaturesForAddress(
    programId,
    { limit: 1000 }
  );

  const distributionEvents = [];

  for (const sig of signatures) {
    try {
      const tx = await connection.getTransaction(sig.signature, {
        commitment: 'confirmed',
        maxSupportedTransactionVersion: 0,
      });

      if (tx?.meta?.logMessages) {
        const logs = tx.meta.logMessages;
        
        // Parse events from logs (implement based on event structure)
        const events = parseEventsFromLogs(logs, vault);
        distributionEvents.push(...events);
      }
    } catch (error) {
      console.error(`Failed to process transaction ${sig.signature}:`, error);
    }
  }

  console.log(`Recovered ${distributionEvents.length} distribution events`);
  return distributionEvents;
}

function parseEventsFromLogs(logs: string[], vault: PublicKey): any[] {
  // Implement event parsing logic based on your event structure
  const events = [];
  
  for (const log of logs) {
    if (log.includes('Program data:')) {
      // Parse event data from log
      // This is a simplified example
      try {
        // Extract and decode event data
        const eventData = extractEventData(log);
        if (eventData && eventData.vault === vault.toString()) {
          events.push(eventData);
        }
      } catch (error) {
        // Skip invalid events
      }
    }
  }
  
  return events;
}

function extractEventData(log: string): any {
  // Implement actual event extraction logic
  return null;
}
```

This troubleshooting guide provides comprehensive solutions for the most common issues encountered when working with the Meteora Fee Router program. Use it as a reference when debugging problems and implementing recovery procedures.