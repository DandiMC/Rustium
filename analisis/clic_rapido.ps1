# Clic sintetico (izquierdo o derecho) sin la espera final de clic.ps1, para
# capturar efectos cortos (particulas). Uso:
#   .\clic_rapido.ps1 -Title "rd-20090515" -Boton right
param(
    [string]$Title = "rd-20090515",
    [ValidateSet("left", "right")]
    [string]$Boton = "left"
)

Add-Type -Namespace CR -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, int e);
'@

$proc = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $Title } | Select-Object -First 1
if (-not $proc) { Write-Output "NO-VENTANA"; exit 1 }
[CR.U]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 250

$abajo = if ($Boton -eq "right") { 0x0008 } else { 0x0002 }
$arriba = if ($Boton -eq "right") { 0x0010 } else { 0x0004 }
[CR.U]::mouse_event($abajo, 0, 0, 0, 0)
Start-Sleep -Milliseconds 40
[CR.U]::mouse_event($arriba, 0, 0, 0, 0)
Write-Output "CLIC($Boton)"
