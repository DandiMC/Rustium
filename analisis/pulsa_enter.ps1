param([string]$Title = "^Game$")
Add-Type -Namespace KB -Name E -MemberDefinition @'
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
'@
$proc = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $Title } | Select-Object -First 1
if (-not $proc) { Write-Output "NO-VENTANA"; exit 1 }
[KB.E]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 300
[KB.E]::keybd_event(0x0D, 0, 0, 0)
Start-Sleep -Milliseconds 120
[KB.E]::keybd_event(0x0D, 0, 2, 0)
Write-Output "ENTER"
