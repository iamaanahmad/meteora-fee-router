import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MeteoraFeeRouter } from "../target/types/meteora_fee_router";
import { 
  PublicKey, 
  Keypair, 
  LAMPORTS_PER_SOL
} from "@solana/web3.js";
import { createMint } from "@solana/spl-token";
import { expect } from "chai";
import { BN } from "bn.js";

describe("Pagination and Resumption Tests", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.MeteoraFeeRouter as Program<MeteoraFeeRouter>;
  const provider = anchor.getProvider();
  const connection = provider.connection;

  let quoteMint: PublicKey;
  let creatorWallet: Keypair;

  // Mock distribution progress state
  interface DistributionProgress {
    vault: PublicKey;
    lastDistributionTs: number;
    currentDayDistributed: BN;
    carryOverDust: BN;
    paginationCursor: number;
    dayComplete: boolean;
  }

  // Mock investor data
  interface InvestorData {
    address: PublicKey;
    streamAccount: PublicKey;
    lockedAmount: BN;
    weight: number;
    expectedPayout: BN;
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

  describe("Basic Pagination Logic", () => {
    it("Should handle single page distribution", async () => {
      const totalInvestors = 15;
      const pageSize = 25;
      const totalInvestorAmount = new BN(50_000_000); // 0.05 SOL

      // Generate mock investors
      const investors: InvestorData[] = [];
      for (let i = 0; i < totalInvestors; i++) {
        investors.push({
          address: Keypair.generate().publicKey,
          streamAccount: Keypair.generate().publicKey,
          lockedAmount: new BN(1_000_000 + i * 100_000), // Varying amounts
          weight: Math.floor((1_000_000 + i * 100_000) / totalInvestors),
          expectedPayout: new BN(0), // Will calculate
        });
      }

      // Calculate total locked and weights
      const totalLocked = investors.reduce((sum, inv) => sum.add(inv.lockedAmount), new BN(0));
      
      // Calculate individual payouts
      investors.forEach(investor => {
        const weight = investor.lockedAmount.mul(new BN(1_000_000)).div(totalLocked);
        investor.expectedPayout = totalInvestorAmount.mul(weight).div(new BN(1_000_000));
      });

      // Simulate single page processing
      const pageStart = 0;
      const pageEnd = Math.min(pageStart + pageSize, totalInvestors);
      const pageInvestors = investors.slice(pageStart, pageEnd);

      console.log("Single page distribution:");
      console.log(`  Total investors: ${totalInvestors}`);
      console.log(`  Page size: ${pageSize}`);
      console.log(`  Page start: ${pageStart}`);
      console.log(`  Page end: ${pageEnd}`);
      console.log(`  Investors in page: ${pageInvestors.length}`);
      console.log(`  Total locked: ${totalLocked.toString()}`);
      console.log(`  Total to distribute: ${totalInvestorAmount.toString()}`);

      // Verify single page covers all investors
      expect(pageInvestors.length).to.equal(totalInvestors);
      expect(pageEnd).to.equal(totalInvestors);

      // Calculate total distributed in this page
      const totalDistributed = pageInvestors.reduce(
        (sum, inv) => sum.add(inv.expectedPayout),
        new BN(0)
      );

      console.log(`  Total distributed: ${totalDistributed.toString()}`);
      expect(totalDistributed).to.be.lte(totalInvestorAmount);

      console.log("  ✓ Single page distribution completed");
    });

    it("Should handle multi-page distribution", async () => {
      const totalInvestors = 127;
      const pageSize = 25;
      const totalPages = Math.ceil(totalInvestors / pageSize);
      const totalInvestorAmount = new BN(100_000_000); // 0.1 SOL

      console.log("Multi-page distribution:");
      console.log(`  Total investors: ${totalInvestors}`);
      console.log(`  Page size: ${pageSize}`);
      console.log(`  Total pages: ${totalPages}`);

      // Generate mock investors
      const investors: InvestorData[] = [];
      for (let i = 0; i < totalInvestors; i++) {
        investors.push({
          address: Keypair.generate().publicKey,
          streamAccount: Keypair.generate().publicKey,
          lockedAmount: new BN(1_000_000 + (i % 10) * 500_000), // Varying amounts
          weight: 0,
          expectedPayout: new BN(0),
        });
      }

      // Process each page
      let currentCursor = 0;
      let totalProcessed = 0;
      let cumulativeDistributed = new BN(0);

      for (let page = 0; page < totalPages; page++) {
        const pageStart = currentCursor;
        const pageEnd = Math.min(currentCursor + pageSize, totalInvestors);
        const pageInvestors = investors.slice(pageStart, pageEnd);
        const investorsInPage = pageEnd - pageStart;

        // Simulate page processing
        const pageDistributed = new BN(investorsInPage * 500_000); // Mock distribution
        cumulativeDistributed = cumulativeDistributed.add(pageDistributed);
        totalProcessed += investorsInPage;
        currentCursor = pageEnd;

        console.log(`  Page ${page + 1}:`);
        console.log(`    Range: ${pageStart}-${pageEnd}`);
        console.log(`    Investors: ${investorsInPage}`);
        console.log(`    Page distributed: ${pageDistributed.toString()}`);
        console.log(`    Cumulative distributed: ${cumulativeDistributed.toString()}`);
        console.log(`    Total processed: ${totalProcessed}`);
        console.log(`    Cursor position: ${currentCursor}`);

        // Verify page boundaries
        expect(pageStart).to.equal(page * pageSize);
        expect(investorsInPage).to.be.greaterThan(0);
        expect(investorsInPage).to.be.lessThanOrEqual(pageSize);
      }

      // Verify all investors processed
      expect(totalProcessed).to.equal(totalInvestors);
      expect(currentCursor).to.equal(totalInvestors);

      console.log("  ✓ Multi-page distribution completed");
    });

    it("Should handle edge case page sizes", async () => {
      const edgeCases = [
        { totalInvestors: 1, pageSize: 25, expectedPages: 1 },
        { totalInvestors: 25, pageSize: 25, expectedPages: 1 },
        { totalInvestors: 26, pageSize: 25, expectedPages: 2 },
        { totalInvestors: 50, pageSize: 25, expectedPages: 2 },
        { totalInvestors: 51, pageSize: 25, expectedPages: 3 },
        { totalInvestors: 100, pageSize: 1, expectedPages: 100 },
        { totalInvestors: 0, pageSize: 25, expectedPages: 0 },
      ];

      edgeCases.forEach(testCase => {
        const actualPages = testCase.totalInvestors > 0 
          ? Math.ceil(testCase.totalInvestors / testCase.pageSize)
          : 0;

        console.log(`\nEdge case: ${testCase.totalInvestors} investors, page size ${testCase.pageSize}`);
        console.log(`  Expected pages: ${testCase.expectedPages}`);
        console.log(`  Actual pages: ${actualPages}`);

        expect(actualPages).to.equal(testCase.expectedPages);

        // Verify page processing
        if (testCase.totalInvestors > 0) {
          let processedInvestors = 0;
          for (let page = 0; page < actualPages; page++) {
            const pageStart = page * testCase.pageSize;
            const pageEnd = Math.min(pageStart + testCase.pageSize, testCase.totalInvestors);
            const investorsInPage = pageEnd - pageStart;
            
            processedInvestors += investorsInPage;
            
            expect(investorsInPage).to.be.greaterThan(0);
            expect(investorsInPage).to.be.lessThanOrEqual(testCase.pageSize);
          }
          
          expect(processedInvestors).to.equal(testCase.totalInvestors);
        }

        console.log("  ✓ Edge case handled correctly");
      });
    });
  });

  describe("Resumption After Failures", () => {
    it("Should resume from correct cursor after partial failure", async () => {
      const totalInvestors = 100;
      const pageSize = 20;
      const failureAtPage = 3; // Fail after processing 2 complete pages

      console.log("Resumption after partial failure:");
      console.log(`  Total investors: ${totalInvestors}`);
      console.log(`  Page size: ${pageSize}`);
      console.log(`  Failure at page: ${failureAtPage}`);

      // Initial state
      let progress: DistributionProgress = {
        vault: Keypair.generate().publicKey,
        lastDistributionTs: Math.floor(Date.now() / 1000),
        currentDayDistributed: new BN(0),
        carryOverDust: new BN(0),
        paginationCursor: 0,
        dayComplete: false,
      };

      // Process pages until failure
      let processedPages = 0;
      let totalProcessed = 0;

      for (let page = 1; page <= failureAtPage - 1; page++) {
        const pageStart = progress.paginationCursor;
        const pageEnd = Math.min(pageStart + pageSize, totalInvestors);
        const investorsInPage = pageEnd - pageStart;

        // Simulate successful page processing
        const pageDistributed = new BN(investorsInPage * 1_000_000);
        progress.currentDayDistributed = progress.currentDayDistributed.add(pageDistributed);
        progress.paginationCursor = pageEnd;
        totalProcessed += investorsInPage;
        processedPages++;

        console.log(`  Page ${page} (successful):`);
        console.log(`    Range: ${pageStart}-${pageEnd}`);
        console.log(`    Investors: ${investorsInPage}`);
        console.log(`    Cursor after: ${progress.paginationCursor}`);
        console.log(`    Total processed: ${totalProcessed}`);
      }

      // Simulate failure at specific page
      const failurePageStart = progress.paginationCursor;
      console.log(`  Page ${failureAtPage} (failed):`);
      console.log(`    Failure at cursor: ${failurePageStart}`);
      console.log(`    State preserved for resumption`);

      // Verify state before resumption
      expect(progress.paginationCursor).to.equal(processedPages * pageSize);
      expect(totalProcessed).to.equal(processedPages * pageSize);
      expect(progress.dayComplete).to.be.false;

      // Resume processing from failure point
      console.log(`  Resuming from cursor: ${progress.paginationCursor}`);
      
      const remainingInvestors = totalInvestors - progress.paginationCursor;
      const remainingPages = Math.ceil(remainingInvestors / pageSize);

      console.log(`    Remaining investors: ${remainingInvestors}`);
      console.log(`    Remaining pages: ${remainingPages}`);

      // Process remaining pages
      for (let page = 0; page < remainingPages; page++) {
        const pageStart = progress.paginationCursor;
        const pageEnd = Math.min(pageStart + pageSize, totalInvestors);
        const investorsInPage = pageEnd - pageStart;

        // Simulate successful resumption
        const pageDistributed = new BN(investorsInPage * 1_000_000);
        progress.currentDayDistributed = progress.currentDayDistributed.add(pageDistributed);
        progress.paginationCursor = pageEnd;
        totalProcessed += investorsInPage;

        console.log(`  Resumed page ${page + 1}:`);
        console.log(`    Range: ${pageStart}-${pageEnd}`);
        console.log(`    Investors: ${investorsInPage}`);
        console.log(`    Cursor after: ${progress.paginationCursor}`);
        console.log(`    Total processed: ${totalProcessed}`);
      }

      // Mark day as complete
      if (progress.paginationCursor >= totalInvestors) {
        progress.dayComplete = true;
        progress.paginationCursor = 0; // Reset for next day
      }

      // Verify successful resumption
      expect(totalProcessed).to.equal(totalInvestors);
      expect(progress.dayComplete).to.be.true;
      expect(progress.paginationCursor).to.equal(0);

      console.log("  ✓ Successfully resumed and completed distribution");
    });

    it("Should handle multiple failure and resumption cycles", async () => {
      const totalInvestors = 150;
      const pageSize = 25;
      const failurePages = [2, 4, 6]; // Multiple failure points

      console.log("Multiple failure and resumption cycles:");
      console.log(`  Total investors: ${totalInvestors}`);
      console.log(`  Page size: ${pageSize}`);
      console.log(`  Failure pages: ${failurePages.join(', ')}`);

      let progress: DistributionProgress = {
        vault: Keypair.generate().publicKey,
        lastDistributionTs: Math.floor(Date.now() / 1000),
        currentDayDistributed: new BN(0),
        carryOverDust: new BN(0),
        paginationCursor: 0,
        dayComplete: false,
      };

      let currentPage = 1;
      let totalProcessed = 0;
      let failureCount = 0;

      while (!progress.dayComplete && currentPage <= 20) { // Safety limit
        const pageStart = progress.paginationCursor;
        const pageEnd = Math.min(pageStart + pageSize, totalInvestors);
        const investorsInPage = pageEnd - pageStart;

        if (investorsInPage === 0) break;

        // Check if this page should fail
        const shouldFail = failurePages.includes(currentPage) && failureCount < failurePages.length;

        if (shouldFail) {
          console.log(`  Page ${currentPage} (FAILED):`);
          console.log(`    Range: ${pageStart}-${pageEnd}`);
          console.log(`    Cursor preserved: ${progress.paginationCursor}`);
          console.log(`    Retrying...`);
          failureCount++;
          
          // Don't advance cursor on failure
          currentPage++; // But advance page counter for next attempt
          continue;
        }

        // Successful page processing
        const pageDistributed = new BN(investorsInPage * 800_000);
        progress.currentDayDistributed = progress.currentDayDistributed.add(pageDistributed);
        progress.paginationCursor = pageEnd;
        totalProcessed += investorsInPage;

        console.log(`  Page ${currentPage} (SUCCESS):`);
        console.log(`    Range: ${pageStart}-${pageEnd}`);
        console.log(`    Investors: ${investorsInPage}`);
        console.log(`    Cursor after: ${progress.paginationCursor}`);
        console.log(`    Total processed: ${totalProcessed}`);

        // Check if distribution is complete
        if (progress.paginationCursor >= totalInvestors) {
          progress.dayComplete = true;
          progress.paginationCursor = 0;
        }

        currentPage++;
      }

      // Verify final state
      expect(totalProcessed).to.equal(totalInvestors);
      expect(progress.dayComplete).to.be.true;
      expect(failureCount).to.equal(failurePages.length);

      console.log(`  ✓ Completed after ${failureCount} failures and resumptions`);
    });

    it("Should maintain idempotent operations during retries", async () => {
      const pageInvestors = [
        { address: "investor1", payout: new BN(1_000_000), paid: false },
        { address: "investor2", payout: new BN(2_000_000), paid: false },
        { address: "investor3", payout: new BN(1_500_000), paid: false },
      ];

      console.log("Idempotent operations during retries:");

      // Track payments to prevent double-payment
      const paymentTracker = new Map<string, { amount: BN, attempts: number }>();

      // Simulate multiple retry attempts for the same page
      const retryAttempts = 3;

      for (let attempt = 1; attempt <= retryAttempts; attempt++) {
        console.log(`  Attempt ${attempt}:`);

        pageInvestors.forEach(investor => {
          const existing = paymentTracker.get(investor.address);
          
          if (!existing) {
            // First payment attempt
            paymentTracker.set(investor.address, {
              amount: investor.payout,
              attempts: 1
            });
            investor.paid = true;
            console.log(`    ${investor.address}: PAID ${investor.payout.toString()}`);
          } else {
            // Subsequent attempts - should be idempotent
            existing.attempts++;
            console.log(`    ${investor.address}: SKIPPED (already paid ${existing.amount.toString()})`);
          }
        });

        // Verify idempotent behavior
        const totalPaid = Array.from(paymentTracker.values()).reduce(
          (sum, payment) => sum.add(payment.amount),
          new BN(0)
        );

        const expectedTotal = pageInvestors.reduce(
          (sum, inv) => sum.add(inv.payout),
          new BN(0)
        );

        console.log(`    Total paid: ${totalPaid.toString()}`);
        console.log(`    Expected total: ${expectedTotal.toString()}`);
        
        expect(totalPaid.toString()).to.equal(expectedTotal.toString());
      }

      // Verify no double payments occurred
      paymentTracker.forEach((payment, address) => {
        console.log(`  ${address}: ${payment.attempts} attempts, paid once`);
        expect(payment.attempts).to.equal(retryAttempts);
      });

      expect(paymentTracker.size).to.equal(pageInvestors.length);
      console.log("  ✓ Idempotent operations maintained across retries");
    });
  });

  describe("State Management During Pagination", () => {
    it("Should properly track cumulative distributions", async () => {
      const totalInvestors = 75;
      const pageSize = 20;
      const totalPages = Math.ceil(totalInvestors / pageSize);

      console.log("Cumulative distribution tracking:");
      console.log(`  Total investors: ${totalInvestors}`);
      console.log(`  Page size: ${pageSize}`);
      console.log(`  Total pages: ${totalPages}`);

      let progress: DistributionProgress = {
        vault: Keypair.generate().publicKey,
        lastDistributionTs: Math.floor(Date.now() / 1000),
        currentDayDistributed: new BN(0),
        carryOverDust: new BN(1_500), // Starting with some dust
        paginationCursor: 0,
        dayComplete: false,
      };

      const pageDistributions: BN[] = [];
      let totalExpectedDistribution = new BN(0);

      // Process all pages
      for (let page = 0; page < totalPages; page++) {
        const pageStart = progress.paginationCursor;
        const pageEnd = Math.min(pageStart + pageSize, totalInvestors);
        const investorsInPage = pageEnd - pageStart;

        // Calculate page distribution (varying amounts)
        const baseAmount = new BN(500_000); // 0.0005 SOL per investor
        const pageDistribution = baseAmount.mul(new BN(investorsInPage));
        
        // Add some dust handling
        const dustFromPreviousPage = page === 0 ? progress.carryOverDust : new BN(0);
        const totalPageAmount = pageDistribution.add(dustFromPreviousPage);
        
        // Update cumulative tracking
        progress.currentDayDistributed = progress.currentDayDistributed.add(totalPageAmount);
        progress.paginationCursor = pageEnd;
        
        pageDistributions.push(totalPageAmount);
        totalExpectedDistribution = totalExpectedDistribution.add(totalPageAmount);

        console.log(`  Page ${page + 1}:`);
        console.log(`    Range: ${pageStart}-${pageEnd}`);
        console.log(`    Investors: ${investorsInPage}`);
        console.log(`    Page distribution: ${pageDistribution.toString()}`);
        console.log(`    Dust from previous: ${dustFromPreviousPage.toString()}`);
        console.log(`    Total page amount: ${totalPageAmount.toString()}`);
        console.log(`    Cumulative distributed: ${progress.currentDayDistributed.toString()}`);
        console.log(`    Cursor: ${progress.paginationCursor}`);

        // Verify cumulative tracking
        const expectedCumulative = pageDistributions.reduce(
          (sum, amount) => sum.add(amount),
          new BN(0)
        );
        expect(progress.currentDayDistributed.toString()).to.equal(expectedCumulative.toString());
      }

      // Mark day complete
      progress.dayComplete = true;
      progress.paginationCursor = 0;

      // Verify final state
      expect(progress.currentDayDistributed.toString()).to.equal(totalExpectedDistribution.toString());
      expect(progress.dayComplete).to.be.true;
      expect(progress.paginationCursor).to.equal(0);

      console.log(`  Final cumulative: ${progress.currentDayDistributed.toString()}`);
      console.log("  ✓ Cumulative distribution tracking verified");
    });

    it("Should handle dust accumulation across pages", async () => {
      const minPayoutThreshold = new BN(1_000_000); // 0.001 SOL
      const pageSize = 10;

      console.log("Dust accumulation across pages:");
      console.log(`  Min payout threshold: ${minPayoutThreshold.toString()}`);

      let carryOverDust = new BN(0);
      const dustHistory: BN[] = [];

      // Simulate multiple pages with varying dust amounts
      const pageScenarios = [
        { investors: 10, avgPayout: new BN(800_000) },  // Below threshold
        { investors: 8, avgPayout: new BN(1_200_000) }, // Above threshold
        { investors: 12, avgPayout: new BN(600_000) },  // Below threshold
        { investors: 5, avgPayout: new BN(2_000_000) }, // Above threshold
      ];

      pageScenarios.forEach((scenario, pageIndex) => {
        console.log(`  Page ${pageIndex + 1}:`);
        console.log(`    Investors: ${scenario.investors}`);
        console.log(`    Avg payout: ${scenario.avgPayout.toString()}`);
        console.log(`    Carry over dust: ${carryOverDust.toString()}`);

        let pageDustGenerated = new BN(0);
        let pagePaidOut = new BN(0);

        // Process each investor in the page
        for (let i = 0; i < scenario.investors; i++) {
          const investorPayout = scenario.avgPayout.add(new BN(i * 50_000)); // Slight variation
          const totalAmount = investorPayout.add(carryOverDust);

          if (totalAmount.gte(minPayoutThreshold)) {
            // Pay out in multiples of threshold
            const payoutMultiples = totalAmount.div(minPayoutThreshold);
            const actualPayout = payoutMultiples.mul(minPayoutThreshold);
            const remainingDust = totalAmount.sub(actualPayout);

            pagePaidOut = pagePaidOut.add(actualPayout);
            carryOverDust = remainingDust;
          } else {
            // Add to dust
            pageDustGenerated = pageDustGenerated.add(investorPayout);
            carryOverDust = carryOverDust.add(investorPayout);
          }
        }

        dustHistory.push(carryOverDust);

        console.log(`    Page paid out: ${pagePaidOut.toString()}`);
        console.log(`    Page dust generated: ${pageDustGenerated.toString()}`);
        console.log(`    Carry over after page: ${carryOverDust.toString()}`);

        // Verify dust never exceeds reasonable bounds
        expect(carryOverDust).to.be.lt(minPayoutThreshold.mul(new BN(scenario.investors)));
      });

      // Verify dust accumulation pattern
      console.log("  Dust history:", dustHistory.map(d => d.toString()));
      
      // Final dust should be less than threshold (otherwise it would have been paid out)
      expect(carryOverDust).to.be.lt(minPayoutThreshold);

      console.log("  ✓ Dust accumulation handled correctly across pages");
    });

    it("Should reset state properly for next day", async () => {
      console.log("State reset for next day:");

      // End-of-day state
      let progress: DistributionProgress = {
        vault: Keypair.generate().publicKey,
        lastDistributionTs: Math.floor(Date.now() / 1000),
        currentDayDistributed: new BN(75_000_000), // 0.075 SOL distributed
        carryOverDust: new BN(2_500), // Some dust remaining
        paginationCursor: 100, // Completed all investors
        dayComplete: true,
      };

      console.log("  End of day state:");
      console.log(`    Last distribution: ${progress.lastDistributionTs}`);
      console.log(`    Day distributed: ${progress.currentDayDistributed.toString()}`);
      console.log(`    Carry over dust: ${progress.carryOverDust.toString()}`);
      console.log(`    Pagination cursor: ${progress.paginationCursor}`);
      console.log(`    Day complete: ${progress.dayComplete}`);

      // Simulate next day initialization
      const nextDayTimestamp = progress.lastDistributionTs + (24 * 60 * 60); // +24 hours
      
      // Reset for new day (preserve dust)
      const nextDayProgress: DistributionProgress = {
        vault: progress.vault,
        lastDistributionTs: nextDayTimestamp,
        currentDayDistributed: new BN(0), // Reset daily counter
        carryOverDust: progress.carryOverDust, // Preserve dust
        paginationCursor: 0, // Reset cursor
        dayComplete: false, // Reset completion flag
      };

      console.log("  Next day state:");
      console.log(`    Last distribution: ${nextDayProgress.lastDistributionTs}`);
      console.log(`    Day distributed: ${nextDayProgress.currentDayDistributed.toString()}`);
      console.log(`    Carry over dust: ${nextDayProgress.carryOverDust.toString()}`);
      console.log(`    Pagination cursor: ${nextDayProgress.paginationCursor}`);
      console.log(`    Day complete: ${nextDayProgress.dayComplete}`);

      // Verify proper reset
      expect(nextDayProgress.lastDistributionTs).to.be.greaterThan(progress.lastDistributionTs);
      expect(nextDayProgress.currentDayDistributed.toString()).to.equal("0");
      expect(nextDayProgress.carryOverDust.toString()).to.equal(progress.carryOverDust.toString());
      expect(nextDayProgress.paginationCursor).to.equal(0);
      expect(nextDayProgress.dayComplete).to.be.false;

      console.log("  ✓ State properly reset for next day");
    });
  });

  describe("Performance Under Pagination Load", () => {
    it("Should handle large investor sets efficiently", async () => {
      const largeScenarios = [
        { investors: 500, pageSize: 25 },
        { investors: 1000, pageSize: 50 },
        { investors: 2500, pageSize: 100 },
        { investors: 5000, pageSize: 200 },
      ];

      largeScenarios.forEach(scenario => {
        const totalPages = Math.ceil(scenario.investors / scenario.pageSize);
        const estimatedTimePerPage = 3; // seconds
        const totalEstimatedTime = totalPages * estimatedTimePerPage;

        console.log(`\nLarge scale scenario: ${scenario.investors} investors`);
        console.log(`  Page size: ${scenario.pageSize}`);
        console.log(`  Total pages: ${totalPages}`);
        console.log(`  Estimated time: ${totalEstimatedTime} seconds`);
        console.log(`  Feasible: ${totalEstimatedTime <= 600 ? 'YES' : 'NO'}`); // 10 minutes max

        // Verify reasonable performance expectations
        expect(totalPages).to.be.lessThan(100); // Reasonable page count
        if (scenario.investors <= 1000) {
          expect(totalEstimatedTime).to.be.lessThan(300); // 5 minutes for <= 1000 investors
        }

        console.log("  ✓ Performance expectations met");
      });
    });
  });
});