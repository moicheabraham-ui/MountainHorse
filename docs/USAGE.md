# Using Mountain Horse

## Connections

| Field | Meaning |
|-------|---------|
| Name | Label shown in the list and in the tray menu. |
| Drive | Windows drive letter. Free letters are suggested. |
| Server / Port | Host name or IP of the SFTP (SSH) server, default port 22. |
| User / Password | SSH account. Leave the password empty when editing to keep the saved one. |
| Remote folder | Folder to show as the drive root, for example `/home/me`. Empty means the server root `/`. |
| Use SSH key | Log in with a private key instead of a password. Empty path means `%USERPROFILE%\.ssh\id_rsa`. Keys with a passphrase are not supported. |
| Mount automatically | Mounts this connection whenever Mountain Horse starts. |

Host and user cannot contain spaces or `\ / ! @`.

## Mounting

- **Mount** starts the connection. If the server does not answer within 30 seconds, the attempt is cancelled and the error is shown.
- **Unmount** stops it. Close any file that is open on the drive first.
- Closing the window hides it to the tray. Choose **Salir** in the tray menu to quit; this unmounts every drive.

## Start with Windows

The *Iniciar con Windows* switch adds the app to the current user's startup list (it starts hidden, in the tray). Combine it with *Mount automatically* to have your drives ready after login.

## Where data is stored

- Connection list: `%APPDATA%\online.moiche.sftpdrive\connections.json` (no passwords).
- Passwords: Windows Credential Manager, entries ending in `.SftpDrive`.
