#!/usr/bin/env ts-node

/**
 * Comprehensive Test Suite Runner
 * 
 * This script runs all test suites for the Meteora Fee Router program
 * and provides a summary of test coverage and results.
 */

import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

interface TestSuite {
  name: string;
  file: string;
  description: string;
  requirements: string[];
}

const testSuites: TestSuite[] = [
  {
    name: "Initialize Honorary Position",
    file: "initialize-honorary-position.test.ts",
    description: "Tests for honorary position initialization with quote-only validation",
    requirements: ["1.1", "1.2", "1.3", "1.4", "1.5", "2.1", "2.2", "2.3", "8.1", "8.5"]
  },
  {
    name: "Fee Claiming",
    file: "fee-claiming.test.ts", 
    description: "Tests for DAMM V2 fee claiming and quote-only enforcement",
    requirements: ["3.3", "3.4", "2.4", "9.1"]
  },
  {
    name: "Comprehensive Integration",
    file: "comprehensive-integration.test.ts",
    description: "End-to-end integration tests covering full distribution cycles",
    requirements: ["9.1", "9.2", "9.3", "9.4", "9.5"]
  },
  {
    name: "Streamflow Integration", 
    file: "streamflow-integration.test.ts",
    description: "Tests for Streamflow vesting schedule integration and locked amount calculations",
    requirements: ["4.1", "8.3", "9.2"]
  },
  {
    name: "Performance and Compute",
    file: "performance-compute.test.ts",
    description: "Performance tests and compute budget optimization analysis",
    requirements: ["9.5", "8.5"]
  },
  {
    name: "Failure and Edge Cases",
    file: "failure-edge-cases.test.ts", 
    description: "Comprehensive failure case testing and edge case validation",
    requirements: ["9.1", "9.3", "2.4"]
  },
  {
    name: "Pagination and Resumption",
    file: "pagination-resumption.test.ts",
    description: "Tests for pagination logic and resumable operations after failures", 
    requirements: ["6.1", "6.2", "6.3", "6.4", "9.4"]
  }
];

interface TestResult {
  suite: string;
  passed: boolean;
  duration: number;
  error?: string;
}

class TestRunner {
  private results: TestResult[] = [];
  private startTime: number = 0;

  async runAllTests(): Promise<void> {
    console.log("🚀 Starting Comprehensive Test Suite for Meteora Fee Router");
    console.log("=" .repeat(80));
    
    this.startTime = Date.now();
    
    // Print test plan
    this.printTestPlan();
    
    // Run each test suite
    for (const suite of testSuites) {
      await this.runTestSuite(suite);
    }
    
    // Print summary
    this.printSummary();
  }

  private printTestPlan(): void {
    console.log("\n📋 Test Plan:");
    console.log("-".repeat(80));
    
    testSuites.forEach((suite, index) => {
      console.log(`${index + 1}. ${suite.name}`);
      console.log(`   File: ${suite.file}`);
      console.log(`   Description: ${suite.description}`);
      console.log(`   Requirements: ${suite.requirements.join(', ')}`);
      console.log();
    });
  }

