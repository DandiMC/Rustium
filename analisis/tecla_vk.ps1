param(
    [string]$Title = "RubyDung",
    [int]$Vk = 0x31,
    [int]$Ms = 120
)
Add-Type -Namespace KB -Name V -MemberDefinition @'
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
'@
$proc = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $Title } | Select-Object -First 1
if (-not $proc) { Write-Output "NO-VENTANA"; exit 1 }
[KB.V]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 250
[KB.V]::keybd_event([byte]$Vk, 0, 0, 0)
Start-Sleep -Milliseconds $Ms
[KB.V]::keybd_event([byte]$Vk, 0, 2, 0)
Write-Output "VK $Vk"
