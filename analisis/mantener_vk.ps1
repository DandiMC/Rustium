param(
    [string]$Title = "RubyDung",
    [int]$Vk = 0x53,
    [int]$Ms = 900
)
Add-Type -Namespace KBM -Name V -MemberDefinition @'
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
'@
$proc = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $Title } | Select-Object -First 1
if (-not $proc) { Write-Output "NO-VENTANA"; exit 1 }
[KBM.V]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 250
[KBM.V]::keybd_event([byte]$Vk, 0, 0, 0)
Start-Sleep -Milliseconds $Ms
[KBM.V]::keybd_event([byte]$Vk, 0, 2, 0)
Write-Output "VK $Vk durante $Ms ms"
