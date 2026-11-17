param([string]$TitleMatch = "")

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Win {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, ref RECT r);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, ref RECT r);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[Win]::SetProcessDPIAware() | Out-Null

$p = Get-Process | Where-Object { $_.MainWindowTitle -like "*$TitleMatch*" } | Select-Object -First 1
if (-not $p) { exit 0 }
$h = $p.MainWindowHandle
$c = New-Object Win+RECT
$w = New-Object Win+RECT
[Win]::GetClientRect($h, [ref]$c) | Out-Null
[Win]::GetWindowRect($h, [ref]$w) | Out-Null
Write-Output ("{0}|{1}|cliente {2}x{3}|ventana {4}x{5}" -f $p.ProcessName, $p.MainWindowTitle, $c.Right, $c.Bottom, ($w.Right - $w.Left), ($w.Bottom - $w.Top))
