# Fuerza el foco en la ventana del juego, pone el cursor en su centro, hace clic
# (para que agarre el cursor) y envia movimiento relativo de raton.
#   .\girar.ps1 -Title "^RubyDung" -dy -400        -> mira hacia arriba
#   .\girar.ps1 -Title "^Game"     -dy 400          -> mira hacia abajo
param(
    [string]$Title = "^RubyDung",
    [int]$dx = 0,
    [int]$dy = 0,
    [int]$veces = 1,
    [switch]$click
)

Add-Type -Namespace GW -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, int e);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
public struct RECT { public int Left, Top, Right, Bottom; }
'@

function Get-GameWindow([string]$pat) {
    $procs = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -match $pat }
    if (-not $procs) { return $null }
    return $procs[0]
}

$p = Get-GameWindow $Title
if (-not $p) { Write-Output "NO-VENTANA"; exit 1 }

$h = $p.MainWindowHandle
[GW.U]::ShowWindow($h, 9) | Out-Null      # SW_RESTORE
[GW.U]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 400

# Cursor al centro de la ventana
$r = New-Object GW.U+RECT
[GW.U]::GetWindowRect($h, [ref]$r) | Out-Null
$cx = [int](($r.Left + $r.Right) / 2)
$cy = [int](($r.Top + $r.Bottom) / 2)
[GW.U]::SetCursorPos($cx, $cy) | Out-Null
Start-Sleep -Milliseconds 150

if ($click) {
    [GW.U]::mouse_event(0x0002, 0, 0, 0, 0)
    Start-Sleep -Milliseconds 80
    [GW.U]::mouse_event(0x0004, 0, 0, 0, 0)
    Start-Sleep -Milliseconds 300
}

for ($i = 0; $i -lt $veces; $i++) {
    [GW.U]::mouse_event(0x0001, $dx, $dy, 0, 0)
    Start-Sleep -Milliseconds 25
}

$sb = New-Object System.Text.StringBuilder 256
[GW.U]::GetWindowText([GW.U]::GetForegroundWindow(), $sb, 256) | Out-Null
Write-Output "FOCO='$($sb.ToString())' centro=$cx,$cy"
