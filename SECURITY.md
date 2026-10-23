# Security

Mountain Horse is a vibe-coded app (see the README). This file records a security review of the code, what was fixed and what is still open. It is a self-review done with an AI assistant, **not** an independent audit.

## Reporting a vulnerability

Open a private security advisory on GitHub (Security > Report a vulnerability) or an issue without exploit details.

## Threat model

Mountain Horse runs as a normal user, stores SFTP connection details and passwords for that user, and starts `sshfs.exe` (SSHFS-Win) to mount a remote server. It has no network service of its own. The assets are the saved passwords, the SSH session itself, and the user's machine. The main attackers considered are: someone on the network between you and the server, a tampered download or build input, and anything able to edit the connection file or the settings that reach the `sshfs` command line.

## Findings

| # | Severity | Finding | Status |
|---|----------|---------|--------|
| 1 | Medium | Host keys were not verified (`StrictHostKeyChecking=no`, known_hosts at `/dev/null`), so a man-in-the-middle could capture the password or the session. | **Fixed.** Trust on first use: keys are stored in `%APPDATA%\online.moiche.sftpdrive\known_hosts` and a changed key is refused. Tested: a mismatching key makes ssh abort with "REMOTE HOST IDENTIFICATION HAS CHANGED". |
| 2 | Medium | Option injection into `sshfs`. `host`/`user` allowed `-` at the start and `,`, so a value like `-oProxyCommand=...` or `x,ProxyCommand=...` could add ssh options (command execution). The key path was also placed inside `-oIdentityFile=` where a `,` adds options. Needs control of the connection data (the form or `connections.json`), so it is local, but it was exploitable by a crafted or imported config. | **Fixed.** Strict whitelist for host and user (letters, digits, `. _ -`, no leading `-`), key path rejects `, " \n \r` and must exist, and the same validation runs again right before mounting because the JSON file is not trusted. |
| 3 | Medium | Bundled WinFsp and SSHFS-Win MSIs were downloaded from upstream "latest" in CI without any integrity check (supply chain). | **Fixed.** CI downloads pinned versions (WinFsp 2.1.25156, SSHFS-Win 3.5.20357) and verifies SHA-256 before building. |
| 4 | Low | No Content-Security-Policy (`csp: null`) and an unescaped id in an HTML attribute. XSS would need a tampered local file, but a CSP limits the damage. | **Fixed.** Restrictive CSP (self only, no remote scripts, no objects, no forms); ids are escaped. The UI was checked to still render and use IPC. |
| 5 | Low | `sshfs` stderr was piped but only read on failure; a full pipe could block the mount. | **Fixed.** stderr is drained after a successful mount. |
| 6 | Low | The installer is not code-signed, so Windows SmartScreen warns and users cannot verify the publisher. | Open. Needs a code-signing certificate. |
| 7 | Low | Passwords live in process memory as ordinary strings and are written to the child's stdin; they are not zeroized. They never appear on the command line, in the connection file or in logs. | Open (accepted). |
| 8 | Info | Passwords are stored with the `keyring` crate in Windows Credential Manager (per user, DPAPI). Other programs running as the same user can read them, as with any saved credential. | By design. |
| 9 | Info | SSH keys with a passphrase are not supported (there is no way to pass the passphrase). Unencrypted private keys on disk are only as safe as the file permissions. | By design. |
| 10 | Info | In-app updates (v0.1.4+) download `latest.json` and the installer from GitHub Releases. The updater only installs a package whose minisign signature matches the public key built into the app; the private key lives outside the repo (CI secret `TAURI_SIGNING_PRIVATE_KEY`). Losing the key means installed apps cannot be updated; a leaked key lets an attacker with release access push a malicious update. | By design; protect the key and the repository account (enable 2FA). |
| 11 | Info | GitHub Actions use version tags (`@v4`), not commit SHAs, and workflows only run on pushes to `main` and tags, never on pull requests, so repository secrets are not exposed to forks. | Open (hardening: pin actions by SHA). |

## Checked and found fine

- No secrets, tokens or private keys in the repository or its git history.
- Child processes are started with an argument list, never through a shell.
- Drive letters are validated as `A-Z`; `reg`/`net` calls take separate arguments.
- The webview only loads local files; Tauri IPC is not exposed to remote pages.

## Notes for users

- If a server's host key legitimately changes (reinstall), the mount fails with a host key error. Remove that server's line from `%APPDATA%\online.moiche.sftpdrive\known_hosts`, or delete the file, and mount again.
- The first connection to a server trusts whatever key it presents. Connect for the first time from a network you trust.
