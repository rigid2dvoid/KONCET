# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| `main` | Development only |
| `0.1.x` | Not yet released |

KONCET has no production release yet. This table will be updated at the first
public release. Security fixes will target the latest released minor version.

## Reporting a vulnerability

Please use **GitHub private vulnerability reporting** for this repository. Do not
disclose the issue publicly until a fix or mitigation is available.

Include:

- affected version or commit;
- steps to reproduce;
- expected and actual behaviour;
- impact, especially any path that could escape the simulation sandbox, corrupt
  local data, or execute code;
- relevant logs or screenshots with personal data removed.

There is no bug bounty. Good-faith reports will be acknowledged in the release notes
if the reporter wishes to be credited.

## Scope and security model

KONCET is an offline desktop simulator. It does not execute project source code,
start real containers, bind network ports, or call external services. The assets to
protect are local project data, the application's SQLite database, and the integrity
of the host.

The threat model, trust boundaries, and mitigations are documented in
[docs/architecture/security.md](docs/architecture/security.md).

Particularly valuable reports include:

- escaping the virtual filesystem or writing outside the application data directory;
- code execution caused by a crafted project, scenario, log message, or database;
- FFI memory-safety issues or panics that cross the language boundary;
- SQL injection or database corruption paths;
- unexpected network access at runtime;
- dependency licence or supply-chain problems.

## Disclosure timeline

We aim to acknowledge reports within 7 days and to provide a remediation plan within
30 days. Timelines may vary for a volunteer-maintained project; we will communicate
in the advisory thread.
