# Mantiene una tecla pulsada durante N ms dentro de la ventana activa.
#   .\teclas.ps1 -Title "^RubyDung" -Key w -Ms 1200
param(
    [string]$Title = "^RubyDung",
    [string]$Key = "w",
    [int]$Ms = 1000
)

Add-Type -Namespace KB -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
'@

$proc = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $Title } | Select-Object -First 1
if (-not $proc) { Write-Output "NO-VENTANA"; exit 1 }
[KB.U]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 300

$vk = @{ w = 0x57; a = 0x41; s = 0x53; d = 0x44; space = 0x20; r = 0x52 }[$Key.ToLower()]
if (-not $vk) { Write-Output "TECLA-DESCONOCIDA"; exit 1 }

# KEYEVENTF_KEYUP = 2
[KB.U]::keybd_event([byte]$vk, 0, 0, 0)
Start-Sleep -Milliseconds $Ms
[KB.U]::keybd_event([byte]$vk, 0, 2, 0)
Write-Output "PULSADA $Key durante $Ms ms"
