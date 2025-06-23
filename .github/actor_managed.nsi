!define VERSION 1.0.0

Name "Actor Managed"
OutFile "openUDS-Managed_Installer-${VERSION}.exe"
InstallDir "$PROGRAMFILES\UDSActor"
Page Directory
Page InstFiles
Section
  SetOutPath $INSTDIR
  File actor_client.exe
  File actor_config.exe
  File actor_service.exe

  !define COMPANYNAME "OpenUDS"

  !define appName "actor_service.exe"
  !define displayName "UDS Actor Service"
  !define serviceName "UDSActorService"

  createDirectory "$SMPROGRAMS\${COMPANYNAME}"
  createShortCut "$SMPROGRAMS\${COMPANYNAME}\UDSActorConfig.lnk" "$INSTDIR\actor_config.exe" "" ""

  ExecWait 'sc create ${serviceName} error= "severe" displayname= "${displayName}" type= "own" start= "auto" binpath= "$INSTDIR\${appName}"'

  WriteUninstaller $INSTDIR\uninstaller.exe
SectionEnd

Section "Uninstall"

Delete $INSTDIR\uninstaller.exe
 
RMDir /r $INSTDIR
SectionEnd
