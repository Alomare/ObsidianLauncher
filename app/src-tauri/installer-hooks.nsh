; Extra steps of the Windows installer (tauri.conf.json, bundle.windows.nsis.installerHooks).
;
; The product was called Obsidian Mod Manager until it became Obsidian Launcher. The installer keys the
; install folder, the Add or Remove Programs entry, the shortcuts and the install-location registry key
; by the product name, so an installer with the new name would put a second copy beside the old one and
; leave the old shortcuts opening the old copy. Before the new files go in, the old copy is removed by
; the uninstaller that came with it, run silently, and the shortcuts the old copy had are made again
; under the new name. The mod library, the profiles and the app data are not touched: they live outside
; the install folder, and the app moves them on the first start.

Var ObsOldDir
Var ObsOldUninstall
Var ObsOldDesktop
Var ObsOldMenu

!macro NSIS_HOOK_PREINSTALL
  StrCpy $ObsOldDesktop 0
  StrCpy $ObsOldMenu 0
  ReadRegStr $ObsOldDir SHCTX "Software\${MANUFACTURER}\Obsidian Mod Manager" ""
  ReadRegStr $ObsOldUninstall SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\Obsidian Mod Manager" "UninstallString"
  ${If} $ObsOldDir != ""
  ${AndIf} $ObsOldUninstall != ""
    ${If} ${FileExists} "$DESKTOP\Obsidian Mod Manager.lnk"
      StrCpy $ObsOldDesktop 1
    ${EndIf}
    ${If} ${FileExists} "$SMPROGRAMS\Obsidian Mod Manager.lnk"
      StrCpy $ObsOldMenu 1
    ${EndIf}
    ; The install folder goes last on the line and unquoted, as the uninstaller expects.
    ExecWait '$ObsOldUninstall /S _?=$ObsOldDir' $0
    ${If} $0 = 0
      ; Run in place, the uninstaller can't delete itself or the folder.
      Delete "$ObsOldDir\uninstall.exe"
      RMDir "$ObsOldDir"
      DeleteRegKey SHCTX "Software\${MANUFACTURER}\Obsidian Mod Manager"
    ${Else}
      StrCpy $ObsOldDesktop 0
      StrCpy $ObsOldMenu 0
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $ObsOldMenu = 1
  ${AndIfNot} ${FileExists} "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  ${EndIf}
  ${If} $ObsOldDesktop = 1
  ${AndIfNot} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
!macroend
