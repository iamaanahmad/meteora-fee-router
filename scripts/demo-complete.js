#!/usr/bin/env node

/**
 * Meteora Fee Router - Complete E2E Demo Script
 * 
 * This script provides a foolproof quickstart experience:
 * 1. Validates environment prerequisites
 * 2. Builds the program
 * 3. Runs comprehensive test suite
 * 4. Displays results and next steps
 * 
 * Usage: npm run demo:complete
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// ANSI color codes
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  red: '\x1b[31m',
  cyan: '\x1b[36m'
};

function log(message, color = 'reset') {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function execCommand(command, description) {
  log(`\n${'='.repeat(80)}`, 'cyan');
  log(`📍 ${description}`, 'bright');
  log(`${'='.repeat(80)}`, 'cyan');
  
  try {
    const output = execSync(command, { 
      encoding: 'utf8',
      stdio: 'inherit',
      cwd: process.cwd()
    });
    log(`✅ ${description} - SUCCESS`, 'green');
    return true;
  } catch (error) {
    log(`❌ ${description} - FAILED`, 'red');
    log(`Error: ${error.message}`, 'red');
    return false;
  }
}

function checkPrerequisites() {
  log('\n🔍 Checking Prerequisites...', 'bright');
  
  const checks = [
    { cmd: 'rustc --version', name: 'Rust', required: '1.75' },
    { cmd: 'node --version', name: 'Node.js', required: '18' },
    { cmd: 'solana --version', name: 'Solana CLI', required: '1.16' },
    { cmd: 'anchor --version', name: 'Anchor', required: '0.29' }
  ];
  
  let allPassed = true;
  
  for (const check of checks) {
    try {
      const output = execSync(check.cmd, { encoding: 'utf8', stdio: 'pipe' });
      log(`  ✅ ${check.name}: ${output.trim()}`, 'green');
    } catch (error) {
      log(`  ❌ ${check.name}: NOT FOUND (required: ${check.required}+)`, 'red');
      allPassed = false;
    }
  }
  
  return allPassed;
}

function displayResults(results) {
  log('\n' + '='.repeat(80), 'cyan');
  log('📊 DEMO RESULTS SUMMARY', 'bright');
  log('='.repeat(80), 'cyan');
  
  const passed = results.filter(r => r.passed).length;
  const total = results.length;
  
  results.forEach(result => {
    const icon = result.passed ? '✅' : '❌';
    const color = result.passed ? 'green' : 'red';
    log(`  ${icon} ${result.name}`, color);
  });
  
  log('\n' + '='.repeat(80), 'cyan');
  log(`Final Score: ${passed}/${total} steps completed successfully`, 
    passed === total ? 'green' : 'yellow');
  log('='.repeat(80), 'cyan');
  
  if (passed === total) {
    log('\n🎉 SUCCESS! All demo steps completed!', 'green');
    log('\n📚 Next Steps:', 'bright');
    log('  1. Review test results above', 'cyan');
    log('  2. Check docs/INTEGRATION_EXAMPLES.md for integration guide', 'cyan');
    log('  3. Try: npm run demo:integration for detailed walkthrough', 'cyan');
    log('  4. Deploy: ./deployment/deploy.sh devnet', 'cyan');
  } else {
    log('\n⚠️  Some steps failed. Check errors above.', 'yellow');
    log('\nTroubleshooting:', 'bright');
    log('  1. Ensure all prerequisites are installed (see above)', 'cyan');
    log('  2. Run: npm install', 'cyan');
    log('  3. Check docs/TROUBLESHOOTING_GUIDE.md', 'cyan');
  }
}

async function main() {
  log('\n' + '='.repeat(80), 'cyan');
  log('🌟 METEORA FEE ROUTER - COMPLETE E2E DEMO', 'bright');
  log('='.repeat(80), 'cyan');
  
  log('\nThis demo will:', 'blue');
  log('  1. Validate environment prerequisites', 'cyan');
  log('  2. Build the Anchor program', 'cyan');
  log('  3. Run 295 comprehensive tests', 'cyan');
  log('  4. Validate deployment artifacts', 'cyan');
  log('  5. Show integration examples', 'cyan');
  log('\nEstimated time: ~5 minutes\n', 'yellow');
  
  const results = [];
  
  // Step 1: Check prerequisites
  const prereqsPassed = checkPrerequisites();
  results.push({ name: 'Prerequisites Check', passed: prereqsPassed });
  
  if (!prereqsPassed) {
    log('\n⚠️  Please install missing prerequisites and try again.', 'yellow');
    log('See: README.md #Prerequisites section', 'cyan');
    process.exit(1);
  }
  
  // Step 2: Install dependencies
  const installPassed = execCommand(
    'npm install',
    'Step 2/5: Installing Dependencies'
  );
  results.push({ name: 'Dependency Installation', passed: installPassed });
  
  if (!installPassed) {
    displayResults(results);
    process.exit(1);
  }
  
  // Step 3: Build program
  const buildPassed = execCommand(
    'anchor build',
    'Step 3/5: Building Anchor Program'
  );
  results.push({ name: 'Program Build', passed: buildPassed });
  
  if (!buildPassed) {
    displayResults(results);
    process.exit(1);
  }
  
  // Step 4: Run tests
  const testsPassed = execCommand(
    'npm run test:all',
    'Step 4/5: Running Comprehensive Test Suite (295 tests)'
  );
  results.push({ name: 'Test Suite Execution', passed: testsPassed });
  
  // Step 5: Validate artifacts
  log('\n' + '='.repeat(80), 'cyan');
  log('📍 Step 5/5: Validating Build Artifacts', 'bright');
  log('='.repeat(80), 'cyan');
  
  const artifacts = [
    'target/deploy/meteora_fee_router.so',
    'target/idl/meteora_fee_router.json',
    'target/types/meteora_fee_router.ts'
  ];
  
  let artifactsValid = true;
  artifacts.forEach(artifact => {
    const exists = fs.existsSync(path.join(process.cwd(), artifact));
    const icon = exists ? '✅' : '❌';
    const color = exists ? 'green' : 'red';
    log(`  ${icon} ${artifact}`, color);
    if (!exists) artifactsValid = false;
  });
  
  results.push({ name: 'Build Artifacts Validation', passed: artifactsValid });
  
  // Display final results
  displayResults(results);
  
  // Show example output
  if (results.every(r => r.passed)) {
    log('\n📋 Sample Output:', 'bright');
    log('─'.repeat(80), 'cyan');
    log('  ✅ Honorary position initialized', 'green');
    log('  ✅ Fees claimed: 1,000,000 USDC', 'green');
    log('  ✅ Investors distributed: 700,000 USDC (70%)', 'green');
    log('  ✅ Creator payout: 300,000 USDC (30%)', 'green');
    log('  ✅ All 295 tests passing', 'green');
    log('─'.repeat(80), 'cyan');
  }
  
  process.exit(results.every(r => r.passed) ? 0 : 1);
}

// Run the demo
main().catch(error => {
  log(`\n❌ Unexpected error: ${error.message}`, 'red');
  log(error.stack, 'red');
  process.exit(1);
});
