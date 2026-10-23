# Mountain Horse

(Formerly SftpDrive; internal ids like the keyring service `SftpDrive`, the app identifier and the ssh-config markers keep the old name so existing data keeps working.) Windows app (alternative to Mountain Duck) that mounts SFTP servers as drive letters. Tauri v2: Rust backend in `src-tauri/src/main.rs`, plain HTML/JS UI in `dist/index.html` (no bundler, `withGlobalTauri`).

## How mounting works
It does not implement SFTP. For each connection it spawns `sshfs.exe` (SSHFS-Win, foreground, one child process per mount, tracked in `children()`) with `-ouid=-1,gid=-1 -oidmap=user -opassword_stdin` (password written to its stdin) and the drive letter as mountpoint; this needs **WinFsp + SSHFS-Win** installed (`check_prereqs` shows a banner if missing). We do NOT use `net use \sshfs.r\...`: through the WinFsp launcher the remote owner maps to SID `S-1-5-1-0`, which makes the user's own folders read-only on some accounts (e.g. Azure AD). Unmount = kill the child (fallback `net use X: /delete /y` for mounts from older versions). Quitting from the tray kills all children. SSHFS-Win reports a failed login as "System error 67".

## Data
- Connections: `connections.json` in the app config dir. Passwords: Windows Credential Manager via the `keyring` crate (service `SftpDrive`, user = connection id), never in the JSON.
- host/user are validated (`clean`) because they are placed in the sshfs target (`user@host:path`).

## Build
GitHub Actions (`.github/workflows/build.yml`, windows-latest) builds the NSIS installer and uploads it as an artifact. Icons are generated in CI from `app-icon.png` (`src-tauri/icons/` is gitignored).

## Installer
The NSIS installer (perMachine, asks for admin) bundles the WinFsp and SSHFS-Win MSIs (downloaded in CI from their latest GitHub releases into `src-tauri/resources/`) and `installer-hooks.nsh` installs them silently if missing. Uninstall leaves them in place.

## Known gaps / ideas
No tray icon, no auto-mount at login, no SSH key picker, password passed to `net use` as an argument. Mount command syntax is untested on a real Windows machine.

## Icon
`assets/icon.svg` is the source (horse head on mountains, Mountain Duck style); `app-icon.png` (1024px) is rendered from it with `rsvg-convert -w 1024 -h 1024 assets/icon.svg -o app-icon.png`; CI builds all sizes from that.
