; womm installer. Installs to %AppData%\Womm. Unsigned, as is tradition.
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

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "womm"
  SetOutPath "$INSTDIR"
  File /r "${SRC}\*.*"
  CreateDirectory "$INSTDIR\bin"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  ; Put %AppData%\Womm and its bin folder on the user PATH (no admin needed).
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "$$p=[Environment]::GetEnvironmentVariable(''Path'',''User''); foreach($$d in @(''$INSTDIR'',''$INSTDIR\bin'')){ if(-not ($$p -split '';'' -contains $$d)){ $$p=($$p.TrimEnd('';'')+'';''+$$d).TrimStart('';'') } }; [Environment]::SetEnvironmentVariable(''Path'',$$p,''User'')"'
  DetailPrint "Installed. It worked on my machine."
SectionEnd

Section "Uninstall"
  RMDir /r "$INSTDIR"
SectionEnd
