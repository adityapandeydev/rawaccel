; Tauri's default uninstaller removes the product's Run value by name.
; Preserve a registration that belongs to a different RawAccel installation.
Var RawAccelOtherStartup

!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $RawAccelOtherStartup ""
  ${If} $UpdateMode <> 1
    Push $0
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RawAccel"
    ${If} $0 == '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --silent'
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RawAccel"
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "RawAccel"
    ${Else}
      StrCpy $RawAccelOtherStartup $0
    ${EndIf}
    Pop $0
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $RawAccelOtherStartup != ""
    Push $0
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RawAccel"
    ${If} $0 == ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RawAccel" $RawAccelOtherStartup
    ${EndIf}
    Pop $0
  ${EndIf}
  ; Settings, recovery files and the separately installed driver are retained.
!macroend
