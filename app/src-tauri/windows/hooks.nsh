; Kuchu's own steps in its Windows installer, which installs for the current account only.
;
; Once the files are in place, `kuchu setup --no-start` puts the relay where the hooks point and adds Kuchu to this
; account's startup apps. The installer's last page offers to start Kuchu, and its first start opens Settings, where
; the person connects it to Claude Code after seeing the change.
;
; Kuchu's hooks run the copy of the relay in Kuchu's app data, not the one installed here, so an uninstall that keeps
; the app data leaves hooks that find no Kuchu and stay silent (and an upgrade keeps working). Deleting the app data
; removes that relay, so then `kuchu uninstall` must succeed first: it refuses while the hooks are still in Claude
; Code's settings.json (they are taken out in Kuchu's Settings, with the change shown first, which it opens), so no
; hook is ever left pointing at a program that is gone.

!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$INSTDIR\kuchu.exe" setup --no-start'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ExecWait '"$INSTDIR\kuchu.exe" uninstall' $0
    ${If} $0 <> 0
      MessageBox MB_ICONINFORMATION|MB_OK "Kuchu is still connected to Claude Code, so nothing was removed.$\r$\n$\r$\nIn Kuchu's Settings, which are now open, click Disconnect under Claude Code (Kuchu shows the change first). Then uninstall Kuchu again."
      Abort
    ${EndIf}
  ${EndIf}
!macroend
