!define VERSION 5.0.0
!define SRCDIR "..\..\target\release"

Name "OpenUDS Actor"
OutFile "OpenUDS-Managed_Installer-${VERSION}.exe"
InstallDir "$PROGRAMFILES\UDSActor"

Page Directory
Page InstFiles

Section
  SetOutPath $INSTDIR

  ; === Copy binaries from build dir ===
  File "${SRCDIR}\gui-helper.exe"
  File "${SRCDIR}\udsactor-client.exe"
  File "${SRCDIR}\udsactor-config.exe"
  File "${SRCDIR}\udsactor-service.exe"

  !define COMPANYNAME "OpenUDS"
  !define appName "actor_service.exe"
  !define displayName "UDS Actor Service"
  !define serviceName "UDSActorService"

  CreateDirectory "$SMPROGRAMS\${COMPANYNAME}"
  CreateShortCut "$SMPROGRAMS\${COMPANYNAME}\UDSActorConfig.lnk" "$INSTDIR\udsactor_config.exe" "" ""

  ExecWait '"$INSTDIR\udsactor_service.exe" --install'

  WriteUninstaller "$INSTDIR\uninstaller.exe"
SectionEnd

Section "Uninstall"
  ExecWait '"$INSTDIR\udsactor_service.exe" --uninstall'

  Delete "$SMPROGRAMS\${COMPANYNAME}\UDSActorConfig.lnk"
  Delete "$INSTDIR\gui-helper.exe"
  Delete "$INSTDIR\udsactor-client.exe"
  Delete "$INSTDIR\udsactor-config.exe"
  Delete "$INSTDIR\udsactor-service.exe"
  Delete "$INSTDIR\uninstaller.exe"

  RMDir /r "$INSTDIR"
SectionEnd
