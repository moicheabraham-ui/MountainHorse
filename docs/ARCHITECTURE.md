# Architecture

- **UI** (`dist/index.html`): plain HTML/CSS/JS with no bundler. It talks to the backend through Tauri's global `invoke` (`withGlobalTauri`).
- **Backend** (`src-tauri/src/main.rs`, Rust, Tauri v2):
  - `connections.json` in the app config dir holds the connection list. Passwords live in Windows Credential Manager via the `keyring` crate (service `SftpDrive`, user = connection id).
  - `mount_conn` starts `sshfs.exe` (from SSHFS-Win) in the foreground, one process per connection, with `-ouid=-1,gid=-1 -oidmap=user`. The password is written to its stdin (`-opassword_stdin`). The app waits up to 30 s for the drive letter to appear or for the process to exit, and reports `sshfs` errors.
  - `unmount_conn` kills the tracked process. Mounts made by an older version (via `net use`) are removed with `net use X: /delete`.
  - Tray icon with a menu per connection, auto-mount thread, autostart through `HKCU\...\Run`.
  - `host` and `user` are validated because they are placed in the sshfs target.
- **Installer** (NSIS, per-machine): bundles the WinFsp and SSHFS-Win MSIs and installs them silently when missing (`src-tauri/installer-hooks.nsh`), checking the `msiexec` exit codes and logging to `%TEMP%`.
- **CI** (`.github/workflows/build.yml`): on Windows, generates icons, downloads the latest WinFsp and SSHFS-Win MSIs, runs `tauri build`, uploads the installer and, for `v*` tags, creates a GitHub release.

## Why not `net use \\sshfs.r\...`?

That path goes through the WinFsp launcher, which runs `sshfs-win svc` and maps the remote owner to a SID that does not match the logged-in Windows user on some accounts, so your own files appear read-only. Starting `sshfs.exe` directly with `uid=-1,gid=-1` maps them to the current user.
