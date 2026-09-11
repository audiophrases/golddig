@echo off
powershell -NoProfile -ExecutionPolicy Bypass -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('%~dp0Golddig.lnk'); $Shortcut.TargetPath = '%~dp0..\src-tauri\target\release\golddig.exe'; $Shortcut.WorkingDirectory = '%~dp0..'; $Shortcut.Description = 'Golddig - Fast Local Dictionary'; $Shortcut.IconLocation = '%~dp0..\src-tauri\icons\icon.ico, 0'; $Shortcut.Save()"
echo Shortcut Golddig.lnk recreated successfully.
pause
