param(
    [string]$TitleMatch = "Rustium",
    [int]$X = 800,
    [int]$Y = 600,
    [int]$Delta = -600
)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Rueda {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, int d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
}
"@
[Rueda]::SetProcessDPIAware() | Out-Null
$p = Get-Process | Where-Object { $_.MainWindowTitle -like "*$TitleMatch*" } | Select-Object -First 1
if (-not $p) { Write-Output "rueda: no-window"; exit 1 }
$h = $p.MainWindowHandle
[Rueda]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 250
$pt = New-Object Rueda+POINT
$pt.X = $X
$pt.Y = $Y
[Rueda]::ClientToScreen($h, [ref]$pt) | Out-Null
[Rueda]::SetCursorPos($pt.X, $pt.Y) | Out-Null
Start-Sleep -Milliseconds 150
[Rueda]::mouse_event(0x0800, 0, 0, $Delta, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 400
Write-Output "rueda $Delta"
