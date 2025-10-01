# Integration Examples

This document provides comprehensive examples for integrating with the Meteora Fee Router program.

## Table of Contents

- [Basic Setup](#basic-setup)
- [TypeScript Client](#typescript-client)
- [Rust Integration](#rust-integration)
- [CLI Tools](#cli-tools)
- [Monitoring and Automation](#monitoring-and-automation)
- [Error Handling Patterns](#error-handling-patterns)

## Basic Setup

### Environment Configuration

```typescript
// config.ts
export const CONFIG = {
  // Program IDs
  METEORA_FEE_ROUTER_PROGRAM_ID: "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
  DAMM_V2_PROGRAM_ID: "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
  STREAMFLOW_PROGRAM_ID: "strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m",
  
  // Common mints
  USDC_MINT: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  USDT_MINT: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
  
  // RPC endpoints
  RPC_MAINNET: "https://api.mainnet-beta.solana.com",
  RPC_DEVNET: "https://api.devnet.solana.com",
  
  // Default parameters
  DEFAULT_PAGE_SIZE: 20,
  DEFAULT_MIN_PAYOUT: 1000,
  DEFAULT_INVESTOR_SHARE_BPS: 7000, // 70%
};
```

### Utility Functions

```typescript
// utils.ts
import { PublicKey, Connection } from "@solana/web3.js";
import { getAssociatedTokenAddress } from "@solana/spl-token";

export class MeteoraUtils {
  static derivePolicyConfigPDA(programId: PublicKey, vault: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), vault.toBuffer()],
      programId
    );
  }

  static deriveDistributionProgressPDA(programId: PublicKey, vault: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("progress"), vault.toBuffer()],
      programId
    );
  }

  static derivePositionOwnerPDA(programId: PublicKey, vault: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), vault.toBuffer(), Buffer.from("investor_fee_pos_owner")],
      programId
    );
  }

  static async getTreasuryATA(
    quoteMint: PublicKey,
    positionOwnerPda: PublicKey
  ): Promise<PublicKey> {
    return getAssociatedTokenAddress(quoteMint, positionOwnerPda, true);
  }

  static async getCreatorATA(
    quoteMint: PublicKey,
    creatorWallet: PublicKey
  ): Promise<PublicKey> {
    return getAssociatedTokenAddress(quoteMint, creatorWallet);
  }

  static formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toISOString();
  }

  static formatTokenAmount(amount: string | number, decimals: number = 6): string {
    const num = typeof amount === 'string' ? parseFloat(amount) : amount;
    return (num / Math.pow(10, decimals)).toFixed(decimals);
  }
}
```

## TypeScript Client

### Complete Client Implementation

```typescript
// meteora-client.ts
import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  Keypair,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { Program, AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import { MeteoraFeeRouter } from "./target/types/meteora_fee_router";
import { CONFIG } from "./config";
import { MeteoraUtils } from "./utils";

export interface InitializeParams {
  vault: PublicKey;
  quoteMint: PublicKey;
  creatorWallet: PublicKey;
  investorFeeShareBps: number;
  dailyCapLamports?: number;
  minPayoutLamports: number;
  y0TotalAllocation: number;
  poolAccounts: {
    pool: PublicKey;
    poolConfig: PublicKey;
    quoteVault: PublicKey;
    baseVault: PublicKey;
  };
}

export interface DistributeParams {
  vault: PublicKey;
  quoteMint: PublicKey;
  creatorWallet: PublicKey;
  honoraryPosition: PublicKey;
  streamflowAccounts: PublicKey[];
  pageSize?: number;
  cursorPosition?: number;
}

export interface DistributionStatus {
  policy: any;
  progress: any;
  timing: {
    canStartNewDay: boolean;
    canContinueSameDay: boolean;
    timeUntilNext: number;
  };
  treasury?: {
    balance: number;
    mint: string;
  };
}

export class MeteoraFeeRouterClient {
  constructor(
    private program: Program<MeteoraFeeRouter>,
    private connection: Connection
  ) {}

  static async create(
    connection: Connection,
    wallet: Wallet,
    programId: PublicKey = new PublicKey(CONFIG.METEORA_FEE_ROUTER_PROGRAM_ID)
  ): Promise<MeteoraFeeRouterClient> {
    const provider = new AnchorProvider(connection, wallet, {
      commitment: 'confirmed',
      preflightCommitment: 'confirmed',
    });

    // Load IDL (you'll need to provide the actual IDL)
    const idl = await Program.fetchIdl(programId, provider);
    if (!idl) {
      throw new Error("Failed to fetch program IDL");
    }

    const program = new Program<MeteoraFeeRouter>(idl as any, programId, provider);
    return new MeteoraFeeRouterClient(program, connection);
  }

  async initializeHonoraryPosition(params: InitializeParams): Promise<{
    signature: string;
    policyConfig: PublicKey;
    distributionProgress: PublicKey;
    positionOwnerPda: PublicKey;
  }> {
    const [policyConfig] = MeteoraUtils.derivePolicyConfigPDA(
      this.program.programId,
      params.vault
    );
    const [distributionProgress] = MeteoraUtils.deriveDistributionProgressPDA(
      this.program.programId,
      params.vault
    );
    const [positionOwnerPda] = MeteoraUtils.derivePositionOwnerPDA(
      this.program.programId,
      params.vault
    );

    const tx = await this.program.methods
      .initializeHonoraryPosition({
        quoteMint: params.quoteMint,
        creatorWallet: params.creatorWallet,
        investorFeeShareBps: params.investorFeeShareBps,
        dailyCapLamports: params.dailyCapLamports ? new BN(params.dailyCapLamports) : null,
        minPayoutLamports: new BN(params.minPayoutLamports),
        y0TotalAllocation: new BN(params.y0TotalAllocation),
      })
      .accounts({
        payer: this.program.provider.publicKey!,
        policyConfig,
        distributionProgress,
        positionOwnerPda,
        vault: params.vault,
        pool: params.poolAccounts.pool,
        poolConfig: params.poolAccounts.poolConfig,
        quoteVault: params.poolAccounts.quoteVault,
        baseVault: params.poolAccounts.baseVault,
        cpAmmProgram: new PublicKey(CONFIG.DAMM_V2_PROGRAM_ID),
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    return {
      signature: tx,
      policyConfig,
      distributionProgress,
      positionOwnerPda,
    };
  }

  async distributeFees(params: DistributeParams): Promise<string> {
    const [policyConfig] = MeteoraUtils.derivePolicyConfigPDA(
      this.program.programId,
      params.vault
    );
    const [distributionProgress] = MeteoraUtils.deriveDistributionProgressPDA(
      this.program.programId,
      params.vault
    );
    const [positionOwnerPda] = MeteoraUtils.derivePositionOwnerPDA(
      this.program.programId,
      params.vault
    );

    const treasuryAta = await MeteoraUtils.getTreasuryATA(
      params.quoteMint,
      positionOwnerPda
    );
    const creatorAta = await MeteoraUtils.getCreatorATA(
      params.quoteMint,
      params.creatorWallet
    );

    // Check if ATAs exist and create if necessary
    const instructions: TransactionInstruction[] = [];
    
    const treasuryInfo = await this.connection.getAccountInfo(treasuryAta);
    if (!treasuryInfo) {
      instructions.push(
        createAssociatedTokenAccountInstruction(
          this.program.provider.publicKey!,
          treasuryAta,
          positionOwnerPda,
          params.quoteMint
        )
      );
    }

    const creatorInfo = await this.connection.getAccountInfo(creatorAta);
    if (!creatorInfo) {
      instructions.push(
        createAssociatedTokenAccountInstruction(
          this.program.provider.publicKey!,
          creatorAta,
          params.creatorWallet,
          params.quoteMint
        )
      );
    }

    // Build distribution instruction
    const distributionIx = await this.program.methods
      .distributeFees({
        pageSize: params.pageSize || CONFIG.DEFAULT_PAGE_SIZE,
        cursorPosition: params.cursorPosition || null,
      })
      .accounts({
        crankCaller: this.program.provider.publicKey!,
        policyConfig,
        distributionProgress,
        positionOwnerPda,
        vault: params.vault,
        honoraryPosition: params.honoraryPosition,
        treasuryAta,
        creatorAta,
        cpAmmProgram: new PublicKey(CONFIG.DAMM_V2_PROGRAM_ID),
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts(
        params.streamflowAccounts.map(account => ({
          pubkey: account,
          isWritable: false,
          isSigner: false,
        }))
      )
      .instruction();

    instructions.push(distributionIx);

    // Send transaction
    if (instructions.length === 1) {
      // Only distribution instruction
      return await this.program.methods
        .distributeFees({
          pageSize: params.pageSize || CONFIG.DEFAULT_PAGE_SIZE,
          cursorPosition: params.cursorPosition || null,
        })
        .accounts({
          crankCaller: this.program.provider.publicKey!,
          policyConfig,
          distributionProgress,
          positionOwnerPda,
          vault: params.vault,
          honoraryPosition: params.honoraryPosition,
          treasuryAta,
          creatorAta,
          cpAmmProgram: new PublicKey(CONFIG.DAMM_V2_PROGRAM_ID),
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(
          params.streamflowAccounts.map(account => ({
            pubkey: account,
            isWritable: false,
            isSigner: false,
          }))
        )
        .rpc();
    } else {
      // Multiple instructions including ATA creation
      const transaction = new Transaction().add(...instructions);
      const signature = await this.program.provider.sendAndConfirm!(transaction);
      return signature;
    }
  }

  async getDistributionStatus(vault: PublicKey): Promise<DistributionStatus> {
    const [policyConfig] = MeteoraUtils.derivePolicyConfigPDA(
      this.program.programId,
      vault
    );
    const [distributionProgress] = MeteoraUtils.deriveDistributionProgressPDA(
      this.program.programId,
      vault
    );
    const [positionOwnerPda] = MeteoraUtils.derivePositionOwnerPDA(
      this.program.programId,
      vault
    );

    const policy = await this.program.account.policyConfig.fetch(policyConfig);
    const progress = await this.program.account.distributionProgress.fetch(distributionProgress);

    const currentTime = Math.floor(Date.now() / 1000);
    const timeUntilNext = progress.lastDistributionTs.toNumber() + 86400 - currentTime;

    const status: DistributionStatus = {
      policy,
      progress,
      timing: {
        canStartNewDay: timeUntilNext <= 0,
        canContinueSameDay: timeUntilNext > 0 && !progress.dayComplete,
        timeUntilNext: Math.max(0, timeUntilNext),
      },
    };

    // Get treasury balance if possible
    try {
      const treasuryAta = await MeteoraUtils.getTreasuryATA(
        policy.quoteMint,
        positionOwnerPda
      );
      const balance = await this.connection.getTokenAccountBalance(treasuryAta);
      status.treasury = {
        balance: balance.value.uiAmount || 0,
        mint: policy.quoteMint.toString(),
      };
    } catch (error) {
      // Treasury ATA might not exist yet
    }

    return status;
  }

  async executeFullDistribution(
    vault: PublicKey,
    quoteMint: PublicKey,
    creatorWallet: PublicKey,
    honoraryPosition: PublicKey,
    streamflowAccounts: PublicKey[],
    options: {
      pageSize?: number;
      maxRetries?: number;
      delayBetweenPages?: number;
    } = {}
  ): Promise<string[]> {
    const {
      pageSize = CONFIG.DEFAULT_PAGE_SIZE,
      maxRetries = 3,
      delayBetweenPages = 1000,
    } = options;

    const status = await this.getDistributionStatus(vault);
    
    if (!status.timing.canStartNewDay && !status.timing.canContinueSameDay) {
      throw new Error(
        `Cannot distribute yet. Wait ${status.timing.timeUntilNext} seconds.`
      );
    }

    const signatures: string[] = [];
    let currentCursor = status.progress.paginationCursor;
    const totalInvestors = streamflowAccounts.length;

    console.log(`Starting distribution: cursor=${currentCursor}, total=${totalInvestors}`);

    while (currentCursor < totalInvestors) {
      const pageEnd = Math.min(currentCursor + pageSize, totalInvestors);
      const pageAccounts = streamflowAccounts.slice(currentCursor, pageEnd);

      console.log(`Processing page: ${currentCursor}-${pageEnd}/${totalInvestors}`);

      let attempt = 0;
      let success = false;

      while (attempt < maxRetries && !success) {
        try {
          const signature = await this.distributeFees({
            vault,
            quoteMint,
            creatorWallet,
            honoraryPosition,
            streamflowAccounts: pageAccounts,
            pageSize: pageAccounts.length,
            cursorPosition: attempt > 0 ? currentCursor : undefined, // Use explicit cursor for retries
          });

          signatures.push(signature);
          console.log(`Page processed successfully: ${signature}`);
          success = true;

        } catch (error) {
          attempt++;
          console.error(`Attempt ${attempt} failed:`, error);

          if (attempt < maxRetries) {
            const delay = 1000 * Math.pow(2, attempt); // Exponential backoff
            console.log(`Retrying in ${delay}ms...`);
            await new Promise(resolve => setTimeout(resolve, delay));
          } else {
            throw new Error(`Failed after ${maxRetries} attempts: ${error}`);
          }
        }
      }

      currentCursor = pageEnd;

      // Delay between pages to avoid rate limits
      if (currentCursor < totalInvestors && delayBetweenPages > 0) {
        await new Promise(resolve => setTimeout(resolve, delayBetweenPages));
      }
    }

    console.log(`Distribution completed successfully. Processed ${signatures.length} pages.`);
    return signatures;
  }

  // Event monitoring
  addEventListener(
    eventName: string,
    callback: (event: any) => void
  ): number {
    return this.program.addEventListener(eventName, callback);
  }

  removeEventListener(listenerId: number): Promise<void> {
    return this.program.removeEventListener(listenerId);
  }
}
```

## Rust Integration

### Account Validation Utilities

```rust
// validation.rs
use anchor_lang::prelude::*;
use meteora_fee_router::{
    state::{PolicyConfig, DistributionProgress},
    utils::pda::PdaUtils,
    error::ErrorCode,
};

pub struct AccountValidator;

impl AccountValidator {
    /// Validate all PDAs for a given vault
    pub fn validate_vault_pdas(
        program_id: &Pubkey,
        vault: &Pubkey,
        policy_config: &AccountInfo,
        distribution_progress: &AccountInfo,
        position_owner_pda: &AccountInfo,
    ) -> Result<()> {
        // Validate policy config PDA
        let (expected_policy, _) = PdaUtils::derive_policy_config_pda(program_id, vault);
        require_keys_eq!(expected_policy, policy_config.key());

        // Validate distribution progress PDA
        let (expected_progress, _) = PdaUtils::derive_distribution_progress_pda(program_id, vault);
        require_keys_eq!(expected_progress, distribution_progress.key());

        // Validate position owner PDA
        let (expected_owner, _) = PdaUtils::derive_position_owner_pda(program_id, vault);
        require_keys_eq!(expected_owner, position_owner_pda.key());

        Ok(())
    }

    /// Validate policy configuration parameters
    pub fn validate_policy_params(
        quote_mint: &Pubkey,
        creator_wallet: &Pubkey,
        investor_fee_share_bps: u16,
        daily_cap_lamports: Option<u64>,
        min_payout_lamports: u64,
        y0_total_allocation: u64,
    ) -> Result<()> {
        // Validate fee share basis points
        require!(
            investor_fee_share_bps <= 10000,
            ErrorCode::InvalidFeeShareBasisPoints
        );

        // Validate minimum payout
        require!(
            min_payout_lamports > 0,
            ErrorCode::InvalidMinPayoutThreshold
        );

        // Validate total allocation
        require!(
            y0_total_allocation > 0,
            ErrorCode::InvalidTotalAllocation
        );

        // Validate daily cap if provided
        if let Some(cap) = daily_cap_lamports {
            require!(cap > 0, ErrorCode::InvalidDailyCap);
        }

        Ok(())
    }

    /// Read and validate policy configuration
    pub fn read_policy_config(account_info: &AccountInfo) -> Result<PolicyConfig> {
        let policy_config = PolicyConfig::try_deserialize(&mut account_info.data.borrow().as_ref())?;
        policy_config.validate()?;
        Ok(policy_config)
    }

    /// Read and validate distribution progress
    pub fn read_distribution_progress(account_info: &AccountInfo) -> Result<DistributionProgress> {
        DistributionProgress::try_deserialize(&mut account_info.data.borrow().as_ref())
    }

    /// Validate timing for distribution operations
    pub fn validate_distribution_timing(
        progress: &DistributionProgress,
        current_timestamp: i64,
    ) -> Result<()> {
        let timing_state = progress.validate_distribution_timing(current_timestamp)?;
        msg!("Distribution timing validated: {:?}", timing_state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_params_validation() {
        let quote_mint = Pubkey::new_unique();
        let creator_wallet = Pubkey::new_unique();

        // Valid parameters
        assert!(AccountValidator::validate_policy_params(
            &quote_mint,
            &creator_wallet,
            7000,
            Some(1000000),
            1000,
            1000000,
        ).is_ok());

        // Invalid fee share (too high)
        assert!(AccountValidator::validate_policy_params(
            &quote_mint,
            &creator_wallet,
            10001,
            Some(1000000),
            1000,
            1000000,
        ).is_err());

        // Invalid minimum payout (zero)
        assert!(AccountValidator::validate_policy_params(
            &quote_mint,
            &creator_wallet,
            7000,
            Some(1000000),
            0,
            1000000,
        ).is_err());

        // Invalid total allocation (zero)
        assert!(AccountValidator::validate_policy_params(
            &quote_mint,
            &creator_wallet,
            7000,
            Some(1000000),
            1000,
            0,
        ).is_err());
    }
}
```

### Integration Helper

```rust
// integration_helper.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use meteora_fee_router::{
    state::{PolicyConfig, DistributionProgress},
    utils::pda::PdaUtils,
    instructions::{InitializeHonoraryPositionParams, DistributeFeesParams},
};

pub struct IntegrationHelper;

impl IntegrationHelper {
    /// Create initialization parameters with validation
    pub fn create_init_params(
        quote_mint: Pubkey,
        creator_wallet: Pubkey,
        investor_fee_share_bps: u16,
        daily_cap_lamports: Option<u64>,
        min_payout_lamports: u64,
        y0_total_allocation: u64,
    ) -> Result<InitializeHonoraryPositionParams> {
        // Validate parameters
        require!(
            investor_fee_share_bps <= 10000,
            "Invalid investor fee share basis points"
        );
        require!(min_payout_lamports > 0, "Invalid minimum payout threshold");
        require!(y0_total_allocation > 0, "Invalid total allocation");

        if let Some(cap) = daily_cap_lamports {
            require!(cap > 0, "Invalid daily cap");
        }

        Ok(InitializeHonoraryPositionParams {
            quote_mint,
            creator_wallet,
            investor_fee_share_bps,
            daily_cap_lamports,
            min_payout_lamports,
            y0_total_allocation,
        })
    }

    /// Create distribution parameters with validation
    pub fn create_distribute_params(
        page_size: u32,
        cursor_position: Option<u32>,
    ) -> Result<DistributeFeesParams> {
        require!(page_size > 0 && page_size <= 50, "Invalid page size");

        Ok(DistributeFeesParams {
            page_size,
            cursor_position,
        })
    }

    /// Calculate expected account sizes
    pub fn calculate_account_sizes() -> (usize, usize) {
        let policy_size = 8 + PolicyConfig::INIT_SPACE;
        let progress_size = 8 + DistributionProgress::INIT_SPACE;
        (policy_size, progress_size)
    }

    /// Estimate rent costs for accounts
    pub fn estimate_rent_costs(rent: &Rent) -> (u64, u64) {
        let (policy_size, progress_size) = Self::calculate_account_sizes();
        let policy_rent = rent.minimum_balance(policy_size);
        let progress_rent = rent.minimum_balance(progress_size);
        (policy_rent, progress_rent)
    }

    /// Derive all PDAs for a vault
    pub fn derive_all_pdas(
        program_id: &Pubkey,
        vault: &Pubkey,
    ) -> (Pubkey, u8, Pubkey, u8, Pubkey, u8) {
        let (policy_config, policy_bump) = PdaUtils::derive_policy_config_pda(program_id, vault);
        let (distribution_progress, progress_bump) = PdaUtils::derive_distribution_progress_pda(program_id, vault);
        let (position_owner_pda, owner_bump) = PdaUtils::derive_position_owner_pda(program_id, vault);

        (
            policy_config, policy_bump,
            distribution_progress, progress_bump,
            position_owner_pda, owner_bump,
        )
    }

    /// Create signer seeds for position owner PDA
    pub fn create_position_owner_seeds<'a>(
        vault: &'a Pubkey,
        bump: &'a [u8; 1],
    ) -> [&'a [u8]; 4] {
        PdaUtils::get_position_owner_signer_seeds(vault, bump)
    }
}
```

## CLI Tools

### Command Line Interface

```typescript
#!/usr/bin/env node
// cli.ts
import { Command } from 'commander';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { Wallet } from '@coral-xyz/anchor';
import { MeteoraFeeRouterClient } from './meteora-client';
import { CONFIG } from './config';
import * as fs from 'fs';

const program = new Command();

program
  .name('meteora-fee-router')
  .description('CLI for Meteora Fee Router operations')
  .version('1.0.0');

// Global options
program
  .option('-r, --rpc <url>', 'RPC endpoint', CONFIG.RPC_MAINNET)
  .option('-k, --keypair <path>', 'Keypair file path', '~/.config/solana/id.json')
  .option('-p, --program-id <pubkey>', 'Program ID', CONFIG.METEORA_FEE_ROUTER_PROGRAM_ID);

// Initialize command
program
  .command('initialize')
  .description('Initialize honorary position')
  .requiredOption('-v, --vault <pubkey>', 'Vault public key')
  .requiredOption('-q, --quote-mint <pubkey>', 'Quote mint public key')
  .requiredOption('-c, --creator <pubkey>', 'Creator wallet public key')
  .requiredOption('--pool <pubkey>', 'DAMM V2 pool public key')
  .requiredOption('--pool-config <pubkey>', 'DAMM V2 pool config public key')
  .requiredOption('--quote-vault <pubkey>', 'Pool quote vault public key')
  .requiredOption('--base-vault <pubkey>', 'Pool base vault public key')
  .option('-s, --investor-share <bps>', 'Investor fee share in basis points', '7000')
  .option('-d, --daily-cap <amount>', 'Daily distribution cap in lamports')
  .option('-m, --min-payout <amount>', 'Minimum payout threshold in lamports', '1000')
  .option('-y, --y0-allocation <amount>', 'Total investor allocation at TGE', '1000000')
  .action(async (options) => {
    try {
      const { client } = await setupClient(program.opts());
      
      const result = await client.initializeHonoraryPosition({
        vault: new PublicKey(options.vault),
        quoteMint: new PublicKey(options.quoteMint),
        creatorWallet: new PublicKey(options.creator),
        investorFeeShareBps: parseInt(options.investorShare),
        dailyCapLamports: options.dailyCap ? parseInt(options.dailyCap) : undefined,
        minPayoutLamports: parseInt(options.minPayout),
        y0TotalAllocation: parseInt(options.y0Allocation),
        poolAccounts: {
          pool: new PublicKey(options.pool),
          poolConfig: new PublicKey(options.poolConfig),
          quoteVault: new PublicKey(options.quoteVault),
          baseVault: new PublicKey(options.baseVault),
        },
      });

      console.log('✅ Honorary position initialized successfully!');
      console.log('Transaction:', result.signature);
      console.log('Policy Config:', result.policyConfig.toString());
      console.log('Distribution Progress:', result.distributionProgress.toString());
      console.log('Position Owner PDA:', result.positionOwnerPda.toString());

    } catch (error) {
      console.error('❌ Failed to initialize:', error);
      process.exit(1);
    }
  });

// Distribute command
program
  .command('distribute')
  .description('Execute distribution crank')
  .requiredOption('-v, --vault <pubkey>', 'Vault public key')
  .requiredOption('-q, --quote-mint <pubkey>', 'Quote mint public key')
  .requiredOption('-c, --creator <pubkey>', 'Creator wallet public key')
  .requiredOption('-p, --position <pubkey>', 'Honorary position public key')
  .requiredOption('-s, --streams <file>', 'JSON file with Streamflow account addresses')
  .option('--page-size <size>', 'Page size for pagination', '20')
  .option('--cursor <position>', 'Cursor position for retry')
  .option('--full', 'Execute full distribution cycle')
  .action(async (options) => {
    try {
      const { client } = await setupClient(program.opts());
      
      // Load Streamflow accounts
      const streamflowAccounts = JSON.parse(fs.readFileSync(options.streams, 'utf8'))
        .map((addr: string) => new PublicKey(addr));

      if (options.full) {
        // Execute full distribution
        const signatures = await client.executeFullDistribution(
          new PublicKey(options.vault),
          new PublicKey(options.quoteMint),
          new PublicKey(options.creator),
          new PublicKey(options.position),
          streamflowAccounts,
          {
            pageSize: parseInt(options.pageSize),
          }
        );

        console.log('✅ Full distribution completed!');
        console.log(`Processed ${signatures.length} pages`);
        signatures.forEach((sig, i) => {
          console.log(`Page ${i + 1}: ${sig}`);
        });

      } else {
        // Execute single page
        const signature = await client.distributeFees({
          vault: new PublicKey(options.vault),
          quoteMint: new PublicKey(options.quoteMint),
          creatorWallet: new PublicKey(options.creator),
          honoraryPosition: new PublicKey(options.position),
          streamflowAccounts,
          pageSize: parseInt(options.pageSize),
          cursorPosition: options.cursor ? parseInt(options.cursor) : undefined,
        });

        console.log('✅ Distribution page completed!');
        console.log('Transaction:', signature);
      }

    } catch (error) {
      console.error('❌ Failed to distribute:', error);
      process.exit(1);
    }
  });

// Status command
program
  .command('status')
  .description('Check distribution status')
  .requiredOption('-v, --vault <pubkey>', 'Vault public key')
  .action(async (options) => {
    try {
      const { client } = await setupClient(program.opts());
      
      const status = await client.getDistributionStatus(new PublicKey(options.vault));

      console.log('📊 Distribution Status');
      console.log('===================');
      console.log('Policy Configuration:');
      console.log(`  Vault: ${status.policy.vault}`);
      console.log(`  Quote Mint: ${status.policy.quoteMint}`);
      console.log(`  Creator: ${status.policy.creatorWallet}`);
      console.log(`  Investor Share: ${status.policy.investorFeeShareBps / 100}%`);
      console.log(`  Daily Cap: ${status.policy.dailyCapLamports?.toString() || 'None'}`);
      console.log(`  Min Payout: ${status.policy.minPayoutLamports}`);
      console.log(`  Y0 Allocation: ${status.policy.y0TotalAllocation}`);

      console.log('\nDistribution Progress:');
      console.log(`  Last Distribution: ${new Date(status.progress.lastDistributionTs * 1000).toISOString()}`);
      console.log(`  Current Day Distributed: ${status.progress.currentDayDistributed}`);
      console.log(`  Carry Over Dust: ${status.progress.carryOverDust}`);
      console.log(`  Pagination Cursor: ${status.progress.paginationCursor}`);
      console.log(`  Day Complete: ${status.progress.dayComplete}`);

      console.log('\nTiming:');
      console.log(`  Can Start New Day: ${status.timing.canStartNewDay}`);
      console.log(`  Can Continue Same Day: ${status.timing.canContinueSameDay}`);
      console.log(`  Time Until Next: ${status.timing.timeUntilNext}s`);

      if (status.treasury) {
        console.log('\nTreasury:');
        console.log(`  Balance: ${status.treasury.balance}`);
        console.log(`  Mint: ${status.treasury.mint}`);
      }

    } catch (error) {
      console.error('❌ Failed to get status:', error);
      process.exit(1);
    }
  });

// Monitor command
program
  .command('monitor')
  .description('Monitor distribution events')
  .requiredOption('-v, --vault <pubkey>', 'Vault public key')
  .action(async (options) => {
    try {
      const { client } = await setupClient(program.opts());
      
      console.log('🔍 Monitoring events for vault:', options.vault);
      console.log('Press Ctrl+C to stop...\n');

      // Set up event listeners
      const feeClaimedListener = client.addEventListener('quoteFeesClaimed', (event) => {
        console.log('💰 Fees Claimed:', {
          vault: event.vault.toString(),
          amount: event.claimedAmount.toString(),
          timestamp: new Date(event.timestamp * 1000).toISOString(),
        });
      });

      const payoutListener = client.addEventListener('investorPayoutPage', (event) => {
        console.log('👥 Investor Page Processed:', {
          vault: event.vault.toString(),
          pageStart: event.pageStart,
          pageEnd: event.pageEnd,
          distributed: event.totalDistributed.toString(),
          processed: event.processedCount,
        });
      });

      const creatorListener = client.addEventListener('creatorPayoutDayClosed', (event) => {
        console.log('🎯 Creator Payout (Day Complete):', {
          vault: event.vault.toString(),
          creatorPayout: event.creatorPayout.toString(),
          totalDistributed: event.totalDayDistributed.toString(),
          investorsProcessed: event.totalInvestorsProcessed,
        });
      });

      // Handle graceful shutdown
      process.on('SIGINT', async () => {
        console.log('\n🛑 Stopping monitor...');
        await client.removeEventListener(feeClaimedListener);
        await client.removeEventListener(payoutListener);
        await client.removeEventListener(creatorListener);
        process.exit(0);
      });

      // Keep the process running
      await new Promise(() => {}); // Run forever

    } catch (error) {
      console.error('❌ Failed to monitor:', error);
      process.exit(1);
    }
  });

// Helper function to setup client
async function setupClient(options: any) {
  const connection = new Connection(options.rpc, 'confirmed');
  
  // Load keypair
  const keypairPath = options.keypair.replace('~', process.env.HOME || '');
  const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf8'));
  const keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
  const wallet = new Wallet(keypair);

  const client = await MeteoraFeeRouterClient.create(
    connection,
    wallet,
    new PublicKey(options.programId)
  );

  return { client, connection, wallet, keypair };
}

program.parse();
```

### Batch Operations Script

```bash
#!/bin/bash
# batch-distribute.sh

set -e

VAULT=""
QUOTE_MINT=""
CREATOR=""
POSITION=""
STREAMS_FILE=""
PAGE_SIZE=20
RPC_URL="https://api.mainnet-beta.solana.com"
KEYPAIR="~/.config/solana/id.json"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -v|--vault)
      VAULT="$2"
      shift 2
      ;;
    -q|--quote-mint)
      QUOTE_MINT="$2"
      shift 2
      ;;
    -c|--creator)
      CREATOR="$2"
      shift 2
      ;;
    -p|--position)
      POSITION="$2"
      shift 2
      ;;
    -s|--streams)
      STREAMS_FILE="$2"
      shift 2
      ;;
    --page-size)
      PAGE_SIZE="$2"
      shift 2
      ;;
    --rpc)
      RPC_URL="$2"
      shift 2
      ;;
    --keypair)
      KEYPAIR="$2"
      shift 2
      ;;
    *)
      echo "Unknown option $1"
      exit 1
      ;;
  esac
done

# Validate required parameters
if [[ -z "$VAULT" || -z "$QUOTE_MINT" || -z "$CREATOR" || -z "$POSITION" || -z "$STREAMS_FILE" ]]; then
  echo "Error: Missing required parameters"
  echo "Usage: $0 -v VAULT -q QUOTE_MINT -c CREATOR -p POSITION -s STREAMS_FILE [options]"
  exit 1
fi

echo "🚀 Starting batch distribution..."
echo "Vault: $VAULT"
echo "Quote Mint: $QUOTE_MINT"
echo "Creator: $CREATOR"
echo "Position: $POSITION"
echo "Streams File: $STREAMS_FILE"
echo "Page Size: $PAGE_SIZE"
echo ""

# Check status first
echo "📊 Checking distribution status..."
node cli.js status -v "$VAULT" --rpc "$RPC_URL" --keypair "$KEYPAIR"

echo ""
read -p "Continue with distribution? (y/N): " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "❌ Distribution cancelled"
  exit 0
fi

# Execute full distribution
echo "🔄 Executing full distribution..."
node cli.js distribute \
  -v "$VAULT" \
  -q "$QUOTE_MINT" \
  -c "$CREATOR" \
  -p "$POSITION" \
  -s "$STREAMS_FILE" \
  --page-size "$PAGE_SIZE" \
  --full \
  --rpc "$RPC_URL" \
  --keypair "$KEYPAIR"

echo "✅ Batch distribution completed!"
```

## Monitoring and Automation

### Automated Distribution Service

```typescript
// distribution-service.ts
import { Connection, PublicKey } from '@solana/web3.js';
import { Wallet } from '@coral-xyz/anchor';
import { MeteoraFeeRouterClient } from './meteora-client';
import { CONFIG } from './config';
import * as cron from 'node-cron';

interface DistributionConfig {
  vault: PublicKey;
  quoteMint: PublicKey;
  creatorWallet: PublicKey;
  honoraryPosition: PublicKey;
  streamflowAccounts: PublicKey[];
  pageSize: number;
  enabled: boolean;
}

export class DistributionService {
  private client: MeteoraFeeRouterClient;
  private configs: Map<string, DistributionConfig> = new Map();
  private isRunning = false;

  constructor(
    private connection: Connection,
    private wallet: Wallet
  ) {}

  async initialize(): Promise<void> {
    this.client = await MeteoraFeeRouterClient.create(
      this.connection,
      this.wallet
    );

    // Set up event monitoring
    this.setupEventMonitoring();

    // Schedule distribution checks
    this.scheduleDistributions();

    console.log('✅ Distribution service initialized');
  }

  addDistributionConfig(id: string, config: DistributionConfig): void {
    this.configs.set(id, config);
    console.log(`📝 Added distribution config: ${id}`);
  }

  removeDistributionConfig(id: string): void {
    this.configs.delete(id);
    console.log(`🗑️ Removed distribution config: ${id}`);
  }

  private setupEventMonitoring(): void {
    // Monitor fee claiming events
    this.client.addEventListener('quoteFeesClaimed', (event) => {
      console.log('💰 Fees claimed:', {
        vault: event.vault.toString(),
        amount: event.claimedAmount.toString(),
        timestamp: new Date(event.timestamp * 1000).toISOString(),
      });

      // Send notification (implement your notification system)
      this.sendNotification('fee_claimed', {
        vault: event.vault.toString(),
        amount: event.claimedAmount.toString(),
      });
    });

    // Monitor investor payouts
    this.client.addEventListener('investorPayoutPage', (event) => {
      console.log('👥 Investor page processed:', {
        vault: event.vault.toString(),
        pageStart: event.pageStart,
        pageEnd: event.pageEnd,
        distributed: event.totalDistributed.toString(),
      });
    });

    // Monitor creator payouts
    this.client.addEventListener('creatorPayoutDayClosed', (event) => {
      console.log('🎯 Creator payout completed:', {
        vault: event.vault.toString(),
        creatorPayout: event.creatorPayout.toString(),
        totalDistributed: event.totalDayDistributed.toString(),
      });

      // Send notification for day completion
      this.sendNotification('day_completed', {
        vault: event.vault.toString(),
        creatorPayout: event.creatorPayout.toString(),
        totalDistributed: event.totalDayDistributed.toString(),
      });
    });
  }

  private scheduleDistributions(): void {
    // Check every hour for distributions that can be started
    cron.schedule('0 * * * *', async () => {
      if (this.isRunning) {
        console.log('⏳ Distribution already running, skipping...');
        return;
      }

      console.log('🔍 Checking for distributions to execute...');
      await this.checkAndExecuteDistributions();
    });

    console.log('⏰ Distribution scheduler started (hourly checks)');
  }

  private async checkAndExecuteDistributions(): Promise<void> {
    this.isRunning = true;

    try {
      for (const [id, config] of this.configs) {
        if (!config.enabled) {
          continue;
        }

        try {
          const status = await this.client.getDistributionStatus(config.vault);

          if (status.timing.canStartNewDay) {
            console.log(`🚀 Starting new distribution for ${id}...`);
            await this.executeDistribution(id, config);
          } else if (status.timing.canContinueSameDay) {
            console.log(`🔄 Continuing distribution for ${id}...`);
            await this.executeDistribution(id, config);
          } else {
            console.log(`⏱️ ${id}: Next distribution in ${status.timing.timeUntilNext}s`);
          }

        } catch (error) {
          console.error(`❌ Error checking ${id}:`, error);
          this.sendNotification('error', {
            configId: id,
            error: error.message,
          });
        }
      }

    } finally {
      this.isRunning = false;
    }
  }

  private async executeDistribution(id: string, config: DistributionConfig): Promise<void> {
    try {
      const signatures = await this.client.executeFullDistribution(
        config.vault,
        config.quoteMint,
        config.creatorWallet,
        config.honoraryPosition,
        config.streamflowAccounts,
        {
          pageSize: config.pageSize,
          maxRetries: 3,
          delayBetweenPages: 2000,
        }
      );

      console.log(`✅ Distribution completed for ${id}: ${signatures.length} pages`);

      this.sendNotification('distribution_completed', {
        configId: id,
        pagesProcessed: signatures.length,
        signatures,
      });

    } catch (error) {
      console.error(`❌ Distribution failed for ${id}:`, error);
      
      this.sendNotification('distribution_failed', {
        configId: id,
        error: error.message,
      });
    }
  }

  private sendNotification(type: string, data: any): void {
    // Implement your notification system here
    // Examples: Discord webhook, Slack, email, etc.
    console.log(`📢 Notification [${type}]:`, data);
  }

  async stop(): Promise<void> {
    console.log('🛑 Stopping distribution service...');
    // Clean up event listeners and scheduled tasks
    this.isRunning = false;
  }
}

// Usage example
async function main() {
  const connection = new Connection(CONFIG.RPC_MAINNET, 'confirmed');
  const wallet = new Wallet(/* your keypair */);

  const service = new DistributionService(connection, wallet);
  await service.initialize();

  // Add distribution configurations
  service.addDistributionConfig('vault1', {
    vault: new PublicKey('...'),
    quoteMint: new PublicKey(CONFIG.USDC_MINT),
    creatorWallet: new PublicKey('...'),
    honoraryPosition: new PublicKey('...'),
    streamflowAccounts: [
      new PublicKey('...'),
      // ... more accounts
    ],
    pageSize: 20,
    enabled: true,
  });

  // Handle graceful shutdown
  process.on('SIGINT', async () => {
    await service.stop();
    process.exit(0);
  });

  console.log('🎯 Distribution service running...');
}

if (require.main === module) {
  main().catch(console.error);
}
```

## Error Handling Patterns

### Comprehensive Error Handler

```typescript
// error-handler.ts
import { AnchorError } from '@coral-xyz/anchor';

export enum MeteoraErrorCode {
  InvalidQuoteMint = 6000,
  BaseFeeDetected = 6001,
  CooldownNotElapsed = 6002,
  DailyCapExceeded = 6003,
  PayoutBelowMinimum = 6004,
  InvalidPaginationCursor = 6005,
  ArithmeticOverflow = 6006,
  StreamflowValidationFailed = 6007,
  // ... add all error codes
}

export interface ErrorContext {
  operation: string;
  vault?: string;
  cursor?: number;
  pageSize?: number;
  timestamp?: number;
}

export class MeteoraErrorHandler {
  static handle(error: any, context: ErrorContext): {
    shouldRetry: boolean;
    retryDelay: number;
    userMessage: string;
    technicalMessage: string;
  } {
    console.error(`Error in ${context.operation}:`, error);

    // Handle Anchor program errors
    if (error instanceof AnchorError) {
      return this.handleAnchorError(error, context);
    }

    // Handle RPC errors
    if (error.message?.includes('Transaction simulation failed')) {
      return this.handleSimulationError(error, context);
    }

    // Handle network errors
    if (error.code === 'ECONNRESET' || error.code === 'ETIMEDOUT') {
      return {
        shouldRetry: true,
        retryDelay: 5000,
        userMessage: 'Network connection issue. Retrying...',
        technicalMessage: `Network error: ${error.message}`,
      };
    }

    // Default error handling
    return {
      shouldRetry: false,
      retryDelay: 0,
      userMessage: 'An unexpected error occurred. Please check the logs.',
      technicalMessage: error.message || 'Unknown error',
    };
  }

  private static handleAnchorError(error: AnchorError, context: ErrorContext) {
    const errorCode = error.error.errorCode.number;

    switch (errorCode) {
      case MeteoraErrorCode.CooldownNotElapsed:
        return {
          shouldRetry: false,
          retryDelay: 0,
          userMessage: 'Distribution cooldown period has not elapsed. Please wait 24 hours.',
          technicalMessage: `Cooldown not elapsed for vault ${context.vault}`,
        };

      case MeteoraErrorCode.InvalidPaginationCursor:
        return {
          shouldRetry: true,
          retryDelay: 1000,
          userMessage: 'Pagination cursor issue. Retrying with corrected cursor...',
          technicalMessage: `Invalid cursor ${context.cursor} for vault ${context.vault}`,
        };

      case MeteoraErrorCode.BaseFeeDetected:
        return {
          shouldRetry: false,
          retryDelay: 0,
          userMessage: 'Base fees detected. This position is not quote-only compliant.',
          technicalMessage: `Base fee detected for vault ${context.vault}`,
        };

      case MeteoraErrorCode.DailyCapExceeded:
        return {
          shouldRetry: false,
          retryDelay: 0,
          userMessage: 'Daily distribution cap has been reached.',
          technicalMessage: `Daily cap exceeded for vault ${context.vault}`,
        };

      case MeteoraErrorCode.StreamflowValidationFailed:
        return {
          shouldRetry: true,
          retryDelay: 2000,
          userMessage: 'Streamflow account validation failed. Retrying...',
          technicalMessage: `Streamflow validation failed for vault ${context.vault}`,
        };

      default:
        return {
          shouldRetry: false,
          retryDelay: 0,
          userMessage: `Program error: ${error.error.errorMessage}`,
          technicalMessage: `Anchor error ${errorCode}: ${error.error.errorMessage}`,
        };
    }
  }

  private static handleSimulationError(error: any, context: ErrorContext) {
    const logs = error.logs || [];
    
    // Check for specific error patterns in logs
    if (logs.some((log: string) => log.includes('insufficient funds'))) {
      return {
        shouldRetry: false,
        retryDelay: 0,
        userMessage: 'Insufficient funds for the operation.',
        technicalMessage: 'Insufficient funds detected in simulation',
      };
    }

    if (logs.some((log: string) => log.includes('already in use'))) {
      return {
        shouldRetry: true,
        retryDelay: 3000,
        userMessage: 'Account conflict detected. Retrying...',
        technicalMessage: 'Account already in use error',
      };
    }

    return {
      shouldRetry: true,
      retryDelay: 2000,
      userMessage: 'Transaction simulation failed. Retrying...',
      technicalMessage: `Simulation failed: ${error.message}`,
    };
  }
}

// Usage example
export async function executeWithErrorHandling<T>(
  operation: () => Promise<T>,
  context: ErrorContext,
  maxRetries: number = 3
): Promise<T> {
  let lastError: any;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      
      const errorInfo = MeteoraErrorHandler.handle(error, {
        ...context,
        timestamp: Date.now(),
      });

      console.log(`Attempt ${attempt}/${maxRetries} failed: ${errorInfo.userMessage}`);

      if (!errorInfo.shouldRetry || attempt === maxRetries) {
        throw new Error(`${errorInfo.userMessage} (${errorInfo.technicalMessage})`);
      }

      if (errorInfo.retryDelay > 0) {
        console.log(`Waiting ${errorInfo.retryDelay}ms before retry...`);
        await new Promise(resolve => setTimeout(resolve, errorInfo.retryDelay));
      }
    }
  }

  throw lastError;
}
```

This comprehensive integration guide provides everything needed to successfully integrate with the Meteora Fee Router program, including complete client implementations, CLI tools, monitoring systems, and robust error handling patterns.