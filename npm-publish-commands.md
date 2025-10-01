# 📦 Manual NPM Publishing Commands

## Prerequisites
1. **NPM Account**: Ensure you're logged in as `@ashqking`
2. **Package Built**: Run build commands first
3. **Tests Passing**: Verify all tests pass

## Step-by-Step Publishing

### 1. Login to NPM
```bash
npm login
# Enter credentials for @ashqking account
```

### 2. Verify Login
```bash
npm whoami
# Should return: ashqking
```

### 3. Build the Project
```bash
# Build Anchor program
anchor build

# Install dependencies
npm install

# Run tests to ensure everything works
npm run test:unit
```

### 4. Update Version (if needed)
```bash
# Bump version automatically
npm version patch  # for 1.0.1
npm version minor  # for 1.1.0
npm version major  # for 2.0.0

# Or manually edit package.json version field
```

### 5. Publish to NPM
```bash
# Publish with public access (required for scoped packages)
npm publish --access public
```

### 6. Verify Publication
```bash
# Check if package is published
npm view @ashqking/meteora-fee-router

# Install your own package to test
npm install @ashqking/meteora-fee-router
```

## Alternative: Use the Publishing Script
```bash
# Make script executable
chmod +x scripts/publish-npm.js

# Run the automated publishing script
node scripts/publish-npm.js
```

## Package Information
- **Package Name**: `@ashqking/meteora-fee-router`
- **Scope**: `@ashqking`
- **Registry**: https://registry.npmjs.org
- **Package URL**: https://www.npmjs.com/package/@ashqking/meteora-fee-router

## Troubleshooting

### Error: "You do not have permission to publish"
```bash
# Ensure you're logged in as the correct user
npm whoami

# Login again if needed
npm logout
npm login
```

### Error: "Version already exists"
```bash
# Update version in package.json
npm version patch
```

### Error: "Package name too similar to existing package"
```bash
# The scoped name @ashqking/meteora-fee-router should be unique
# If not, try: @ashqking/meteora-fee-router-bounty
```

## Post-Publication
1. **Verify on NPM**: Visit https://www.npmjs.com/package/@ashqking/meteora-fee-router
2. **Test Installation**: `npm install @ashqking/meteora-fee-router`
3. **Update Documentation**: Add NPM installation instructions to README
4. **Create GitHub Release**: Tag and release the version on GitHub