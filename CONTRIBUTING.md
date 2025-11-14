# Contributing to Meteora Fee Router

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Getting Started

### Prerequisites

- **Rust**: 1.75.0+ (install via [rustup](https://rustup.rs/))
- **Node.js**: 18.17.0+ (use [nvm](https://github.com/nvm-sh/nvm))
- **Solana CLI**: 1.16.0+
- **Anchor**: 0.29.0

### Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/iamaanahmad/meteora-fee-router.git
cd meteora-fee-router

# Install dependencies
npm install

# Build the project
anchor build

# Run tests (should see 295 tests pass)
npm run test:all
```

## How to Contribute

### Reporting Issues

1. **Check existing issues** - Search to avoid duplicates
2. **Provide context** - Include:
   - Reproduction steps
   - Expected behavior
   - Actual behavior
   - Environment details (OS, versions)
3. **Use labels** - Mark as bug, enhancement, question, etc.

### Security Issues

**Do not** create public GitHub issues for security vulnerabilities.  
See [SECURITY.md](./SECURITY.md) for responsible disclosure.

### Submitting Pull Requests

1. **Fork and branch** - Create feature branch from `main`
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make changes** - Keep commits focused and atomic
   ```bash
   git commit -m "feat: Brief description of change"
   ```

3. **Test thoroughly** - All tests must pass
   ```bash
   npm run test:all
   npm run lint
   npm run format:check
   ```

4. **Update documentation** - Add docs for new features/changes

5. **Push and submit PR**
   ```bash
   git push origin feature/your-feature-name
   ```

6. **PR requirements**:
   - Clear description of changes
   - Reference related issues (#123)
   - All tests passing
   - Code formatted and linted

## Development Guidelines

### Code Style

- Follow Rust conventions (rustfmt enforces this)
- Use TypeScript for integration tests
- Add comments for complex logic
- Keep functions focused and small

### Testing

- All changes require tests
- Run full test suite before PR: `npm run test:all`
- Add integration tests in `tests/` directory
- Aim for 100% code coverage

### Commits

- Use conventional commits: `type(scope): description`
- Types: feat, fix, docs, test, refactor, chore
- Example: `feat(distribution): add pagination support`

### Documentation

- Update relevant docs when changing functionality
- Add code comments for non-obvious logic
- Update README.md for user-facing changes
- Keep docs in `docs/` directory

## Review Process

1. **Automated checks** - CI/CD pipeline runs tests and linting
2. **Code review** - Maintainers review for quality and alignment
3. **Feedback** - Address any requested changes
4. **Merge** - Once approved, PR is merged to main

## Release Process

Releases follow semantic versioning (MAJOR.MINOR.PATCH):

1. **Version bump** - Update `package.json` version
2. **Create tag** - Push git tag (e.g., `v1.1.0`)
3. **GitHub release** - Automated via GitHub Actions
4. **NPM publish** - Automated deployment to npm

See [CHANGELOG.md](./CHANGELOG.md) for version history.

## Questions?

- **Issues** - Use GitHub Issues for questions
- **Discussions** - Use GitHub Discussions for broader topics
- **Discord** - Join Solana/Meteora communities for real-time chat

## Code of Conduct

We welcome contributors from all backgrounds. Be respectful and constructive in all interactions.

---

**Thank you for contributing! 🙏**
