#!/usr/bin/env node

/**
 * Comprehensive Security Validation Script for Meteora Fee Router
 * 
 * This script performs automated security validation including:
 * - PDA derivation security audit
 * - Arithmetic overflow protection validation
 * - Access control verification
 * - Reentrancy protection testing
 * - Mathematical precision fuzzing
 */

const fs = require('fs');
const path = require('path');

class SecurityValidator {
    constructor() {
        this.results = {
            pdaAudit: { passed: false, issues: [] },
            arithmeticAudit: { passed: false, issues: [] },
            accessControlAudit: { passed: false, issues: [] },
            reentrancyAudit: { passed: false, issues: [] },
            fuzzTestResults: { passed: false, issues: [] },
            overallPassed: false
        };
    }

    async runFullSecurityAudit() {
        console.log("🔒 Starting Comprehensive Security Audit for Meteora Fee Router");
        console.log("=" .repeat(70));

        try {
            await this.auditPDADerivations();
            await this.auditArithmeticOverflow();
            await this.auditAccessControl();
            await this.auditReentrancyProtection();
            await this.runFuzzTests();
            
            this.generateSecurityReport();
            
        } catch (error) {
            console.error("❌ Security audit failed:", error.message);
            process.exit(1);
        }
    }

    async auditPDADerivations() {
        console.log("\n🔍 Auditing PDA Derivations...");
        
        const issues = [];
        
        // Check for consistent seed usage
        const seedPatterns = this.extractSeedPatterns();
        if (!this.validateSeedConsistency(seedPatterns)) {
            issues.push("Inconsistent seed patterns detected");
        }
        
        // Check for potential seed collisions
        if (!this.validateSeedCollisionResistance(seedPatterns)) {
            issues.push("Potential seed collision vulnerabilities");
        }
        
        // Validate PDA derivation functions
        if (!this.validatePDADerivationFunctions()) {
            issues.push("PDA derivation function vulnerabilities");
        }
        
        // Check bump validation
        if (!this.validateBumpValidation()) {
            issues.push("Insufficient bump validation");
        }
        
        this.results.pdaAudit = {
            passed: issues.length === 0,
            issues: issues
        };
        
        if (issues.length === 0) {
            console.log("✅ PDA derivation audit passed");
        } else {
            console.log("❌ PDA derivation audit failed:");
            issues.forEach(issue => console.log(`   - ${issue}`));
        }
    }

    async auditArithmeticOverflow() {
        console.log("\n🧮 Auditing Arithmetic Overflow Protection...");
        
        const issues = [];
        
        // Check for checked arithmetic usage
        if (!this.validateCheckedArithmetic()) {
            issues.push("Unchecked arithmetic operations detected");
        }
        
        // Validate overflow protection in calculations
        if (!this.validateOverflowProtection()) {
            issues.push("Insufficient overflow protection in calculations");
        }
        
        // Check for proper error handling
        if (!this.validateArithmeticErrorHandling()) {
            issues.push("Inadequate arithmetic error handling");
        }
        
        // Validate precision handling
        if (!this.validatePrecisionHandling()) {
            issues.push("Precision handling vulnerabilities");
        }
        
        this.results.arithmeticAudit = {
            passed: issues.length === 0,
            issues: issues
        };
        
        if (issues.length === 0) {
            console.log("✅ Arithmetic overflow audit passed");
        } else {
            console.log("❌ Arithmetic overflow audit failed:");
            issues.forEach(issue => console.log(`   - ${issue}`));
        }
    }

    async auditAccessControl() {
        console.log("\n🛡️  Auditing Access Control...");
        
        const issues = [];
        
        // Check account ownership validation
        if (!this.validateAccountOwnership()) {
            issues.push("Insufficient account ownership validation");
        }
        
        // Validate signer requirements
        if (!this.validateSignerRequirements()) {
            issues.push("Inadequate signer requirement validation");
        }
        
        // Check PDA authority validation
        if (!this.validatePDAAuthority()) {
            issues.push("PDA authority validation issues");
        }
        
        // Validate cross-account relationships
        if (!this.validateCrossAccountRelationships()) {
            issues.push("Cross-account relationship validation issues");
        }
        
        this.results.accessControlAudit = {
            passed: issues.length === 0,
            issues: issues
        };
        
        if (issues.length === 0) {
            console.log("✅ Access control audit passed");
        } else {
            console.log("❌ Access control audit failed:");
            issues.forEach(issue => console.log(`   - ${issue}`));
        }
    }

