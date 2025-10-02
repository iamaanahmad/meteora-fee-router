# 🤝 Contributing to Meteora Fee Router

We're thrilled that you're interested in contributing to the Meteora Fee Router! This document provides guidelines and information for contributors.

## 🌟 How to Contribute

### 🐛 Reporting Bugs

Before creating bug reports, please check the existing issues to avoid duplicates. When creating a bug report, include:

- **Clear description** of the issue
- **Steps to reproduce** the behavior  
- **Expected vs actual behavior**
- **Environment details** (Solana version, Anchor version, etc.)
- **Relevant logs** or error messages

### 💡 Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion:

- **Use a clear, descriptive title**
- **Provide detailed description** of the proposed feature
- **Explain why this enhancement would be useful**
- **Consider the scope** and impact on existing functionality

### 🔄 Pull Requests

1. **Fork** the repository
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Make your changes** with proper documentation
4. **Add tests** for new functionality
5. **Run the test suite** (`npm run test:all`)
6. **Commit your changes** (`git commit -m 'Add amazing feature'`)
7. **Push to the branch** (`git push origin feature/amazing-feature`)
8. **Open a Pull Request**

## 🧪 Development Setup

### Prerequisites

- **Rust** 1.70+ with `cargo`
- **Node.js** 16+ with `npm`
- **Solana CLI** 1.16+
- **Anchor CLI** 0.29.0+

### Local Development

```bash
# Clone your fork
git clone https://github.com/iamaanahmad/meteora-fee-router.git
cd meteora-fee-router

# Install dependencies
npm install

# Build the program
anchor build

# Run tests
npm run test:all
```

## 📋 Coding Standards

### Rust Code

- Follow **standard Rust formatting** (`cargo fmt`)
- Use **meaningful variable names**
- Add **comprehensive comments** for complex logic
- Include **error handling** with proper error types
- Write **unit tests** for all new functions

### TypeScript Code

- Follow **ESLint configuration**
- Use **TypeScript strictly** (no `any` types)
- Add **JSDoc comments** for public functions
- Write **integration tests** for new features

### Testing Requirements

- **All new code must have tests**
- **Maintain 100% test coverage** where possible
- **Test edge cases** and error conditions
- **Include performance tests** for critical paths

## 🔒 Security Guidelines

- **Never commit private keys** or sensitive data
- **Use secure coding practices**
- **Validate all inputs** thoroughly
- **Follow principle of least privilege**
- **Report security issues privately** first

## 📚 Documentation

- **Update README** for significant changes
- **Add inline code documentation**
- **Update integration examples** if APIs change
- **Include migration guides** for breaking changes

## 🏷️ Commit Message Format

We follow the [Conventional Commits](https://conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (no logic changes)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Examples

```
feat(distribution): add daily cap enforcement
fix(math): resolve overflow in batch calculations
docs(readme): update installation instructions
test(security): add comprehensive audit tests
```

## 🎯 Development Focus Areas

### High Priority

- **Security enhancements**
- **Performance optimizations**
- **Error handling improvements**
- **Documentation updates**

### Medium Priority

- **Additional test coverage**
- **Code refactoring**
- **Developer experience improvements**
- **Integration examples**

### Low Priority

- **Code style improvements**
- **Non-critical optimizations**
- **Additional tooling**

## 🧑‍💼 Code Review Process

1. **Automated checks** must pass (tests, linting)
2. **At least one approval** from a maintainer
3. **Security review** for sensitive changes
4. **Performance impact** assessment
5. **Documentation completeness** check

## 📞 Getting Help

- **GitHub Issues**: For bugs and feature requests
- **Discussions**: For questions and general discussion
- **Documentation**: Check the [docs/](docs/) directory
- **Examples**: See [docs/INTEGRATION_EXAMPLES.md](docs/INTEGRATION_EXAMPLES.md)

## 🎉 Recognition

Contributors will be:

- **Listed in CONTRIBUTORS.md**
- **Mentioned in release notes**
- **Credited in documentation**
- **Invited to future project discussions**

## 📄 License

By contributing, you agree that your contributions will be licensed under the same [MIT License](LICENSE) as the project.

---

**Thank you for contributing to Meteora Fee Router! 🚀**

*Together, we're building the future of automated fee distribution on Solana.*