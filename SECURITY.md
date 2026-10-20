# Security Policy

PlayRewire runs learner code on learners' own devices, so the security of the **local lab sandbox** matters to us as much as the security of our services.

## Reporting a vulnerability

**Please don't report security vulnerabilities through public GitHub issues.**

Instead, use one of these:

- [GitHub private vulnerability reporting](https://github.com/iemafzalhassan/playrewire/security/advisories/new) (preferred)
- Email **iemafzalhassan@gmail.com** with the subject `[SECURITY] PlayRewire`

Please include:

- the affected component (`lab-runner`, `guard-agent`, a service name, etc.) and version or commit,
- steps to reproduce or a proof of concept,
- the impact you think it has.

## What to expect

| Step | Target |
|---|---|
| Acknowledgement | within 3 business days |
| Initial assessment | within 7 days |
| Fix or mitigation plan | depends on severity, shared with you |
| Public advisory | after a fix is available, crediting you unless you'd rather stay anonymous |

## Especially in scope

- Escaping the local lab sandbox (filesystem, network, process, or resource-limit bypass)
- Bypassing session-guard or exam mode
- Forging or tampering with Open Badges credentials
- Reading hidden tests before an attempt
- Exposing personal data or learning analytics

## Supported versions

PlayRewire is pre-release. Until 1.0, only the latest `main` is supported.
