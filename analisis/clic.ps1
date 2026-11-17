param(
    [string]$TitleMatch = "Rustium",
    [int]$X = 0,
    [int]$Y = 0
)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Clic {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
}
"@

[Clic]::SetProcessDPIAware() | Out-Null

$p = Get-Process | Where-Object { $_.MainWindowTitle -like "*$TitleMatch*" } | Select-Object -First 1
if (-not $p) { Write-Output "click: no-window"; exit 1 }
$h = $p.MainWindowHandle
[Clic]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 300

$pt = New-Object Clic+POINT
$pt.X = $X
$pt.Y = $Y
[Clic]::ClientToScreen($h, [ref]$pt) | Out-Null

[Clic]::SetCursorPos($pt.X, $pt.Y) | Out-Null
Start-Sleep -Milliseconds 150
[Clic]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 70
[Clic]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 200

Write-Output ("click cliente={0},{1} pantalla={2},{3}" -f $X, $Y, $pt.X, $pt.Y)
