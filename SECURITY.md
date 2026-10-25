# Security Policy

## 🔒 Privacy & Architecture Principles

**FileDeck** is engineered with a strict **local-first, 100% offline security posture**:

1. **Zero External Data Transmission**: All document indexing, OCR processing, semantic vector calculations, and local AI queries execute strictly on `127.0.0.1` (localhost).
2. **Deterministic File Storage**: Documents and generated thumbnails are stored in your configured local directory without remote sync dependencies.
3. **No Telemetry**: FileDeck does not collect usage analytics, telemetry pings, or diagnostic telemetry.

---

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

---

## Reporting a Vulnerability

If you discover a security vulnerability or issue in FileDeck, please report it directly by opening an issue on GitHub:

- **Open a GitHub Issue**: [https://github.com/samolubukun/Filedeck/issues](https://github.com/samolubukun/Filedeck/issues)
- **Security Advisory**: You can also report via [GitHub Security Advisories](https://github.com/samolubukun/Filedeck/security/advisories)

### Details to Include

- Component affected (`backend`, `worker`, or `frontend`).
- Detailed steps to reproduce the issue.
- Expected behavior versus observed behavior.
- Any suggested fixes or mitigations.
