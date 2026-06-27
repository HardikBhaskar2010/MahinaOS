# Security Policy

## Supported Versions

Currently, Mahina OS is in early active development (v0.x). We are rapidly iterating on the architecture and core foundation. 

Therefore, **we do not provide long-term security support or guaranteed patching for pre-1.0 releases.**

| Version | Supported          | Description |
| ------- | ------------------ | ----------- |
| 1.0.x   | ✅ Planned          | The first stable release. Will receive full security updates. |
| 0.x.x   | ❌ Best effort      | Alpha/Beta releases. We will patch critical bugs but offer no guarantees. |

## Reporting a Vulnerability

If you discover a security vulnerability in Mahina OS, **please do NOT report it via public GitHub issues.** Public disclosure of 0-days before a patch is available puts all users at risk.

Instead, please contact the lead maintainer privately. 

**Provide the following details in your report:**
- The specific version or commit hash of Mahina OS you are testing.
- A detailed description of the vulnerability and its potential impact (e.g., Privilege Escalation in `luna-init`, buffer overflow in TOML parser).
- Step-by-step instructions to reproduce the issue.
- Expected behavior vs. actual behavior.
- Any relevant logs, kernel panics, or crash dumps.

We will acknowledge your report within 48 hours, verify the vulnerability, and provide a timeline for a patch. Once the patch is merged and released, we will publicly disclose the vulnerability and credit you for the discovery.
