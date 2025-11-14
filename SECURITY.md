# Security Policy

## Supported Versions

| Version | Supported          |
|---------|------------------|
| 1.x     | ✅ Fully supported |
| 0.x     | ❌ No longer supported |

## Reporting Security Vulnerabilities

**DO NOT** report security vulnerabilities through public GitHub issues.

### Responsible Disclosure

If you discover a security vulnerability, please email:

**security@ashqking.dev** (or your contact email)

Please include:

1. **Description** - What is the vulnerability?
2. **Location** - File path, line numbers, or code snippet
3. **Severity** - Critical, High, Medium, Low
4. **Steps to reproduce** - How to trigger the vulnerability
5. **Impact** - What could be affected?
6. **Suggested fix** (optional) - Your proposed solution

### Response Timeline

- **Acknowledgment**: Within 24 hours
- **Initial assessment**: Within 48 hours  
- **Fix development**: 1-2 weeks (depending on severity)
- **Release**: On next patch version
- **Disclosure**: Coordinated with reporter

## Security Best Practices

### For Users

1. **Keep updated** - Use latest versions
2. **Audit code** - Review source before production use (it's open source!)
3. **Report issues** - Follow responsible disclosure above
4. **Use mainnet carefully** - Test on devnet first

### For Developers

1. **No secrets in code** - Never commit private keys, passwords
2. **Input validation** - Validate all external inputs
3. **Overflow protection** - Use checked math operations
4. **Access control** - Verify wallet signatures and permissions
5. **Test coverage** - Maintain 100% test coverage

## Known Limitations

- Solana program is not formally audited (community audit completed)
- Use at your own risk on mainnet
- Always test on devnet first

## Security Audit

The program has been reviewed for common Solana vulnerabilities:
- See [docs/SECURITY_AUDIT_SUMMARY.md](./docs/SECURITY_AUDIT_SUMMARY.md) for details

## Questions?

- Check [docs/TROUBLESHOOTING_GUIDE.md](./docs/TROUBLESHOOTING_GUIDE.md)
- Open a GitHub Discussion
- Review [docs/OPERATIONAL_PROCEDURES.md](./docs/OPERATIONAL_PROCEDURES.md)

---

**Last Updated:** November 14, 2025
