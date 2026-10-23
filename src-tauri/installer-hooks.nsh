; Installs WinFsp and SSHFS-Win (bundled MSIs) when missing. WinFsp first: SSHFS-Win depends on it.
; The installer is 32-bit, so Sysnative gives the 64-bit msiexec. $PROGRAMFILES32 resolves to the 64-bit
; folder because of SetRegView 64, so the real "Program Files (x86)" comes from the environment.
; msiexec exit codes: 0 ok, 1638 already installed, 3010 ok but reboot needed; anything else is reported.
!macro NSIS_HOOK_POSTINSTALL
  StrCpy $R8 "msiexec"
  IfFileExists "$WINDIR\Sysnative\msiexec.exe" 0 +2
  StrCpy $R8 "$WINDIR\Sysnative\msiexec.exe"
  ReadEnvStr $R7 "ProgramFiles(x86)"
  StrCmp $R7 "" 0 +2
  StrCpy $R7 "C:\Program Files (x86)"

  IfFileExists "$R7\WinFsp\bin\winfsp-x64.dll" winfsp_done
    DetailPrint "Instalando WinFsp..."
    ExecWait '"$R8" /i "$INSTDIR\resources\winfsp.msi" /qn /norestart /l*v "$TEMP\MountainHorse-winfsp.log"' $R9
    StrCmp $R9 0 winfsp_done
    StrCmp $R9 1638 winfsp_done
    StrCmp $R9 3010 winfsp_done
    MessageBox MB_OK|MB_ICONEXCLAMATION "No se pudo instalar WinFsp (codigo $R9). Las unidades no se podran montar.$\r$\nRegistro: $TEMP\MountainHorse-winfsp.log"
  winfsp_done:

  IfFileExists "$PROGRAMFILES64\SSHFS-Win\bin\sshfs-win.exe" sshfs_done
    DetailPrint "Instalando SSHFS-Win..."
    ExecWait '"$R8" /i "$INSTDIR\resources\sshfs-win.msi" /qn /norestart /l*v "$TEMP\MountainHorse-sshfs-win.log"' $R9
    StrCmp $R9 0 sshfs_done
    StrCmp $R9 1638 sshfs_done
    StrCmp $R9 3010 sshfs_done
    MessageBox MB_OK|MB_ICONEXCLAMATION "No se pudo instalar SSHFS-Win (codigo $R9). Las unidades no se podran montar.$\r$\nRegistro: $TEMP\MountainHorse-sshfs-win.log"
  sshfs_done:
!macroend
