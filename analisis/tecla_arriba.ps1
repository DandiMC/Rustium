param(
    [string]$Title = "RubyDung RS$",
    [int]$Ms = 900
)
Add-Type -Namespace KB -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
'@
$proc = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $Title } | Select-Object -First 1
if (-not $proc) { Write-Output "NO-VENTANA"; exit 1 }
[KB.U]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 300
[KB.U]::keybd_event(0x26, 0, 0, 0)
Start-Sleep -Milliseconds $Ms
[KB.U]::keybd_event(0x26, 0, 2, 0)
Write-Output "FLECHA ARRIBA $Ms ms"
