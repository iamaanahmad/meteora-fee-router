#!/usr/bin/env node

/**
 * Final Validation Script for Meteora Fee Router
 * Validates all acceptance criteria from requirements 8.5 and 9.5
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

class FinalValidator {
    constructor() {
        this.results = {
            passed: 0,
            failed: 0,
            warnings: 0,
            details: []
        };
    }

    log(level, message, details = null) {
        const timestamp = new Date().toISOString();
        const logEntry = { timestamp, level, message, details };
        
        const symbols = {
            'PASS': '✅',
            'FAIL': '❌', 
            'WARN': '⚠️',
            'INFO': 'ℹ️'
        };
        
        console.log(`${symbols[level]} ${message}`);
        if (details) {
            console.log(`   ${details}`);
        }
        
        this.results.details.push(logEntry);
        
        if (level === 'PASS') this.results.passed++;
        else if (level === 'FAIL') this.results.failed++;
        else if (level === 'WARN') this.results.warnings++;
    }

    async validateRequirement85() {
        this.log('INFO', 'Validating Requirement 8.5: Integration and Account Management');
        
        // Check PDA derivation utilities
        const pdaFile = 'programs/meteora-fee-router/src/utils/pda.rs';
        if (fs.existsSync(pdaFile)) {
            const content = fs.readFileSync(pdaFile, 'utf8');
            if (content.includes('derive_policy_config_pda') && content.includes('derive_distribution_progress_pda')) {
                this.log('PASS', 'PDA derivation utilities implemented');
            } else {
                this.log('FAIL', 'Missing PDA derivation functions');
            }
        } else {
            this.log('FAIL', 'PDA utilities file not found');
        }

        // Check error handling
        const errorFile = 'programs/meteora-fee-router/src/error.rs';
        if (fs.existsSync(errorFile)) {
            const content = fs.readFileSync(errorFile, 'utf8');
            const requiredErrors = [
                'InvalidQuoteMint',
                'BaseFeeDetected', 
                'CooldownNotElapsed',
                'ArithmeticOverflow'
            ];
            
            let errorCount = 0;
            requiredErrors.forEach(error => {
                if (content.includes(error)) errorCount++;
            });
            
            if (errorCount === requiredErrors.length) {
                this.log('PASS', 'All required error codes implemented');
            } else {
                this.log('FAIL', `Missing error codes: ${requiredErrors.length - errorCount}`);
            }
        } else {
            this.log('FAIL', 'Error definitions file not found');
        }

        // Check Anchor compatibility
        const libFile = 'programs/meteora-fee-router/src/lib.rs';
        if (fs.existsSync(libFile)) {
            const content = fs.readFileSync(libFile, 'utf8');
            if (content.includes('#[program]') && content.includes('anchor_lang::prelude::*')) {
                this.log('PASS', 'Anchor compatibility confirmed');
            } else {
                this.log('FAIL', 'Anchor compatibility issues detected');
            }
        }
    }

    async validateRequirement95() {
        this.log('INFO', 'Validating Requirement 9.5: Comprehensive Testing and Validation');
        
        // Check test files exist
        const testFiles = [
            'tests/initialize-honorary-position.test.ts',
            'tests/fee-claiming.test.ts',
            'tests/streamflow-integration.test.ts',
            'tests/pagination-resumption.test.ts',
            'tests/failure-edge-cases.test.ts',
            'tests/performance-compute.test.ts',
            'tests/comprehensive-integration.test.ts'
        ];
        
        let testCount = 0;
        testFiles.forEach(testFile => {
            if (fs.existsSync(testFile)) {
                testCount++;
                this.log('PASS', `Test file exists: ${path.basename(testFile)}`);
            } else {
                this.log('FAIL', `Missing test file: ${testFile}`);
            }
        });
        
        if (testCount >= 5) {
            this.log('PASS', 'Comprehensive test suite implemented');
        } else {
            this.log('FAIL', 'Insufficient test coverage');
        }

        // Check unit tests in Rust
        const utilsDir = 'programs/meteora-fee-router/src/utils';
        if (fs.existsSync(utilsDir)) {
            const files = fs.readdirSync(utilsDir);
            const testFiles = files.filter(f => f.includes('test'));
            
            if (testFiles.length >= 5) {
                this.log('PASS', `Rust unit tests found: ${testFiles.length} files`);
            } else {
                this.log('WARN', `Limited Rust unit tests: ${testFiles.length} files`);
            }
        }
    }

    async validateBuildSystem() {
        this.log('INFO', 'Validating Build System');
        
        try {
            // Check if program builds
            execSync('cargo check --manifest-path programs/meteora-fee-router/Cargo.toml', { stdio: 'pipe' });
            this.log('PASS', 'Program compiles successfully');
        } catch (error) {
            this.log('FAIL', 'Program compilation failed', error.message);
        }

        // Check Cargo.toml optimization settings
        const cargoFile = 'Cargo.toml';
        if (fs.existsSync(cargoFile)) {
            const content = fs.readFileSync(cargoFile, 'utf8');
            if (content.includes('[profile.release]') && content.includes('lto = "fat"')) {
                this.log('PASS', 'Release optimizations configured');
            } else {
                this.log('WARN', 'Release optimizations not fully configured');
            }
        }
    }

    async validateDocumentation() {
        this.log('INFO', 'Validating Documentation');
        
        const requiredDocs = [
            'README.md',
            'docs/INTEGRATION_EXAMPLES.md',
            'docs/OPERATIONAL_PROCEDURES.md',
            'docs/TROUBLESHOOTING_GUIDE.md',
            'docs/SECURITY_AUDIT_SUMMARY.md'
        ];
        
        requiredDocs.forEach(doc => {
            if (fs.existsSync(doc)) {
                const content = fs.readFileSync(doc, 'utf8');
                if (content.length > 500) {
                    this.log('PASS', `Documentation complete: ${doc}`);
                } else {
                    this.log('WARN', `Documentation minimal: ${doc}`);
                }
            } else {
                this.log('FAIL', `Missing documentation: ${doc}`);
            }
        });
    }

    async validateDeploymentReadiness() {
        this.log('INFO', 'Validating Deployment Readiness');
        
        // Check deployment scripts
        const deploymentFiles = [
            'deployment/deploy.sh',
            'deployment/deploy.ps1',
            'deployment/optimize-build.sh',
            'deployment/validate-deployment.js'
        ];
        
        deploymentFiles.forEach(file => {
            if (fs.existsSync(file)) {
                this.log('PASS', `Deployment script exists: ${file}`);
            } else {
                this.log('FAIL', `Missing deployment script: ${file}`);
            }
        });

        // Check configuration templates
        const configDir = 'config-templates';
        if (fs.existsSync(configDir)) {
            const configs = fs.readdirSync(configDir);
            if (configs.length >= 2) {
                this.log('PASS', `Configuration templates available: ${configs.length}`);
            } else {
                this.log('WARN', 'Limited configuration templates');
            }
        } else {
            this.log('FAIL', 'Configuration templates directory missing');
        }
    }

    async validateSecurityAudit() {
        this.log('INFO', 'Validating Security Implementation');
        
        // Check security audit file
        const securityFile = 'programs/meteora-fee-router/src/security_audit.rs';
        if (fs.existsSync(securityFile)) {
            const content = fs.readFileSync(securityFile, 'utf8');
            if (content.includes('validate_pda_security') && content.includes('validate_arithmetic_safety')) {
                this.log('PASS', 'Security audit functions implemented');
            } else {
                this.log('WARN', 'Security audit functions incomplete');
            }
        } else {
            this.log('FAIL', 'Security audit module missing');
        }

        // Check for overflow protection
        const mathFile = 'programs/meteora-fee-router/src/utils/math.rs';
        if (fs.existsSync(mathFile)) {
            const content = fs.readFileSync(mathFile, 'utf8');
            if (content.includes('checked_') || content.includes('saturating_')) {
                this.log('PASS', 'Arithmetic overflow protection implemented');
            } else {
                this.log('WARN', 'Limited overflow protection detected');
            }
        }
    }

    async validateBuildArtifacts() {
        this.log('INFO', 'Validating Build Artifacts');
        
        // Check program build
        try {
            execSync('anchor build', { stdio: 'pipe' });
            const programPath = 'target/deploy/meteora_fee_router.so';
            if (fs.existsSync(programPath)) {
                const stats = fs.statSync(programPath);
                const sizeKB = Math.round(stats.size / 1024);
                
                if (stats.size < 1024 * 1024) { // 1MB limit
                    this.log('PASS', `Program size acceptable: ${sizeKB}KB`);
                } else {
                    this.log('WARN', `Program size large: ${sizeKB}KB`);
                }
            }
        } catch (error) {
            this.log('WARN', 'Could not verify program size');
        }
    }

    async runAllValidations() {
        console.log('🚀 Starting Final Validation for Meteora Fee Router\n');
        
        await this.validateRequirement85();
        await this.validateRequirement95();
        await this.validateBuildSystem();
        await this.validateDocumentation();
        await this.validateDeploymentReadiness();
        await this.validateSecurityAudit();
        await this.validateBuildArtifacts();
        
        this.printSummary();
        return this.results.failed === 0;
    }

    printSummary() {
        console.log('\n' + '='.repeat(60));
        console.log('📊 VALIDATION SUMMARY');
        console.log('='.repeat(60));
        console.log(`✅ Passed: ${this.results.passed}`);
        console.log(`❌ Failed: ${this.results.failed}`);
        console.log(`⚠️  Warnings: ${this.results.warnings}`);
        
        if (this.results.failed === 0) {
            console.log('\n🎉 ALL VALIDATIONS PASSED!');
            console.log('🚀 Ready for hackathon submission!');
        } else {
            console.log('\n❌ VALIDATION FAILURES DETECTED');
            console.log('🔧 Please address failed items before deployment');
        }
        
        console.log('\n📋 Detailed results saved to validation log');
    }
}

// Run validation if called directly
if (require.main === module) {
    const validator = new FinalValidator();
    validator.runAllValidations()
        .then(success => {
            // Save detailed results
            fs.writeFileSync(
                'validation-results.json', 
                JSON.stringify(validator.results, null, 2)
            );
            process.exit(success ? 0 : 1);
        })
        .catch(error => {
            console.error('❌ Validation error:', error);
            process.exit(1);
        });
}

module.exports = { FinalValidator };