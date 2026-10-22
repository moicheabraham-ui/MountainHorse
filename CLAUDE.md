# Mountain Horse

(Formerly SftpDrive; internal ids like the keyring service `SftpDrive`, the app identifier and the ssh-config markers keep the old name so existing data keeps working.) Windows app (alternative to Mountain Duck) that mounts SFTP servers as drive letters. Tauri v2: Rust backend in `src-tauri/src/main.rs`, plain HTML/JS UI in `dist/index.html` (no bundler, `withGlobalTauri`).

## How mounting works
It does not implement SFTP. It shells out to `net use X: \\sshfs.r\user@host!port\path` (SSHFS-Win UNC syntax; `.k` = key auth from `%USERPROFILE%\.ssh`), which needs **WinFsp + SSHFS-Win** installed on the PC (`check_prereqs` shows a banner if missing). Unmount = `net use X: /delete /y`.

## Data
- Connections: `connections.json` in the app config dir. Passwords: Windows Credential Manager via the `keyring` crate (service `SftpDrive`, user = connection id), never in the JSON.
- host/user are validated (`clean`) because they are interpolated into the UNC path.

## Build
No Rust toolchain on the dev machine (Linux). GitHub Actions (`.github/workflows/build.yml`, windows-latest) builds the NSIS installer and uploads it as an artifact. Icons are generated in CI from `app-icon.png` (`src-tauri/icons/` is gitignored).

## Installer
The NSIS installer (perMachine, asks for admin) bundles the WinFsp and SSHFS-Win MSIs (downloaded in CI from their latest GitHub releases into `src-tauri/resources/`) and `installer-hooks.nsh` installs them silently if missing. Uninstall leaves them in place.

## Known gaps / ideas
No tray icon, no auto-mount at login, no SSH key picker, password passed to `net use` as an argument. Mount command syntax is untested on a real Windows machine.

## Icon
`assets/icon.svg` is the source (horse head on mountains, Mountain Duck style); `app-icon.png` (1024px) is rendered from it with `rsvg-convert -w 1024 -h 1024 assets/icon.svg -o app-icon.png`; CI builds all sizes from that.
