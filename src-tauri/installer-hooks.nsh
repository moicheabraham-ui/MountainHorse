; Installs WinFsp and SSHFS-Win (bundled MSIs) when missing. WinFsp first: SSHFS-Win depends on it.
!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$PROGRAMFILES32\WinFsp\bin\winfsp-x64.dll" winfsp_done
    DetailPrint "Instalando WinFsp..."
    ExecWait 'msiexec /i "$INSTDIR\resources\winfsp.msi" /qn /norestart'
  winfsp_done:
  IfFileExists "$PROGRAMFILES64\SSHFS-Win\bin\sshfs-win.exe" sshfs_done
    DetailPrint "Instalando SSHFS-Win..."
    ExecWait 'msiexec /i "$INSTDIR\resources\sshfs-win.msi" /qn /norestart'
  sshfs_done:
!macroend