  private async runTestSuite(suite: TestSuite): Promise<void> {
    console.log(`\n🧪 Running: ${suite.name}`);
    console.log("-".repeat(50));
    
    const suiteStartTime = Date.now();
    
    try {
      // Check if test file exists
      const testPath = path.join(__dirname, suite.file);
      if (!fs.existsSync(testPath)) {
        throw new Error(`Test file not found: ${suite.file}`);
      }

      // Run the test suite
      const command = `npm run test -- --run ${suite.file}`;
      console.log(`Executing: ${command}`);
      
      execSync(command, { 
        stdio: 'inherit',
        cwd: path.join(__dirname, '..'),
        timeout: 300000 // 5 minute timeout
      });
      
      const duration = Date.now() - suiteStartTime;
      this.results.push({
        suite: suite.name,
        passed: true,
        duration
      });
      
      console.log(`✅ ${suite.name} - PASSED (${duration}ms)`);
      
    } catch (error) {
      const duration = Date.now() - suiteStartTime;
      this.results.push({
        suite: suite.name,
        passed: false,
        duration,
        error: error instanceof Error ? error.message : String(error)
      });
      
      console.log(`❌ ${suite.name} - FAILED (${duration}ms)`);
      console.log(`Error: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  private printSummary(): void {
    const totalDuration = Date.now() - this.startTime;
    const passedTests = this.results.filter(r => r.passed).length;
    const failedTests = this.results.filter(r => !r.passed).length;
    const totalTests = this.results.length;
    
    console.log("\n" + "=".repeat(80));
    console.log("📊 TEST SUMMARY");
    console.log("=".repeat(80));
    
    console.log(`\nOverall Results:`);
    console.log(`  Total Test Suites: ${totalTests}`);
    console.log(`  Passed: ${passedTests} ✅`);
    console.log(`  Failed: ${failedTests} ❌`);
    console.log(`  Success Rate: ${((passedTests / totalTests) * 100).toFixed(1)}%`);
    console.log(`  Total Duration: ${(totalDuration / 1000).toFixed(2)}s`);
    
    console.log(`\nDetailed Results:`);
    this.results.forEach(result => {
      const status = result.passed ? "✅ PASS" : "❌ FAIL";
      const duration = `${(result.duration / 1000).toFixed(2)}s`;
      console.log(`  ${status} ${result.suite.padEnd(30)} ${duration}`);
      
      if (!result.passed && result.error) {
        console.log(`       Error: ${result.error}`);
      }
    });
    
    // Requirements coverage analysis
    this.printRequirementsCoverage();
    
    // Performance analysis
    this.printPerformanceAnalysis();
    
    // Final verdict
    console.log(`\n${"=".repeat(80)}`);
    if (failedTests === 0) {
      console.log("🎉 ALL TESTS PASSED! The Meteora Fee Router is ready for deployment.");
    } else {
      console.log(`⚠️  ${failedTests} test suite(s) failed. Please review and fix issues before deployment.`);
    }
    console.log(`${"=".repeat(80)}\n`);
  }

  private printRequirementsCoverage(): void {
    console.log(`\n📋 Requirements Coverage:`);
    
    // Collect all requirements from test suites
    const allRequirements = new Set<string>();
    testSuites.forEach(suite => {
      suite.requirements.forEach(req => allRequirements.add(req));
    });
    
    const sortedRequirements = Array.from(allRequirements).sort();
    
    console.log(`  Total Requirements Covered: ${sortedRequirements.length}`);
    console.log(`  Requirements: ${sortedRequirements.join(', ')}`);
    
    // Show which test suites cover each requirement
    const requirementMap = new Map<string, string[]>();
    testSuites.forEach(suite => {
      suite.requirements.forEach(req => {
        if (!requirementMap.has(req)) {
          requirementMap.set(req, []);
        }
        requirementMap.get(req)!.push(suite.name);
      });
    });
    
    console.log(`\n  Requirement Coverage Details:`);
    sortedRequirements.forEach(req => {
      const suites = requirementMap.get(req) || [];
      console.log(`    ${req}: ${suites.join(', ')}`);
    });
  }

  private printPerformanceAnalysis(): void {
    console.log(`\n⚡ Performance Analysis:`);
    
    const avgDuration = this.results.reduce((sum, r) => sum + r.duration, 0) / this.results.length;
    const slowestTest = this.results.reduce((prev, curr) => 
      curr.duration > prev.duration ? curr : prev
    );
    const fastestTest = this.results.reduce((prev, curr) => 
      curr.duration < prev.duration ? curr : prev
    );
    
    console.log(`  Average Test Duration: ${(avgDuration / 1000).toFixed(2)}s`);
    console.log(`  Slowest Test: ${slowestTest.suite} (${(slowestTest.duration / 1000).toFixed(2)}s)`);
    console.log(`  Fastest Test: ${fastestTest.suite} (${(fastestTest.duration / 1000).toFixed(2)}s)`);
    
    // Performance recommendations
    if (slowestTest.duration > 60000) { // > 1 minute
      console.log(`  ⚠️  Consider optimizing ${slowestTest.suite} - it's taking over 1 minute`);
    }
    
    if (avgDuration > 30000) { // > 30 seconds average
      console.log(`  ⚠️  Average test duration is high - consider test optimization`);
    }
  }
}

// Main execution
async function main() {
  const runner = new TestRunner();
  
  try {
    await runner.runAllTests();
    process.exit(0);
  } catch (error) {
    console.error("❌ Test runner failed:", error);
    process.exit(1);
  }
}

// Run if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export { TestRunner, testSuites };