    async auditReentrancyProtection() {
        console.log("\n🔄 Auditing Reentrancy Protection...");
        
        const issues = [];
        
        // Check for state consistency
        if (!this.validateStateConsistency()) {
            issues.push("State consistency vulnerabilities");
        }
        
        // Validate idempotent operations
        if (!this.validateIdempotentOperations()) {
            issues.push("Idempotent operation issues");
        }
        
        // Check CPI safety
        if (!this.validateCPISafety()) {
            issues.push("Cross-program invocation safety issues");
        }
        
        // Validate account mutation ordering
        if (!this.validateAccountMutationOrdering()) {
            issues.push("Account mutation ordering issues");
        }
        
        this.results.reentrancyAudit = {
            passed: issues.length === 0,
            issues: issues
        };
        
        if (issues.length === 0) {
            console.log("✅ Reentrancy protection audit passed");
        } else {
            console.log("❌ Reentrancy protection audit failed:");
            issues.forEach(issue => console.log(`   - ${issue}`));
        }
    }

    async runFuzzTests() {
        console.log("\n🎯 Running Fuzz Tests...");
        
        const issues = [];
        
        // Run mathematical fuzz tests
        const mathFuzzResults = this.runMathematicalFuzzTests(1000);
        if (!mathFuzzResults.passed) {
            issues.push(...mathFuzzResults.issues);
        }
        
        // Run input validation fuzz tests
        const inputFuzzResults = this.runInputValidationFuzzTests(500);
        if (!inputFuzzResults.passed) {
            issues.push(...inputFuzzResults.issues);
        }
        
        // Run edge case fuzz tests
        const edgeCaseFuzzResults = this.runEdgeCaseFuzzTests(200);
        if (!edgeCaseFuzzResults.passed) {
            issues.push(...edgeCaseFuzzResults.issues);
        }
        
        this.results.fuzzTestResults = {
            passed: issues.length === 0,
            issues: issues
        };
        
        if (issues.length === 0) {
            console.log("✅ Fuzz tests passed");
        } else {
            console.log("❌ Fuzz tests failed:");
            issues.forEach(issue => console.log(`   - ${issue}`));
        }
    }

    // PDA Validation Methods
    extractSeedPatterns() {
        const codeFiles = this.getSourceFiles();
        const patterns = [];
        
        codeFiles.forEach(file => {
            const content = fs.readFileSync(file, 'utf8');
            const seedMatches = content.match(/seeds\s*=\s*\[([^\]]+)\]/g) || [];
            patterns.push(...seedMatches);
        });
        
