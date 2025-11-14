# NPM Package Setup & GitHub Release Guide

## 📦 NPM Package Status

**Package Name:** `@ashqking/meteora-fee-router`  
**Version:** 1.0.0  
**Registry:** https://www.npmjs.com/package/@ashqking/meteora-fee-router

### Current Setup

✅ Package published to NPM  
✅ Scoped package (@ashqking)  
✅ Public visibility set (`"publishConfig": { "access": "public" }`)  
✅ All source files included in package

---

## 🔍 Why Package Might Not Be Showing in GitHub Packages Tab

The NPM package is published to **npm.js registry** (public NPM), NOT GitHub Packages. Here are the reasons:

1. **GitHub Packages requires separate setup** - Different registry than npm.js
2. **Current setup publishes to npm.js** - Which is the right choice for public libraries
3. **GitHub Packages Tab** - Only shows packages published to GitHub's registry

### Solution Options:

#### Option A: Use npm.js (Recommended) ✅

**Advantages:**
- Largest package registry (most visibility)
- Most developers use npm.js by default
- Easiest to discover and install
- Standard for Solana ecosystem packages

**Installation:**
```bash
npm install @ashqking/meteora-fee-router
```

**View Package:**
- NPM Registry: https://www.npmjs.com/package/@ashqking/meteora-fee-router
- Search npm.js for "meteora-fee-router"

#### Option B: Publish to GitHub Packages (Optional)

If you want package visible in GitHub Packages tab, add to `.npmrc`:

```
@ashqking:registry=https://npm.pkg.github.com
```

**Pros:** GitHub integration, automatic with actions  
**Cons:** Most developers won't find it there

---

## 🚀 How to Use GitHub Releases

### Automatic Release Setup (Enabled Now)

The release workflow is now activated. To create a release:

### Step 1: Create a Git Tag

```bash
# Increment version in package.json (e.g., 1.0.0 -> 1.1.0)
npm version minor  # or patch, major

# This will:
# - Update package.json version
# - Create git tag (e.g., v1.1.0)
# - Commit the change

# Push to GitHub
git push origin main
git push origin --tags
```

### Step 2: GitHub Actions Auto-Creates Release

When you push a `v*` tag:

1. ✅ GitHub Actions workflow triggers
2. ✅ Builds the program
3. ✅ Creates GitHub Release with notes
4. ✅ Publishes to NPM (if NPM_TOKEN set)
5. ✅ Uploads build artifacts

### View Your Release

1. Go to GitHub repo
2. Click **Releases** tab
3. You'll see release with:
   - Release notes
   - Build artifacts (.so files)
   - IDL files

---

## 🔑 Setup for Automated NPM Publishing

To auto-publish to NPM when creating releases:

### Step 1: Create NPM Token

1. Go to https://www.npmjs.com/settings/~/tokens
2. Click "Generate New Token"
3. Choose "Automation" type
4. Copy token

### Step 2: Add GitHub Secret

1. Go to GitHub repo → Settings
2. Click **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `NPM_TOKEN`
5. Paste your NPM token

### Step 3: Test Release

```bash
# Create and push a test release
npm version patch  # Bumps 1.0.0 -> 1.0.1
git push origin main
git push origin --tags
```

Watch GitHub Actions → Release workflow execute

---

## 📋 Making Your Package Discoverable

### Option 1: Add to npm.js Trending (Recommended)

1. **Tweet about release** - Tag @npmjs
2. **Add to awesome-solana** - https://github.com/solana-developers/awesome
3. **Submit to registries** - Solana ecosystem lists
4. **Get featured** - Solana Foundation community calls

### Option 2: Improve Package Discovery

Update `package.json` keywords:

```json
{
  "keywords": [
    "solana",
    "anchor",
    "defi",
    "fee-distribution",
    "meteora",
    "streamflow",
    "damm-v2",
    "distribution",
    "vesting",
    "blockchain",
    "cryptocurrency"
  ]
}
```

### Option 3: Publish Release Notes

When creating release, include:

```markdown
# Version 1.1.0 - New Features

## Install

```bash
npm install @ashqking/meteora-fee-router@1.1.0
```

## Changes
- Feature 1
- Bug fix 1
- Documentation update

## Links
- [NPM Package](https://www.npmjs.com/package/@ashqking/meteora-fee-router)
- [GitHub](https://github.com/iamaanahmad/meteora-fee-router)
- [Documentation](https://github.com/iamaanahmad/meteora-fee-router/tree/main/docs)
```

---

## 📊 Release Checklist

Before creating a release:

- [ ] Update version in `package.json`
- [ ] Update `CHANGELOG.md` with new features
- [ ] Run full test suite: `npm run test:all`
- [ ] Update documentation if needed
- [ ] Commit and push to main
- [ ] Create git tag: `git tag v1.x.x && git push --tags`

After release:

- [ ] GitHub Release created automatically
- [ ] Package published to NPM
- [ ] Announce on Twitter/Discord
- [ ] Share in Solana/Meteora communities

---

## 🔗 Quick Links

- **NPM Package:** https://www.npmjs.com/package/@ashqking/meteora-fee-router
- **GitHub Releases:** https://github.com/iamaanahmad/meteora-fee-router/releases
- **Installation:** `npm install @ashqking/meteora-fee-router`

---

## ❓ FAQ

**Q: Why isn't my package in GitHub Packages?**  
A: Because it's published to npm.js registry instead. That's the right choice for public libraries.

**Q: How do I make it visible on GitHub?**  
A: The release appears on the **Releases** tab. That's how most projects show versions.

**Q: How do developers find my package?**  
A: They search npm.js or your GitHub repo. Add good keywords and documentation.

**Q: Can I publish to both npm.js and GitHub?**  
A: Yes, but not necessary. npm.js is better for discoverability.

**Q: When should I update the version?**  
A: 
- PATCH (1.0.1): Bug fixes
- MINOR (1.1.0): New features
- MAJOR (2.0.0): Breaking changes

---

**Last Updated:** November 14, 2025
