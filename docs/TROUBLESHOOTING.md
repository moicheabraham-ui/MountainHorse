# Troubleshooting

## "System error 67 has occurred. The network name cannot be found."

SSHFS-Win reports this when the SSH login fails or the server cannot be reached, not only when it is missing.

1. Re-enter the password: edit the connection, type it again, save.
2. Check that the host is reachable on its SSH port (`Test-NetConnection host -Port 22`).
3. Check that the user can log in with `ssh user@host`.

## The banner says WinFsp or SSHFS-Win is missing

The installer installs both. If it showed an error, read the logs in `%TEMP%\MountainHorse-winfsp.log` and `%TEMP%\MountainHorse-sshfs-win.log`. You can also install them yourself from [WinFsp](https://github.com/winfsp/winfsp/releases) and [SSHFS-Win](https://github.com/winfsp/sshfs-win/releases), then restart Windows.

## Access denied on a folder / cannot write

Versions before 0.1.3 mounted through `net use`, which can map the remote owner to an unknown Windows user. Update to the latest release. If a folder is still denied, check its permissions on the server: you need the same rights as with a normal `sftp` login.

## A drive stays after the app crashed

Remove it with `net use Z: /delete /y` (replace `Z`) or end the `sshfs.exe` process in Task Manager.

## Windows Explorer shows "You do not have permission to access \\sshfs.r\..."

Open the drive letter (`Z:`), not the `\\sshfs.r\...` address. Explorer connects to that address with your Windows login, not with the SSH password.
