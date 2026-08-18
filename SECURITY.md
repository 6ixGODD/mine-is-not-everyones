# Security Policy

## Supported Versions

MINE is currently in early development. Security fixes are provided for the latest released version only.

| Version | Supported |
|---|---|
| Latest release | :white_check_mark: |
| Older releases | :x: |

## Reporting a Vulnerability

Please do not open a public GitHub issue for a suspected security vulnerability.

Use GitHub's private vulnerability reporting feature for this repository when available.

Please include, where possible:

- the affected MINE version;
- your operating system;
- the Agent client involved, if applicable;
- a minimal reproduction;
- the expected and observed behavior;
- the security impact;
- relevant logs or configuration with secrets and personal information removed.

Security-sensitive areas include, but are not limited to:

- unintended Git or filesystem mutations;
- command execution outside the intended repository scope;
- bypasses of MINE's authority or repository-governance boundaries;
- unsafe handling of Agent configuration, credentials, or secrets;
- unintended or malicious Skill or MCP behavior;
- update, bootstrap, binary, or release-integrity issues;
- operations that could modify or destroy unrelated user work.

MINE is intentionally capable of modifying repositories, Git state, and supported Agent configuration under its documented workflow and authorization model. Behavior that is explicitly documented and authorized by MINE is not, by itself, considered a security vulnerability.

I will acknowledge valid reports as soon as practical and will investigate them privately. When appropriate, fixes and coordinated disclosure will be handled before public discussion.

If a report is determined not to be a security vulnerability, I will explain the reasoning where practical.
