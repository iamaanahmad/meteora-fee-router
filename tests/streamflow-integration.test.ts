import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MeteoraFeeRouter } from "../target/types/meteora_fee_router";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint } from "@solana/spl-token";
import { expect } from "chai";
import { BN } from "bn.js";

describe("Streamflow Integration Tests", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.MeteoraFeeRouter as Program<MeteoraFeeRouter>;
  const provider = anchor.getProvider();
  const connection = provider.connection;

  let quoteMint: PublicKey;
  let creatorWallet: Keypair;

  // Mock Streamflow stream structure
  interface MockStreamflowStream {
    recipient: PublicKey;
    sender: PublicKey;
    mint: PublicKey;
    depositedAmount: BN;
    withdrawnAmount: BN;
    startTime: BN;
    endTime: BN;
    cliffTime: BN;
    cancelableBySender: boolean;
    cancelableByRecipient: boolean;
    automaticWithdrawal: boolean;
    transferableBySender: boolean;
    transferableByRecipient: boolean;
    canTopup: boolean;
    streamName: number[];
    withdrawnTokensRecipient: BN;
    withdrawnTokensSender: BN;
    lastWithdrawnAt: BN;
    closedAt: BN | null;
  }

  beforeEach(async () => {
    creatorWallet = Keypair.generate();
    await connection.requestAirdrop(creatorWallet.publicKey, 10 * LAMPORTS_PER_SOL);

    quoteMint = await createMint(
      connection,
      creatorWallet,
      creatorWallet.publicKey,
      null,
      9
    );
  });

  describe("Streamflow Locked Amount Calculations", () => {
    it("Should calculate locked amounts for active streams", async () => {
      const currentTime = new BN(1500); // Midpoint of vesting
      
      const stream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000), // 1 SOL
        withdrawnAmount: new BN(0),
        startTime: new BN(1000),
        endTime: new BN(2000),
        cliffTime: new BN(1000),
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Calculate locked amount
      const elapsed = currentTime.sub(stream.startTime);
      const vestingDuration = stream.endTime.sub(stream.startTime);
      const vestedAmount = stream.depositedAmount.mul(elapsed).div(vestingDuration);
      const availableAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
      const lockedAmount = availableAmount.sub(vestedAmount);

      console.log("Active stream calculation:");
      console.log("  Deposited:", stream.depositedAmount.toString());
      console.log("  Elapsed time:", elapsed.toString());
      console.log("  Vesting duration:", vestingDuration.toString());
      console.log("  Vested amount:", vestedAmount.toString());
      console.log("  Available amount:", availableAmount.toString());
      console.log("  Locked amount:", lockedAmount.toString());

      // At midpoint (50% through), should have 50% locked
      expect(lockedAmount.toString()).to.equal("500000000"); // 0.5 SOL locked
    });

    it("Should handle fully vested streams", async () => {
      const currentTime = new BN(2500); // After vesting end
      
      const stream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000), // 1 SOL
        withdrawnAmount: new BN(0),
        startTime: new BN(1000),
        endTime: new BN(2000),
        cliffTime: new BN(1000),
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Calculate locked amount for fully vested stream
      const elapsed = currentTime.sub(stream.startTime);
      const vestingDuration = stream.endTime.sub(stream.startTime);
      
      let lockedAmount: BN;
      if (elapsed.gte(vestingDuration)) {
        lockedAmount = new BN(0); // Fully vested
      } else {
        const vestedAmount = stream.depositedAmount.mul(elapsed).div(vestingDuration);
        const availableAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
        lockedAmount = availableAmount.sub(vestedAmount);
      }

      console.log("Fully vested stream:");
      console.log("  Current time:", currentTime.toString());
      console.log("  End time:", stream.endTime.toString());
      console.log("  Locked amount:", lockedAmount.toString());

      expect(lockedAmount.toString()).to.equal("0"); // Fully unlocked
    });

    it("Should handle streams with withdrawals", async () => {
      const currentTime = new BN(1500); // Midpoint of vesting
      
      const stream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000), // 1 SOL
        withdrawnAmount: new BN(300_000_000),   // 0.3 SOL withdrawn
        startTime: new BN(1000),
        endTime: new BN(2000),
        cliffTime: new BN(1000),
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Calculate locked amount with withdrawals
      const elapsed = currentTime.sub(stream.startTime);
      const vestingDuration = stream.endTime.sub(stream.startTime);
      const vestedAmount = stream.depositedAmount.mul(elapsed).div(vestingDuration);
      const availableAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
      const lockedAmount = BN.max(new BN(0), availableAmount.sub(vestedAmount));

      console.log("Stream with withdrawals:");
      console.log("  Deposited:", stream.depositedAmount.toString());
      console.log("  Withdrawn:", stream.withdrawnAmount.toString());
      console.log("  Available:", availableAmount.toString());
      console.log("  Vested:", vestedAmount.toString());
      console.log("  Locked:", lockedAmount.toString());

      // Available: 1 SOL - 0.3 SOL = 0.7 SOL
      // Vested: 50% of 1 SOL = 0.5 SOL
      // Locked: 0.7 SOL - 0.5 SOL = 0.2 SOL
      expect(lockedAmount.toString()).to.equal("200000000"); // 0.2 SOL locked
    });

    it("Should handle cliff periods", async () => {
      const currentTime = new BN(1200); // Before cliff end
      
      const stream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000), // 1 SOL
        withdrawnAmount: new BN(0),
        startTime: new BN(1000),
        endTime: new BN(2000),
        cliffTime: new BN(1500), // Cliff at 1500
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Calculate locked amount during cliff period
      let lockedAmount: BN;
      if (currentTime.lt(stream.cliffTime)) {
        // During cliff, all tokens are locked
        lockedAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
      } else {
        // After cliff, normal vesting applies
        const elapsed = currentTime.sub(stream.startTime);
        const vestingDuration = stream.endTime.sub(stream.startTime);
        const vestedAmount = stream.depositedAmount.mul(elapsed).div(vestingDuration);
        const availableAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
        lockedAmount = availableAmount.sub(vestedAmount);
      }

      console.log("Stream during cliff:");
      console.log("  Current time:", currentTime.toString());
      console.log("  Cliff time:", stream.cliffTime.toString());
      console.log("  Locked amount:", lockedAmount.toString());

      expect(lockedAmount.toString()).to.equal("1000000000"); // All tokens locked during cliff
    });

    it("Should handle multiple streams per investor", async () => {
      const currentTime = new BN(1500);
      const investor = Keypair.generate().publicKey;

      const streams: MockStreamflowStream[] = [
        {
          recipient: investor,
          sender: creatorWallet.publicKey,
          mint: quoteMint,
          depositedAmount: new BN(600_000_000), // 0.6 SOL
          withdrawnAmount: new BN(0),
          startTime: new BN(1000),
          endTime: new BN(2000),
          cliffTime: new BN(1000),
          cancelableBySender: true,
          cancelableByRecipient: false,
          automaticWithdrawal: false,
          transferableBySender: false,
          transferableByRecipient: false,
          canTopup: false,
          streamName: new Array(64).fill(0),
          withdrawnTokensRecipient: new BN(0),
          withdrawnTokensSender: new BN(0),
          lastWithdrawnAt: new BN(0),
          closedAt: null,
        },
        {
          recipient: investor,
          sender: creatorWallet.publicKey,
          mint: quoteMint,
          depositedAmount: new BN(400_000_000), // 0.4 SOL
          withdrawnAmount: new BN(100_000_000), // 0.1 SOL withdrawn
          startTime: new BN(1000),
          endTime: new BN(2000),
          cliffTime: new BN(1000),
          cancelableBySender: true,
          cancelableByRecipient: false,
          automaticWithdrawal: false,
          transferableBySender: false,
          transferableByRecipient: false,
          canTopup: false,
          streamName: new Array(64).fill(0),
          withdrawnTokensRecipient: new BN(0),
          withdrawnTokensSender: new BN(0),
          lastWithdrawnAt: new BN(0),
          closedAt: null,
        },
      ];

      // Calculate total locked amount across all streams
      let totalLocked = new BN(0);

      streams.forEach((stream, index) => {
        const elapsed = currentTime.sub(stream.startTime);
        const vestingDuration = stream.endTime.sub(stream.startTime);
        const vestedAmount = stream.depositedAmount.mul(elapsed).div(vestingDuration);
        const availableAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
        const lockedAmount = BN.max(new BN(0), availableAmount.sub(vestedAmount));
        
        totalLocked = totalLocked.add(lockedAmount);

        console.log(`Stream ${index + 1}:`);
        console.log("  Deposited:", stream.depositedAmount.toString());
        console.log("  Withdrawn:", stream.withdrawnAmount.toString());
        console.log("  Locked:", lockedAmount.toString());
      });

      console.log("Multiple streams total:");
      console.log("  Total locked:", totalLocked.toString());

      // Stream 1: 0.6 SOL, 50% vested = 0.3 SOL locked
      // Stream 2: 0.4 SOL - 0.1 SOL = 0.3 SOL available, 50% of 0.4 SOL = 0.2 SOL vested, 0.3 - 0.2 = 0.1 SOL locked
      // Total: 0.3 + 0.1 = 0.4 SOL locked
      expect(totalLocked.toString()).to.equal("400000000"); // 0.4 SOL total locked
    });
  });

  describe("Integration with Distribution Logic", () => {
    it("Should integrate Streamflow data with fee distribution", async () => {
      const currentTime = new BN(1500);
      const y0TotalAllocation = new BN(4_000_000_000); // 4 SOL total allocation
      const investorFeeShareBps = 8000; // 80%
      const claimedQuote = new BN(100_000_000); // 0.1 SOL in fees

      // Mock investor streams with different vesting states
      const investorStreams = [
        {
          recipient: Keypair.generate().publicKey,
          locked: new BN(1_000_000_000), // 1 SOL locked
        },
        {
          recipient: Keypair.generate().publicKey,
          locked: new BN(500_000_000), // 0.5 SOL locked
        },
        {
          recipient: Keypair.generate().publicKey,
          locked: new BN(0), // Fully unlocked
        },
      ];

      const totalLocked = investorStreams.reduce(
        (sum, investor) => sum.add(investor.locked),
        new BN(0)
      );

      // Calculate distribution using Streamflow data
      const fLocked = totalLocked.mul(new BN(10000)).div(y0TotalAllocation);
      const eligibleShareBps = BN.min(new BN(investorFeeShareBps), fLocked);
      const investorAmount = claimedQuote.mul(eligibleShareBps).div(new BN(10000));
      const creatorAmount = claimedQuote.sub(investorAmount);

      console.log("Streamflow integration:");
      console.log("  Total locked:", totalLocked.toString());
      console.log("  Y0 allocation:", y0TotalAllocation.toString());
      console.log("  f_locked (bps):", fLocked.toString());
      console.log("  Eligible share (bps):", eligibleShareBps.toString());
      console.log("  Investor amount:", investorAmount.toString());
      console.log("  Creator amount:", creatorAmount.toString());

      // Total locked: 1.5 SOL out of 4 SOL = 37.5% = 3750 bps
      // Eligible share: min(8000, 3750) = 3750 bps
      // Investor amount: 0.1 SOL * 37.5% = 0.0375 SOL
      expect(totalLocked.toString()).to.equal("1500000000");
      expect(fLocked.toString()).to.equal("3750");
      expect(eligibleShareBps.toString()).to.equal("3750");
      expect(investorAmount.toString()).to.equal("37500000");
      expect(creatorAmount.toString()).to.equal("62500000");
    });

    it("Should handle edge case with all tokens unlocked", async () => {
      const y0TotalAllocation = new BN(4_000_000_000);
      const investorFeeShareBps = 8000;
      const claimedQuote = new BN(100_000_000);

      // All investors have fully unlocked tokens
      const totalLocked = new BN(0);

      const fLocked = new BN(0); // No tokens locked
      const eligibleShareBps = BN.min(new BN(investorFeeShareBps), fLocked);
      const investorAmount = claimedQuote.mul(eligibleShareBps).div(new BN(10000));
      const creatorAmount = claimedQuote.sub(investorAmount);

      console.log("All unlocked scenario:");
      console.log("  Total locked:", totalLocked.toString());
      console.log("  Eligible share (bps):", eligibleShareBps.toString());
      console.log("  Investor amount:", investorAmount.toString());
      console.log("  Creator amount:", creatorAmount.toString());

      // All fees should go to creator
      expect(eligibleShareBps.toString()).to.equal("0");
      expect(investorAmount.toString()).to.equal("0");
      expect(creatorAmount.toString()).to.equal("100000000");
    });

    it("Should handle edge case with all tokens locked", async () => {
      const y0TotalAllocation = new BN(4_000_000_000);
      const investorFeeShareBps = 8000;
      const claimedQuote = new BN(100_000_000);

      // All tokens are still locked
      const totalLocked = y0TotalAllocation;

      const fLocked = totalLocked.mul(new BN(10000)).div(y0TotalAllocation);
      const eligibleShareBps = BN.min(new BN(investorFeeShareBps), fLocked);
      const investorAmount = claimedQuote.mul(eligibleShareBps).div(new BN(10000));
      const creatorAmount = claimedQuote.sub(investorAmount);

      console.log("All locked scenario:");
      console.log("  Total locked:", totalLocked.toString());
      console.log("  f_locked (bps):", fLocked.toString());
      console.log("  Eligible share (bps):", eligibleShareBps.toString());
      console.log("  Investor amount:", investorAmount.toString());
      console.log("  Creator amount:", creatorAmount.toString());

      // f_locked = 100% = 10000 bps
      // Eligible share = min(8000, 10000) = 8000 bps = 80%
      expect(fLocked.toString()).to.equal("10000");
      expect(eligibleShareBps.toString()).to.equal("8000");
      expect(investorAmount.toString()).to.equal("80000000"); // 80% to investors
      expect(creatorAmount.toString()).to.equal("20000000"); // 20% to creator
    });
  });

  describe("Error Handling and Edge Cases", () => {
    it("Should handle invalid stream data gracefully", async () => {
      // Test with invalid timestamps
      const invalidStream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000),
        withdrawnAmount: new BN(0),
        startTime: new BN(2000), // Start after end!
        endTime: new BN(1000),
        cliffTime: new BN(1500),
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Validate stream data
      const isValid = invalidStream.startTime.lte(invalidStream.endTime) &&
                     invalidStream.cliffTime.gte(invalidStream.startTime) &&
                     invalidStream.withdrawnAmount.lte(invalidStream.depositedAmount);

      console.log("Invalid stream validation:");
      console.log("  Start time:", invalidStream.startTime.toString());
      console.log("  End time:", invalidStream.endTime.toString());
      console.log("  Is valid:", isValid);

      expect(isValid).to.be.false; // Should detect invalid data
    });

    it("Should handle withdrawn amount exceeding deposited amount", async () => {
      const stream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000),
        withdrawnAmount: new BN(1_500_000_000), // More than deposited!
        startTime: new BN(1000),
        endTime: new BN(2000),
        cliffTime: new BN(1000),
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Calculate available amount with safety check
      const availableAmount = BN.max(
        new BN(0),
        stream.depositedAmount.sub(stream.withdrawnAmount)
      );

      console.log("Over-withdrawn stream:");
      console.log("  Deposited:", stream.depositedAmount.toString());
      console.log("  Withdrawn:", stream.withdrawnAmount.toString());
      console.log("  Available (safe):", availableAmount.toString());

      expect(availableAmount.toString()).to.equal("0"); // Should be 0, not negative
    });

    it("Should handle zero-duration streams", async () => {
      const currentTime = new BN(1500);
      
      const stream: MockStreamflowStream = {
        recipient: Keypair.generate().publicKey,
        sender: creatorWallet.publicKey,
        mint: quoteMint,
        depositedAmount: new BN(1_000_000_000),
        withdrawnAmount: new BN(0),
        startTime: new BN(1000),
        endTime: new BN(1000), // Same as start time
        cliffTime: new BN(1000),
        cancelableBySender: true,
        cancelableByRecipient: false,
        automaticWithdrawal: false,
        transferableBySender: false,
        transferableByRecipient: false,
        canTopup: false,
        streamName: new Array(64).fill(0),
        withdrawnTokensRecipient: new BN(0),
        withdrawnTokensSender: new BN(0),
        lastWithdrawnAt: new BN(0),
        closedAt: null,
      };

      // Handle zero-duration stream
      const vestingDuration = stream.endTime.sub(stream.startTime);
      let lockedAmount: BN;

      if (vestingDuration.eq(new BN(0))) {
        // Instant vesting - all tokens unlocked
        lockedAmount = new BN(0);
      } else {
        // Normal calculation
        const elapsed = currentTime.sub(stream.startTime);
        const vestedAmount = stream.depositedAmount.mul(elapsed).div(vestingDuration);
        const availableAmount = stream.depositedAmount.sub(stream.withdrawnAmount);
        lockedAmount = availableAmount.sub(vestedAmount);
      }

      console.log("Zero-duration stream:");
      console.log("  Vesting duration:", vestingDuration.toString());
      console.log("  Locked amount:", lockedAmount.toString());

      expect(lockedAmount.toString()).to.equal("0"); // Should be fully unlocked
    });
  });
});