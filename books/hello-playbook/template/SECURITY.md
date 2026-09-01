# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| latest  | ✅        |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities in hello-playbook.

Instead, report them via [GitHub's private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability) for this repository.

Include as much detail as possible:

- A description of the vulnerability and its potential impact
- Steps to reproduce
- Any suggested mitigations

You can expect an acknowledgment within 48 hours and a resolution timeline once the issue has been assessed.

## Dependency Security

`bin/security-scan` is the single source of truth for dependency and static-analysis checks in hello-playbook. It runs on every CI push via `security.yaml` (daily cron) and is also the `security-scan` target in `make ayce`. To check locally:

```shell
make security-scan
```
