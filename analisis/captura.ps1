<#
Captura la ventana de un juego nativo (Minecraft original o RubyDung RS) a PNG,
para poder comparar visualmente el render de ambos.

Uso:
  # ver titulos de ventanas abiertas
  powershell -NoProfile -ExecutionPolicy Bypass -File analisis/captura.ps1 -List

  # capturar la ventana que contenga "Minecraft" (espera hasta 60s a que aparezca)
  powershell -NoProfile -ExecutionPolicy Bypass -File analisis/captura.ps1 -Out analisis/capturas/orig_01.png -Timeout 60

  # capturar la ventana en primer plano
  powershell -NoProfile -ExecutionPolicy Bypass -File analisis/captura.ps1 -Foreground -Out analisis/capturas/x.png
#>
param(
    [string]$Out = "analisis/capturas/captura.png",
    [string]$TitleMatch = "Minecraft|RubyDung",
    [string]$Exclude = "Launcher|Installer",
    [int]$Timeout = 0,
    [switch]$Foreground,
    [switch]$Bring,
    [switch]$FullScreen,
    [switch]$WithFrame,
    [switch]$List
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class WinCap {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT p);
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);
    public static string Title(IntPtr h) {
        var sb = new StringBuilder(512);
        GetWindowTextW(h, sb, sb.Capacity);
        return sb.ToString();
    }
}
"@

# Coordenadas fisicas, para que coincidan con lo que se ve aunque Windows use escalado DPI
[void][WinCap]::SetProcessDPIAware()

function Get-VisibleWindows {
    $list = New-Object System.Collections.ArrayList
    $cb = [WinCap+EnumProc]{
        param($h, $l)
        if ([WinCap]::IsWindowVisible($h)) {
            $t = [WinCap]::Title($h)
            if ($t) { [void]$list.Add([pscustomobject]@{ Handle = $h; Title = $t }) }
        }
        return $true
    }
    [void][WinCap]::EnumWindows($cb, [IntPtr]::Zero)
    return $list
}

if ($List) {
    $i = 0
    foreach ($w in (Get-VisibleWindows)) {
        $i++
        "{0,2}. [{1}] {2}" -f $i, $w.Handle, $w.Title
    }
    exit 0
}

$target = $null
$deadline = (Get-Date).AddSeconds($Timeout)

while ($true) {
    if ($Foreground) {
        $h = [WinCap]::GetForegroundWindow()
        if ($h -ne [IntPtr]::Zero) {
            $target = [pscustomobject]@{ Handle = $h; Title = [WinCap]::Title($h) }
            break
        }
    } else {
        $match = Get-VisibleWindows |
            Where-Object {
                $_.Title -match $TitleMatch -and
                ($Exclude -eq "" -or $_.Title -notmatch $Exclude)
            } |
            Select-Object -First 1
        if ($match) { $target = $match; break }
    }

    if ((Get-Date) -ge $deadline) { break }
    Start-Sleep -Milliseconds 500
}

# Subir la ventana del juego al frente: CopyFromScreen copia lo que se ve, y si
# otra ventana la tapa se captura esa otra (paso con el navegador).
if ($target -and $Bring) {
    [void][WinCap]::ShowWindow($target.Handle, 9)
    [void][WinCap]::SetWindowPos($target.Handle, [IntPtr](-1), 0, 0, 0, 0, 0x0043)
    [void][WinCap]::BringWindowToTop($target.Handle)
    [void][WinCap]::SetForegroundWindow($target.Handle)
    Start-Sleep -Milliseconds 900
}

$rect = New-Object WinCap+RECT

if ($target -and -not $FullScreen) {
    if ($WithFrame) {
        [void][WinCap]::GetWindowRect($target.Handle, [ref]$rect)
        $x = $rect.Left
        $y = $rect.Top
    } else {
        # Solo el area de cliente (sin bordes ni barra de titulo), para tener
        # exactamente el framebuffer del juego y poder comparar pixel a pixel.
        [void][WinCap]::GetClientRect($target.Handle, [ref]$rect)
        $pt = New-Object WinCap+POINT
        [void][WinCap]::ClientToScreen($target.Handle, [ref]$pt)
        $x = $pt.X
        $y = $pt.Y
    }
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    $label = $target.Title
} else {
    $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $x = $vs.Left
    $y = $vs.Top
    $w = $vs.Width
    $h = $vs.Height
    $label = "(pantalla completa)"
}

if ($w -lt 8 -or $h -lt 8) {
    throw "No hay ninguna ventana que capturar (tamano ${w}x${h}). Usa -List para ver los titulos disponibles."
}

$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size($w, $h)))
$g.Dispose()

$dir = Split-Path -Parent $Out
if ($dir) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

"OK  ventana='$label'  {0}x{1}  ->  $Out" -f $w, $h