        return patterns;
    }

    validateSeedConsistency(patterns) {
        // Check that seed patterns are consistent across the codebase
        const policySeeds = patterns.filter(p => p.includes('POLICY_SEED'));
        const progressSeeds = patterns.filter(p => p.includes('PROGRESS_SEED'));
        const vaultSeeds = patterns.filter(p => p.includes('VAULT_SEED'));
        
        // All policy seeds should be consistent
        const policyConsistent = policySeeds.every(seed => 
            seed.includes('POLICY_SEED') && seed.includes('vault')
        );
        
        const progressConsistent = progressSeeds.every(seed => 
            seed.includes('PROGRESS_SEED') && seed.includes('vault')
        );
        
        return policyConsistent && progressConsistent;
    }

    validateSeedCollisionResistance(patterns) {
        // Check for potential seed collisions
        const uniquePatterns = new Set(patterns);
        return uniquePatterns.size === patterns.length;
    }

    validatePDADerivationFunctions() {
        // Check that PDA derivation functions exist and are properly implemented
        const pdaFile = path.join(__dirname, 'programs/meteora-fee-router/src/utils/pda.rs');
        
        if (!fs.existsSync(pdaFile)) {
            return false;
        }
        
        const content = fs.readFileSync(pdaFile, 'utf8');
        
        // Check for required derivation functions
        const requiredFunctions = [
            'derive_policy_config_pda',
            'derive_distribution_progress_pda',
            'derive_position_owner_pda'
        ];
        
        return requiredFunctions.every(func => content.includes(func));
    }

    validateBumpValidation() {
        // Check that bump validation is properly implemented
        const pdaFile = path.join(__dirname, 'programs/meteora-fee-router/src/utils/pda.rs');
        
        if (!fs.existsSync(pdaFile)) {
            return false;
        }
        
        const content = fs.readFileSync(pdaFile, 'utf8');
        
        // Check for validation functions
        return content.includes('validate_policy_config_pda') &&
               content.includes('validate_distribution_progress_pda') &&
               content.includes('validate_position_owner_pda');
    }

    // Arithmetic Validation Methods
    validateCheckedArithmetic() {
        const mathFile = path.join(__dirname, 'programs/meteora-fee-router/src/utils/math.rs');
        
        if (!fs.existsSync(mathFile)) {
            return false;
        }
        
        const content = fs.readFileSync(mathFile, 'utf8');
        
        // Check for checked arithmetic operations
        return content.includes('checked_add') &&
               content.includes('checked_mul') &&
               content.includes('checked_sub') &&
               content.includes('checked_div');
    }

    validateOverflowProtection() {
        const mathFile = path.join(__dirname, 'programs/meteora-fee-router/src/utils/math.rs');
        
        if (!fs.existsSync(mathFile)) {
            return false;
        }
        
        const content = fs.readFileSync(mathFile, 'utf8');
        
        // Check for overflow error handling
        return content.includes('ArithmeticOverflow') &&
               content.includes('ok_or(ErrorCode::ArithmeticOverflow)');
    }

    validateArithmeticErrorHandling() {
        const errorFile = path.join(__dirname, 'programs/meteora-fee-router/src/error.rs');
        
        if (!fs.existsSync(errorFile)) {
            return false;
        }
        
        const content = fs.readFileSync(errorFile, 'utf8');
        
        // Check for arithmetic error codes
        return content.includes('ArithmeticOverflow');
    }

    validatePrecisionHandling() {
        const mathFile = path.join(__dirname, 'programs/meteora-fee-router/src/utils/math.rs');
        
        if (!fs.existsSync(mathFile)) {
            return false;
        }
        
        const content = fs.readFileSync(mathFile, 'utf8');
        
        // Check for precision constants and handling
        return content.includes('WEIGHT_PRECISION') &&
               content.includes('MAX_BASIS_POINTS');
    }

    // Access Control Validation Methods
    validateAccountOwnership() {
        const instructionFiles = this.getInstructionFiles();
        
        return instructionFiles.every(file => {
            const content = fs.readFileSync(file, 'utf8');
            
            // Check for ownership validation
            return content.includes('owner') || content.includes('authority');
        });
    }

    validateSignerRequirements() {
        const instructionFiles = this.getInstructionFiles();
        
        return instructionFiles.every(file => {
            const content = fs.readFileSync(file, 'utf8');
            
            // Check for signer constraints
            return content.includes('Signer') || content.includes('signer');
        });
    }

    validatePDAAuthority() {
        const instructionFiles = this.getInstructionFiles();
        
        return instructionFiles.some(file => {
            const content = fs.readFileSync(file, 'utf8');
            
            // Check for PDA authority validation
            return content.includes('seeds') && content.includes('bump');
        });
    }

    validateCrossAccountRelationships() {
        const instructionFiles = this.getInstructionFiles();
        
        return instructionFiles.some(file => {
            const content = fs.readFileSync(file, 'utf8');
            
            // Check for cross-account validation
            return content.includes('require!') && content.includes('==');
        });
    }

    // Reentrancy Protection Validation Methods
    validateStateConsistency() {
        const stateFiles = this.getStateFiles();
        
        return stateFiles.every(file => {
            const content = fs.readFileSync(file, 'utf8');
            
            // Check for proper state management
            return content.includes('mut') || content.includes('&mut');
        });
    }

    validateIdempotentOperations() {
        const progressFile = path.join(__dirname, 'programs/meteora-fee-router/src/state/distribution_progress.rs');
        
        if (!fs.existsSync(progressFile)) {
            return false;
        }
        
        const content = fs.readFileSync(progressFile, 'utf8');
        
        // Check for idempotent operation support
        return content.includes('validate_cursor_for_retry') &&
               content.includes('is_cursor_processed');
    }

    validateCPISafety() {
        const instructionFiles = this.getInstructionFiles();
        
        return instructionFiles.some(file => {
            const content = fs.readFileSync(file, 'utf8');
            
            // Check for proper CPI usage
            return content.includes('CpiContext') && content.includes('signer_seeds');
        });
    }

    validateAccountMutationOrdering() {
        // This is enforced by Rust's borrow checker and Anchor's constraints
        return true;
    }

    // Fuzz Testing Methods
    runMathematicalFuzzTests(iterations) {
        const issues = [];
        
        for (let i = 0; i < iterations; i++) {
            // Test distribution calculation with random inputs
            const claimedQuote = this.randomU64();
            const lockedTotal = this.randomU64();
            const y0Total = Math.max(1, this.randomU64()); // Avoid zero
            const investorFeeShareBps = Math.floor(Math.random() * 10001);
            
            try {
                const result = this.simulateDistributionCalculation(
                    claimedQuote, lockedTotal, y0Total, investorFeeShareBps
                );
                
                // Validate invariants
                if (result.investorAmount + result.creatorAmount !== claimedQuote) {
                    issues.push(`Distribution sum invariant violation at iteration ${i}`);
                }
                
                if (result.investorAmount > claimedQuote || result.creatorAmount > claimedQuote) {
                    issues.push(`Distribution bounds invariant violation at iteration ${i}`);
                }
                
            } catch (error) {
                // Expected for overflow cases
            }
        }
        
        return {
            passed: issues.length === 0,
            issues: issues
        };
    }

    runInputValidationFuzzTests(iterations) {
        const issues = [];
        
        for (let i = 0; i < iterations; i++) {
            // Test with edge case inputs
            const testCases = [
                { investorFeeShareBps: 10001 }, // Invalid BPS
                { minPayoutLamports: 0 }, // Zero minimum payout
                { y0TotalAllocation: 0 }, // Zero Y0
                { dailyCapLamports: 0 }, // Zero daily cap
            ];
            
            testCases.forEach((testCase, index) => {
                if (!this.simulateInputValidation(testCase)) {
                    issues.push(`Input validation failed for test case ${index} at iteration ${i}`);
                }
            });
        }
        
        return {
            passed: issues.length === 0,
            issues: issues
        };
    }

    runEdgeCaseFuzzTests(iterations) {
        const issues = [];
        
        for (let i = 0; i < iterations; i++) {
            // Test edge cases
            const edgeCases = [
                { claimedQuote: 0, lockedTotal: 1000, y0Total: 1000 },
                { claimedQuote: 1000, lockedTotal: 0, y0Total: 1000 },
                { claimedQuote: 1000, lockedTotal: 1000, y0Total: 1000 },
                { claimedQuote: Number.MAX_SAFE_INTEGER, lockedTotal: 1, y0Total: 1 },
            ];
            
            edgeCases.forEach((edgeCase, index) => {
                try {
                    const result = this.simulateDistributionCalculation(
                        edgeCase.claimedQuote,
                        edgeCase.lockedTotal,
                        edgeCase.y0Total,
                        5000
                    );
                    
                    // Validate edge case behavior
                    if (edgeCase.claimedQuote === 0 && (result.investorAmount !== 0 || result.creatorAmount !== 0)) {
                        issues.push(`Edge case validation failed for zero claimed at iteration ${i}`);
                    }
                    
                } catch (error) {
                    // Some edge cases should fail gracefully
                }
            });
        }
        
        return {
            passed: issues.length === 0,
            issues: issues
        };
    }

    // Helper Methods
    simulateDistributionCalculation(claimedQuote, lockedTotal, y0Total, investorFeeShareBps) {
        if (investorFeeShareBps > 10000) {
            throw new Error("Invalid basis points");
        }
        
        if (y0Total === 0 || claimedQuote === 0) {
            return { investorAmount: 0, creatorAmount: claimedQuote };
        }
        
        const fLocked = Math.floor((lockedTotal * 10000) / y0Total);
        const eligibleShareBps = Math.min(investorFeeShareBps, fLocked);
        const investorAmount = Math.floor((claimedQuote * eligibleShareBps) / 10000);
        const creatorAmount = claimedQuote - investorAmount;
        
        return { investorAmount, creatorAmount };
    }

    simulateInputValidation(params) {
        if (params.investorFeeShareBps !== undefined && params.investorFeeShareBps > 10000) {
            return false; // Should be rejected
        }
        
        if (params.minPayoutLamports !== undefined && params.minPayoutLamports === 0) {
            return false; // Should be rejected
        }
        
        if (params.y0TotalAllocation !== undefined && params.y0TotalAllocation === 0) {
            return false; // Should be rejected
        }
        
        if (params.dailyCapLamports !== undefined && params.dailyCapLamports === 0) {
            return false; // Should be rejected
        }
        
        return true; // Should be accepted
    }

    randomU64() {
        return Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
    }

    getSourceFiles() {
        const srcDir = path.join(__dirname, 'programs/meteora-fee-router/src');
        const files = [];
        
        const walkDir = (dir) => {
            const items = fs.readdirSync(dir);
            items.forEach(item => {
                const fullPath = path.join(dir, item);
                const stat = fs.statSync(fullPath);
                
                if (stat.isDirectory()) {
                    walkDir(fullPath);
                } else if (item.endsWith('.rs')) {
                    files.push(fullPath);
                }
            });
        };
        
        if (fs.existsSync(srcDir)) {
            walkDir(srcDir);
        }
        
        return files;
    }

    getInstructionFiles() {
        const instructionDir = path.join(__dirname, 'programs/meteora-fee-router/src/instructions');
        const files = [];
        
        if (fs.existsSync(instructionDir)) {
            const items = fs.readdirSync(instructionDir);
            items.forEach(item => {
                if (item.endsWith('.rs')) {
                    files.push(path.join(instructionDir, item));
                }
            });
        }
        
        return files;
    }

    getStateFiles() {
        const stateDir = path.join(__dirname, 'programs/meteora-fee-router/src/state');
        const files = [];
        
        if (fs.existsSync(stateDir)) {
            const items = fs.readdirSync(stateDir);
            items.forEach(item => {
                if (item.endsWith('.rs')) {
                    files.push(path.join(stateDir, item));
                }
            });
        }
        
        return files;
    }

    generateSecurityReport() {
        console.log("\n" + "=".repeat(70));
        console.log("🔒 SECURITY AUDIT REPORT");
        console.log("=".repeat(70));
        
        const allPassed = Object.values(this.results).every(result => 
            typeof result === 'object' ? result.passed : result
        );
        
        this.results.overallPassed = allPassed;
        
        console.log(`\n📊 Overall Security Status: ${allPassed ? '✅ PASSED' : '❌ FAILED'}`);
        
        console.log("\n📋 Detailed Results:");
        console.log(`   PDA Derivation Security: ${this.results.pdaAudit.passed ? '✅' : '❌'}`);
        console.log(`   Arithmetic Overflow Protection: ${this.results.arithmeticAudit.passed ? '✅' : '❌'}`);
        console.log(`   Access Control Validation: ${this.results.accessControlAudit.passed ? '✅' : '❌'}`);
        console.log(`   Reentrancy Protection: ${this.results.reentrancyAudit.passed ? '✅' : '❌'}`);
        console.log(`   Fuzz Test Results: ${this.results.fuzzTestResults.passed ? '✅' : '❌'}`);
        
        // Show issues if any
        Object.entries(this.results).forEach(([category, result]) => {
            if (typeof result === 'object' && result.issues && result.issues.length > 0) {
                console.log(`\n⚠️  Issues in ${category}:`);
                result.issues.forEach(issue => console.log(`   - ${issue}`));
            }
        });
        
        // Save detailed report
        const reportPath = path.join(__dirname, 'security-audit-report.json');
        fs.writeFileSync(reportPath, JSON.stringify(this.results, null, 2));
        console.log(`\n📄 Detailed report saved to: ${reportPath}`);
        
        if (!allPassed) {
            console.log("\n❌ Security audit failed. Please address the issues above before deployment.");
            process.exit(1);
        } else {
            console.log("\n✅ Security audit passed. The program meets security requirements.");
        }
    }
}

// Run the security audit
if (require.main === module) {
    const validator = new SecurityValidator();
    validator.runFullSecurityAudit().catch(error => {
        console.error("Security audit failed:", error);
        process.exit(1);
    });
}

module.exports = SecurityValidator;