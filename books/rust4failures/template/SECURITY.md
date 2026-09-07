# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| latest  | ✅        |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities in
rust4failures.

Instead, report them via [GitHub's private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability)
for this repository.

Include as much detail as possible:

- A description of the vulnerability and its potential impact
- Steps to reproduce
- Any suggested mitigations

You can expect an acknowledgment within 48 hours and a resolution timeline once
the issue has been assessed.

## Dependency Security

This repository is generated. Its code and its manifest come from the book at
<https://github.com/ImperialBower/bower/tree/main/books/rust4failures>, so a fix
is made there and replayed here — a patch committed directly to this repository
is overwritten on the next replay.

`.github/workflows/CI.yaml` runs `cargo test`, `cargo clippy`, `cargo fmt` and
`cargo doc` on every push. To check locally:

```shell
cargo test --all
```
