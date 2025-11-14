# NPM Trusted Publisher Setup Guide

**Status:** Setup Instructions for OIDC-based NPM Publishing  
**Date:** November 14, 2025

---

## 📋 What You Need to Fill In

On the NPM Trusted Publisher setup page, use these exact values:

### Form Fields to Fill:

| Field | Value | Notes |
|-------|-------|-------|
| **Publisher** | `GitHub Actions` | Select from dropdown |
| **Organization or user** | `iamaanahmad` | Your GitHub username |
| **Repository** | `meteora-fee-router` | Your repo name |
| **Workflow filename** | `publish.yml` | The workflow file we just created |
| **Environment name** | `npm-publish` | ✅ **IMPORTANT** - See below |

---

## ✅ Step-by-Step Setup

### 1. **Fill NPM Trusted Publisher Form**

Go to: https://www.npmjs.com/settings/@ashqking/meteora-fee-router/access

**Fill in:**
```
Publisher: GitHub Actions
Organization or user: iamaanahmad
Repository: meteora-fee-router
Workflow filename: publish.yml
Environment name: npm-publish
```

Click **"Set up connection"**

---

### 2. **Setup GitHub Environment (Already Done ✅)**

The environment is automatically created by NPM setup, but verify it exists:

1. Go to: https://github.com/iamaanahmad/meteora-fee-router/settings/environments
2. You should see `npm-publish` environment
3. Click it to verify settings (no IP restrictions needed for OIDC)

---

### 3. **Workflow File Location (Already Done ✅)**

The workflow file `publish.yml` is already in:
```
.github/workflows/publish.yml
```

This triggers automatically when you create a GitHub Release.

---

## 🚀 How to Publish New Versions

### Option 1: Automated (Recommended)

1. **Update version in `package.json`:**
   ```bash
   npm version patch  # or minor/major
   ```

2. **Push to GitHub:**
   ```bash
   git push origin main --tags
   ```

3. **Create GitHub Release:**
   - Go to: https://github.com/iamaanahmad/meteora-fee-router/releases
   - Click "Releases" → "Draft a new release"
   - Use tag: `v1.0.1` (matches package.json version)
   - Fill in release notes
   - Click "Publish release"

4. **Automatic Publishing:**
   - GitHub Actions automatically publishes to NPM
   - Check: https://github.com/iamaanahmad/meteora-fee-router/actions
   - Verify on NPM: https://www.npmjs.com/package/@ashqking/meteora-fee-router

---

### Option 2: Manual Publishing (If Needed)

If automated fails, you can manually publish:

```bash
npm publish --access public
```

**Requirements:**
- Must be authenticated: `npm login`
- Must have publish permissions on @ashqking scope

---

## 🔒 Why OIDC is Better Than Tokens

### Traditional NPM Token Approach:
- ❌ Stores token as GitHub secret (security risk if exposed)
- ❌ Token never expires or is hard to rotate
- ❌ Anyone with repo write access can use token
- ❌ Token visible in GitHub secrets

### OIDC Approach (What We're Using):
- ✅ No long-lived tokens stored anywhere
- ✅ Each publish request generates unique short-lived token
- ✅ Only works from GitHub Actions in your repo
- ✅ Automatic token rotation
- ✅ Can restrict to specific workflow files
- ✅ Audit trail in NPM

---

## 📦 Package Settings

Your package is already configured correctly:

```json
{
  "name": "@ashqking/meteora-fee-router",
  "version": "1.0.0",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/iamaanahmad/meteora-fee-router.git"
  },
  "files": [
    "README.md",
    "LICENSE",
    "docs/",
    "programs/meteora-fee-router/src/",
    "programs/meteora-fee-router/Cargo.toml",
    "Anchor.toml",
    "package.json"
  ]
}
```

**What gets published to NPM:**
- ✅ README.md
- ✅ Documentation files
- ✅ Rust source code
- ✅ Package metadata
- ✅ Anchor config

---

## 🔍 Why Package Might Not Be Visible

If your package isn't appearing in search/sidebar, common reasons:

1. **Package is not yet published** - First publish triggers indexing (24-48 hours)
2. **Package name typo** - Double-check `@ashqking/meteora-fee-router`
3. **Package is private** - Verify `npm publish --access public` flag
4. **NPM search indexing lag** - Can take 24 hours to appear
5. **Package not linked in org** - Link in organization settings

### To Fix:

1. **Verify published:**
   ```bash
   npm view @ashqking/meteora-fee-router
   ```

2. **If not published, publish now:**
   ```bash
   npm publish --access public
   ```

3. **Link to organization:**
   - Go to: https://www.npmjs.com/settings/@ashqking/members
   - Add package to organization if needed

4. **Wait for indexing:**
   - Can take up to 48 hours to appear in search
   - Direct URL works immediately: https://www.npmjs.com/package/@ashqking/meteora-fee-router

---

## ✅ Verification Checklist

After completing setup:

- [ ] NPM Trusted Publisher configured with `publish.yml`
- [ ] GitHub environment `npm-publish` exists
- [ ] Workflow file `.github/workflows/publish.yml` committed
- [ ] Package version in `package.json` updated for next release
- [ ] README and docs are complete
- [ ] License file present (MIT)
- [ ] GitHub Release created with version tag
- [ ] NPM publish workflow completed successfully
- [ ] Package visible on https://www.npmjs.com/package/@ashqking/meteora-fee-router

---

## 🎯 Next Release Steps

When you're ready to release a new version:

1. Update `package.json` version
2. Create git tag: `git tag v1.0.1`
3. Push: `git push origin main --tags`
4. Go to GitHub Releases → Create release from tag
5. Fill in release notes
6. Publish release
7. GitHub Actions automatically publishes to NPM ✅

---

## 📞 Troubleshooting

**Q: Workflow shows error "Cannot publish with this token"**
- A: OIDC trust relationship not configured. Verify NPM settings match exactly.

**Q: "Package already published" error**
- A: Version in `package.json` must match release tag AND be higher than previous

**Q: Package still not visible after 48 hours**
- A: Check https://www.npmjs.com/package/@ashqking/meteora-fee-router directly
- May need to manually link in organization settings

**Q: Want to rollback a publish**
- A: NPM allows deprecating versions: `npm deprecate @ashqking/meteora-fee-router@1.0.0 "Use 1.0.1 instead"`

---

## 📚 References

- [NPM Trusted Publisher Docs](https://docs.npmjs.com/creating-and-viewing-access-tokens#via-openid-connect-oidc)
- [GitHub OIDC Docs](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
- [Publishing to NPM](https://docs.npmjs.com/packages-and-modules/contributing-packages-to-the-registry)

---

**Last Updated:** November 14, 2025  
**Status:** Ready for first automated release
