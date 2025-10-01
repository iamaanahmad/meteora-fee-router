#!/usr/bin/env node

/**
 * NPM Publishing Script for Meteora Fee Router
 * Publishes the package to NPM registry under @ashqking scope
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

console.log('🚀 Publishing Meteora Fee Router to NPM...');

// Check if we're logged in to NPM
try {
  const whoami = execSync('npm whoami', { encoding: 'utf8' }).trim();
  console.log(`✅ Logged in as: ${whoami}`);
  
  if (whoami !== 'ashqking') {
    console.error('❌ Error: Must be logged in as @ashqking to publish');
    console.log('💡 Run: npm login');
    process.exit(1);
  }
} catch (error) {
  console.error('❌ Error: Not logged in to NPM');
  console.log('💡 Run: npm login');
  process.exit(1);
}

// Verify package.json
const packagePath = path.join(__dirname, '..', 'package.json');
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));

console.log(`📦 Package: ${packageJson.name}`);
console.log(`🏷️  Version: ${packageJson.version}`);
console.log(`👤 Author: ${packageJson.author}`);

// Check if version already exists
try {
  const existingVersions = execSync(`npm view ${packageJson.name} versions --json`, { encoding: 'utf8' });
  const versions = JSON.parse(existingVersions);
  
  if (versions.includes(packageJson.version)) {
    console.error(`❌ Error: Version ${packageJson.version} already exists`);
    console.log('💡 Update version in package.json and try again');
    process.exit(1);
  }
} catch (error) {
  // Package doesn't exist yet, which is fine for first publish
  console.log('📦 First time publishing this package');
}

// Build the project
console.log('🏗️  Building project...');
try {
  execSync('npm run build', { stdio: 'inherit' });
  console.log('✅ Build successful');
} catch (error) {
  console.error('❌ Build failed');
  process.exit(1);
}

// Run tests
console.log('🧪 Running tests...');
try {
  execSync('npm run test:unit', { stdio: 'inherit' });
  console.log('✅ Tests passed');
} catch (error) {
  console.error('❌ Tests failed');
  process.exit(1);
}

// Publish to NPM
console.log('📤 Publishing to NPM...');
try {
  execSync('npm publish --access public', { stdio: 'inherit' });
  console.log('✅ Successfully published to NPM!');
  console.log(`🔗 Package URL: https://www.npmjs.com/package/${packageJson.name}`);
} catch (error) {
  console.error('❌ Publish failed');
  process.exit(1);
}

console.log('🎉 NPM publishing complete!');