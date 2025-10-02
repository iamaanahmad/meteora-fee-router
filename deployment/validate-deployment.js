#!/usr/bin/env node

/**
 * Meteora Fee Router Deployment Validation Script
 * Validates that the deployed program is working correctly
 */

const { Connection, PublicKey, Keypair } = require('@solana/web3.js');
const { Program, AnchorProvider, Wallet } = require('@coral-xyz/anchor');
const fs = require('fs');

// Configuration
const CLUSTER_URL = process.env.CLUSTER_URL || 'https://api.devnet.solana.com';
const PROGRAM_ID = process.env.PROGRAM_ID || 'Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS';

async function validateDeployment() {
    console.log('🔍 Validating Meteora Fee Router deployment...');
    
    try {
        // Setup connection
        const connection = new Connection(CLUSTER_URL, 'confirmed');
        console.log(`📡 Connected to ${CLUSTER_URL}`);
        
        // Load program
        const programId = new PublicKey(PROGRAM_ID);
        console.log(`📋 Program ID: ${programId.toString()}`);
        
        // Check if program exists
        const programInfo = await connection.getAccountInfo(programId);
        if (!programInfo) {
            throw new Error('Program not found on cluster');
        }
        
        console.log('✅ Program account found');
        console.log(`   Executable: ${programInfo.executable}`);
        console.log(`   Owner: ${programInfo.owner.toString()}`);
        console.log(`   Data length: ${programInfo.data.length} bytes`);
        
        // Load IDL
        let idl;
        try {
            idl = JSON.parse(fs.readFileSync('./target/idl/meteora_fee_router.json', 'utf8'));
            console.log('✅ IDL loaded successfully');
        } catch (error) {
            throw new Error(`Failed to load IDL: ${error.message}`);
        }
        
        // Validate IDL structure
        const expectedInstructions = ['initialize_honorary_position', 'distribute_fees'];
        const actualInstructions = idl.instructions.map(ix => ix.name);
        
        for (const expected of expectedInstructions) {
            if (!actualInstructions.includes(expected)) {
                throw new Error(`Missing instruction: ${expected}`);
            }
        }
        console.log('✅ All expected instructions found in IDL');
        
        // Validate events
        const expectedEvents = [
            'HonoraryPositionInitialized',
            'QuoteFeesClaimed', 
            'InvestorPayoutPage',
            'CreatorPayoutDayClosed'
        ];
        
        const actualEvents = idl.events.map(event => event.name);
        for (const expected of expectedEvents) {
            if (!actualEvents.includes(expected)) {
                throw new Error(`Missing event: ${expected}`);
            }
        }
        console.log('✅ All expected events found in IDL');
        
        // Validate accounts
        const expectedAccounts = ['PolicyConfig', 'DistributionProgress'];
        const actualAccounts = idl.accounts.map(acc => acc.name);
        
        for (const expected of expectedAccounts) {
            if (!actualAccounts.includes(expected)) {
                throw new Error(`Missing account type: ${expected}`);
            }
        }
        console.log('✅ All expected account types found in IDL');
        
        // Check program size
        const programSize = programInfo.data.length;
        const maxSize = 1024 * 1024; // 1MB
        
        if (programSize > maxSize) {
            console.log(`⚠️  Warning: Program size (${programSize} bytes) exceeds recommended limit`);
        } else {
            console.log(`✅ Program size (${programSize} bytes) is within limits`);
        }
        
        console.log('\n🎉 Deployment validation successful!');
        console.log('\nValidation Summary:');
        console.log(`  ✅ Program deployed at: ${programId.toString()}`);
        console.log(`  ✅ Program size: ${programSize} bytes`);
        console.log(`  ✅ Instructions: ${actualInstructions.length}`);
        console.log(`  ✅ Events: ${actualEvents.length}`);
        console.log(`  ✅ Account types: ${actualAccounts.length}`);
        
        return true;
        
    } catch (error) {
        console.error('❌ Deployment validation failed:', error.message);
        return false;
    }
}

// Run validation if called directly
if (require.main === module) {
    validateDeployment()
        .then(success => process.exit(success ? 0 : 1))
        .catch(error => {
            console.error('❌ Validation error:', error);
            process.exit(1);
        });
}

module.exports = { validateDeployment };