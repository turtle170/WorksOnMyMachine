; womm installer. Installs to %AppData%\Womm for the current user only.
Unicode true
RequestExecutionLevel user
!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SRC
  !define SRC "stage"
!endif
!ifndef OUT
  !define OUT "womm-setup.exe"
!endif

Name "womm ${VERSION}"
OutFile "${OUT}"
InstallDir "$APPDATA\Womm"
ShowInstDetails show

; Version metadata (SignPath requires product name and version on signed files).
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "womm"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "womm installer"
VIAddVersionKey "CompanyName" "turtle170"
VIAddVersionKey "LegalCopyright" "MIT License"

; Announce every system change before making it.
PageEx license
  LicenseText "Before we begin, here is everything this installer will change." "Install"
  LicenseData "changes.txt"
PageExEnd
Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

!macro EditUserPath ACTION
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "$$dirs=@(''$INSTDIR'',''$INSTDIR\bin''); $$p=[Environment]::GetEnvironmentVariable(''Path'',''User''); $$parts=@(($$p -split '';'') | Where-Object { $$_ -and ($$dirs -notcontains $$_) }); if (''${ACTION}'' -eq ''add'') { $$parts += $$dirs }; [Environment]::SetEnvironmentVariable(''Path'',($$parts -join '';''),''User'')"'
!macroend

Section "womm"
  SetOutPath "$INSTDIR"
  File /r "${SRC}\*.*"
  CreateDirectory "$INSTDIR\bin"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  !insertmacro EditUserPath add
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm" "DisplayName" "womm"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm" "Publisher" "turtle170"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm" "NoRepair" 1
  DetailPrint "Installed. It worked on my machine."
SectionEnd

Section "Uninstall"
  !insertmacro EditUserPath remove
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\womm"
  RMDir /r "$INSTDIR"
SectionEnd
