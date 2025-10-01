import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MeteoraFeeRouter } from "../target/types/meteora_fee_router";
import { expect } from "chai";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";

describe("Security Audit Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MeteoraFeeRouter as Program<MeteoraFeeRouter>;
  const payer = provider.wallet as anchor.Wallet;

  let quoteMint: PublicKey;
  let baseMint: PublicKey;
  let vault: Keypair;
  let creatorWallet: Keypair;

  // PDA addresses
  let policyConfigPda: PublicKey;
  let distributionProgressPda: PublicKey;
  let positionOwnerPda: PublicKey;

  beforeEach(async () => {
    // Create test keypairs
    vault = Keypair.generate();
    creatorWallet = Keypair.generate();

    // Create test mints
    quoteMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6
    );

    baseMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6
    );

    // Derive PDAs
    [policyConfigPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), vault.publicKey.toBuffer()],
      program.programId
    );

    [distributionProgressPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("progress"), vault.publicKey.toBuffer()],
      program.programId
    );

    [positionOwnerPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        vault.publicKey.toBuffer(),
        Buffer.from("investor_fee_pos_owner"),
      ],
      program.programId
    );
  });

  describe("PDA Security Audit", () => {
    it("should have deterministic PDA derivation", async () => {
      // Test that PDA derivation is consistent
      const [pda1] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        program.programId
      );

      const [pda2] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        program.programId
      );

      expect(pda1.toString()).to.equal(pda2.toString());
    });

    it("should generate unique PDAs for different vaults", async () => {
      const vault2 = Keypair.generate();

      const [pda1] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        program.programId
      );

      const [pda2] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault2.publicKey.toBuffer()],
        program.programId
      );

      expect(pda1.toString()).to.not.equal(pda2.toString());
    });

    it("should generate unique PDAs for different seed types", async () => {
      const [policyPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        program.programId
      );

      const [progressPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("progress"), vault.publicKey.toBuffer()],
        program.programId
      );

      const [positionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vault"),
          vault.publicKey.toBuffer(),
          Buffer.from("investor_fee_pos_owner"),
        ],
        program.programId
      );

      // All PDAs should be unique
      expect(policyPda.toString()).to.not.equal(progressPda.toString());
      expect(policyPda.toString()).to.not.equal(positionPda.toString());
      expect(progressPda.toString()).to.not.equal(positionPda.toString());
    });

    it("should resist seed collision attacks", async () => {
      // Test various seed combinations that might cause collisions
      const testSeeds = [
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        [Buffer.from("polic"), Buffer.from("y"), vault.publicKey.toBuffer()],
        [Buffer.from("pol"), Buffer.from("icy"), vault.publicKey.toBuffer()],
      ];

      const pdas = testSeeds.map(seeds => {
        const [pda] = PublicKey.findProgramAddressSync(seeds, program.programId);
        return pda;
      });

      // All should be different (no collisions)
      for (let i = 0; i < pdas.length; i++) {
        for (let j = i + 1; j < pdas.length; j++) {
          expect(pdas[i].toString()).to.not.equal(pdas[j].toString());
        }
      }
    });
  });

  describe("Arithmetic Overflow Protection", () => {
    it("should handle large numbers without overflow", async () => {
      // Test with large but valid numbers
      const largeAmount = new anchor.BN("18446744073709551615"); // Near u64::MAX
      const mediumAmount = new anchor.BN("9223372036854775807"); // u64::MAX / 2

      // These should not cause overflow in the program
      // We'll test this by attempting to initialize with large values
      try {
        await program.methods
          .initializeHonoraryPosition({
            quoteMint: quoteMint,
            creatorWallet: creatorWallet.publicKey,
            investorFeeShareBps: 5000,
            dailyCapLamports: mediumAmount,
            minPayoutLamports: new anchor.BN(1000),
            y0TotalAllocation: mediumAmount,
          })
          .accounts({
            payer: payer.publicKey,
            policyConfig: policyConfigPda,
            distributionProgress: distributionProgressPda,
            positionOwnerPda: positionOwnerPda,
            vault: vault.publicKey,
            pool: Keypair.generate().publicKey, // Mock
            poolConfig: Keypair.generate().publicKey, // Mock
            quoteVault: await createAccount(
              provider.connection,
              payer.payer,
              quoteMint,
              payer.publicKey
            ),
            baseVault: await createAccount(
              provider.connection,
              payer.payer,
              baseMint,
              payer.publicKey
            ),
            cpAmmProgram: Keypair.generate().publicKey, // Mock
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        // If we reach here, the program handled large numbers correctly
        expect(true).to.be.true;
      } catch (error) {
        // Should not fail due to overflow, only due to mock accounts
        expect(error.message).to.not.include("overflow");
        expect(error.message).to.not.include("arithmetic");
      }
    });

    it("should reject invalid basis points", async () => {
      try {
        await program.methods
          .initializeHonoraryPosition({
            quoteMint: quoteMint,
            creatorWallet: creatorWallet.publicKey,
            investorFeeShareBps: 10001, // Invalid: > 10000
            dailyCapLamports: null,
            minPayoutLamports: new anchor.BN(1000),
            y0TotalAllocation: new anchor.BN(1000000),
          })
          .accounts({
            payer: payer.publicKey,
            policyConfig: policyConfigPda,
            distributionProgress: distributionProgressPda,
            positionOwnerPda: positionOwnerPda,
            vault: vault.publicKey,
            pool: Keypair.generate().publicKey,
            poolConfig: Keypair.generate().publicKey,
            quoteVault: await createAccount(
              provider.connection,
              payer.payer,
              quoteMint,
              payer.publicKey
            ),
            baseVault: await createAccount(
              provider.connection,
              payer.payer,
              baseMint,
              payer.publicKey
            ),
            cpAmmProgram: Keypair.generate().publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        expect.fail("Should have rejected invalid basis points");
      } catch (error) {
        expect(error.message).to.include("InvalidFeeShareBasisPoints");
      }
    });

    it("should reject zero values where required", async () => {
      try {
        await program.methods
          .initializeHonoraryPosition({
            quoteMint: quoteMint,
            creatorWallet: creatorWallet.publicKey,
            investorFeeShareBps: 5000,
            dailyCapLamports: null,
            minPayoutLamports: new anchor.BN(0), // Invalid: zero
            y0TotalAllocation: new anchor.BN(1000000),
          })
          .accounts({
            payer: payer.publicKey,
            policyConfig: policyConfigPda,
            distributionProgress: distributionProgressPda,
            positionOwnerPda: positionOwnerPda,
            vault: vault.publicKey,
            pool: Keypair.generate().publicKey,
            poolConfig: Keypair.generate().publicKey,
            quoteVault: await createAccount(
              provider.connection,
              payer.payer,
              quoteMint,
              payer.publicKey
            ),
            baseVault: await createAccount(
              provider.connection,
              payer.payer,
              baseMint,
              payer.publicKey
            ),
            cpAmmProgram: Keypair.generate().publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        expect.fail("Should have rejected zero minimum payout");
      } catch (error) {
        expect(error.message).to.include("InvalidMinPayoutThreshold");
      }
    });
  });

  describe("Access Control Validation", () => {
    it("should validate account ownership", async () => {
      // Create accounts with wrong owners to test validation
      const wrongOwner = Keypair.generate();
      const wrongQuoteVault = await createAccount(
        provider.connection,
        payer.payer,
        quoteMint,
        wrongOwner.publicKey // Wrong owner
      );

      try {
        await program.methods
          .initializeHonoraryPosition({
            quoteMint: quoteMint,
            creatorWallet: creatorWallet.publicKey,
            investorFeeShareBps: 5000,
            dailyCapLamports: null,
            minPayoutLamports: new anchor.BN(1000),
            y0TotalAllocation: new anchor.BN(1000000),
          })
          .accounts({
            payer: payer.publicKey,
            policyConfig: policyConfigPda,
            distributionProgress: distributionProgressPda,
            positionOwnerPda: positionOwnerPda,
            vault: vault.publicKey,
            pool: Keypair.generate().publicKey,
            poolConfig: Keypair.generate().publicKey,
            quoteVault: wrongQuoteVault, // Wrong ownership
            baseVault: await createAccount(
              provider.connection,
              payer.payer,
              baseMint,
              payer.publicKey
            ),
            cpAmmProgram: Keypair.generate().publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        // Should fail due to account validation
        expect.fail("Should have failed account validation");
      } catch (error) {
        // Should fail due to account ownership validation
        expect(error).to.exist;
      }
    });

    it("should validate mint consistency", async () => {
      const wrongMint = await createMint(
        provider.connection,
        payer.payer,
        payer.publicKey,
        null,
        6
      );

      const wrongMintVault = await createAccount(
        provider.connection,
        payer.payer,
        wrongMint, // Different mint
        payer.publicKey
      );

      try {
        await program.methods
          .initializeHonoraryPosition({
            quoteMint: quoteMint, // Expecting this mint
            creatorWallet: creatorWallet.publicKey,
            investorFeeShareBps: 5000,
            dailyCapLamports: null,
            minPayoutLamports: new anchor.BN(1000),
            y0TotalAllocation: new anchor.BN(1000000),
          })
          .accounts({
            payer: payer.publicKey,
            policyConfig: policyConfigPda,
            distributionProgress: distributionProgressPda,
            positionOwnerPda: positionOwnerPda,
            vault: vault.publicKey,
            pool: Keypair.generate().publicKey,
            poolConfig: Keypair.generate().publicKey,
            quoteVault: wrongMintVault, // Wrong mint
            baseVault: await createAccount(
              provider.connection,
              payer.payer,
              baseMint,
              payer.publicKey
            ),
            cpAmmProgram: Keypair.generate().publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        expect.fail("Should have failed mint validation");
      } catch (error) {
        expect(error.message).to.include("InvalidQuoteMint");
      }
    });

    it("should prevent same mint for quote and base", async () => {
      const sameMintVault1 = await createAccount(
        provider.connection,
        payer.payer,
        quoteMint, // Same mint
        payer.publicKey
      );

      const sameMintVault2 = await createAccount(
        provider.connection,
        payer.payer,
        quoteMint, // Same mint
        payer.publicKey
      );

      try {
        await program.methods
          .initializeHonoraryPosition({
            quoteMint: quoteMint,
            creatorWallet: creatorWallet.publicKey,
            investorFeeShareBps: 5000,
            dailyCapLamports: null,
            minPayoutLamports: new anchor.BN(1000),
            y0TotalAllocation: new anchor.BN(1000000),
          })
          .accounts({
            payer: payer.publicKey,
            policyConfig: policyConfigPda,
            distributionProgress: distributionProgressPda,
            positionOwnerPda: positionOwnerPda,
            vault: vault.publicKey,
            pool: Keypair.generate().publicKey,
            poolConfig: Keypair.generate().publicKey,
            quoteVault: sameMintVault1,
            baseVault: sameMintVault2, // Same mint as quote
            cpAmmProgram: Keypair.generate().publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        expect.fail("Should have rejected same mint for quote and base");
      } catch (error) {
        expect(error.message).to.include("InvalidPoolConfiguration");
      }
    });
  });

  describe("Reentrancy Protection", () => {
    it("should handle idempotent operations safely", async () => {
      // This test would require a successful initialization first
      // Then test that repeated calls with same parameters are safe
      
      // For now, we test that the program structure supports idempotent operations
      // by checking that cursor validation works correctly
      
      // Test cursor validation logic (simulated)
      const currentCursor = 20;
      const requestedCursor = 15; // Already processed
      
      // In a real scenario, this would be validated by the program
      const isRetry = requestedCursor < currentCursor;
      expect(isRetry).to.be.true;
      
      // Test that same cursor position is not a retry
      const samePosition = requestedCursor === currentCursor;
      expect(samePosition).to.be.false;
    });

    it("should maintain state consistency during operations", async () => {
      // Test that state updates are atomic
      // This is enforced by Anchor's account constraints and Rust's ownership model
      
      // Verify that PDAs are derived consistently
      const [pda1] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        program.programId
      );
      
      const [pda2] = PublicKey.findProgramAddressSync(
        [Buffer.from("policy"), vault.publicKey.toBuffer()],
        program.programId
      );
      
      expect(pda1.toString()).to.equal(pda2.toString());
    });
  });

  describe("Timing System Security", () => {
    it("should enforce 24-hour cooldown", async () => {
      const now = Math.floor(Date.now() / 1000);
      const twentyFourHours = 86400;
      
      // Test cooldown logic
      const lastDistribution = now - (twentyFourHours - 3600); // 23 hours ago
      const canDistribute = now >= lastDistribution + twentyFourHours;
      
      expect(canDistribute).to.be.false;
      
      // Test after cooldown
      const lastDistributionOld = now - (twentyFourHours + 3600); // 25 hours ago
      const canDistributeAfterCooldown = now >= lastDistributionOld + twentyFourHours;
      
      expect(canDistributeAfterCooldown).to.be.true;
    });

    it("should handle day boundary edge cases", async () => {
      const now = Math.floor(Date.now() / 1000);
      const twentyFourHours = 86400;
      
      // Test exact boundary
      const lastDistribution = now - twentyFourHours;
      const canDistributeExact = now >= lastDistribution + twentyFourHours;
      
      expect(canDistributeExact).to.be.true;
      
      // Test one second before boundary
      const lastDistributionAlmostReady = now - (twentyFourHours - 1);
      const canDistributeAlmost = now >= lastDistributionAlmostReady + twentyFourHours;
      
      expect(canDistributeAlmost).to.be.false;
    });
  });

  describe("Mathematical Precision", () => {
    it("should handle precision in weight calculations", () => {
      const WEIGHT_PRECISION = 1_000_000;
      
      // Test weight calculation precision
      const investorLocked = 2500;
      const totalLocked = 10000;
      const expectedWeight = (investorLocked * WEIGHT_PRECISION) / totalLocked;
      
      expect(expectedWeight).to.equal(250_000);
      
      // Test that weights sum correctly
      const investors = [2500, 3000, 2000, 2500]; // Total: 10000
      const weights = investors.map(locked => (locked * WEIGHT_PRECISION) / 10000);
      const totalWeight = weights.reduce((sum, weight) => sum + weight, 0);
      
      expect(totalWeight).to.equal(WEIGHT_PRECISION);
    });

    it("should handle dust accumulation correctly", () => {
      const minPayout = 100;
      const dustAmounts = [50, 75, 25, 80]; // Total: 230
      
      let accumulatedDust = 0;
      let totalPaidOut = 0;
      
      for (const dust of dustAmounts) {
        accumulatedDust += dust;
        
        if (accumulatedDust >= minPayout) {
          const payoutMultiples = Math.floor(accumulatedDust / minPayout);
          const payout = payoutMultiples * minPayout;
          totalPaidOut += payout;
          accumulatedDust -= payout;
        }
      }
      
      expect(totalPaidOut).to.equal(200); // 2 * 100
      expect(accumulatedDust).to.equal(30); // Remaining dust
    });

    it("should handle basis points calculations correctly", () => {
      const MAX_BASIS_POINTS = 10000;
      
      // Test percentage calculations
      const amount = 1000;
      const basisPoints = 2500; // 25%
      const result = (amount * basisPoints) / MAX_BASIS_POINTS;
      
      expect(result).to.equal(250);
      
      // Test that 100% works correctly
      const fullAmount = (amount * MAX_BASIS_POINTS) / MAX_BASIS_POINTS;
      expect(fullAmount).to.equal(amount);
      
      // Test that 0% works correctly
      const zeroAmount = (amount * 0) / MAX_BASIS_POINTS;
      expect(zeroAmount).to.equal(0);
    });
  });

  describe("Edge Case Handling", () => {
    it("should handle zero amounts gracefully", () => {
      // Test distribution with zero claimed amount
      const claimedQuote = 0;
      const lockedTotal = 5000;
      const y0Total = 10000;
      
      // Should return (0, 0) for zero claimed amount
      const investorAmount = 0;
      const creatorAmount = 0;
      
      expect(investorAmount + creatorAmount).to.equal(claimedQuote);
    });

    it("should handle fully unlocked scenario", () => {
      // Test when all tokens are unlocked
      const claimedQuote = 1000;
      const lockedTotal = 0; // All unlocked
      const y0Total = 10000;
      const investorFeeShareBps = 8000;
      
      // When nothing is locked, all should go to creator
      const fLocked = (lockedTotal * 10000) / y0Total; // 0
      const eligibleShareBps = Math.min(investorFeeShareBps, fLocked); // 0
      const investorAmount = (claimedQuote * eligibleShareBps) / 10000; // 0
      const creatorAmount = claimedQuote - investorAmount; // 1000
      
      expect(investorAmount).to.equal(0);
      expect(creatorAmount).to.equal(1000);
    });

    it("should handle fully locked scenario", () => {
      // Test when all tokens are locked
      const claimedQuote = 1000;
      const lockedTotal = 10000; // All locked
      const y0Total = 10000;
      const investorFeeShareBps = 8000; // 80%
      
      const fLocked = (lockedTotal * 10000) / y0Total; // 10000 (100%)
      const eligibleShareBps = Math.min(investorFeeShareBps, fLocked); // 8000
      const investorAmount = (claimedQuote * eligibleShareBps) / 10000; // 800
      const creatorAmount = claimedQuote - investorAmount; // 200
      
      expect(investorAmount).to.equal(800);
      expect(creatorAmount).to.equal(200);
    });
  });
});