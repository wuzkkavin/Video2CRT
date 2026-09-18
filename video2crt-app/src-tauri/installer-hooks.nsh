; Installer-only model choice.  The running application never displays a
; model selector; this prompt appears after files are copied by NSIS.
!macro NSIS_HOOK_POSTINSTALL
  MessageBox MB_ICONQUESTION|MB_YESNO \
    "要在安裝時一併下載高品質模型嗎？\r\n\r\n需要網路與約 3.2 GB 磁碟空間；選否仍可使用內建標準模型。" \
    IDYES install_high_quality_model IDNO model_choice_done
  install_high_quality_model:
    DetailPrint "正在安裝高品質模型（約 3.2 GB）..."
    nsExec::ExecToLog '"$INSTDIR\runtime\tools\python\python.exe" "$INSTDIR\runtime\app\install-large-model.py" "$LOCALAPPDATA\Video2CRT\models\large"'
    Pop $0
    ${If} $0 != 0
      MessageBox MB_ICONEXCLAMATION|MB_OK \
        "高品質模型安裝失敗（錯誤碼 $0）。\r\n\r\n程式仍可使用內建標準模型，稍後可重新執行安裝程式。"
    ${Else}
      DetailPrint "高品質模型安裝完成。"
    ${EndIf}
  model_choice_done:
!macroend
