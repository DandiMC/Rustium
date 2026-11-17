<#
Mide por fotograma la zona central (hueco de cielo), una banda de pasto y
cuantos pixeles llegan a blanco puro. Sirve para caracterizar la pulsacion
del resaltado del bloque apuntado.

Uso: powershell -NoProfile -ExecutionPolicy Bypass -File analisis/seleccion.ps1 analisis/capturas/orig_sel_%02d.png 0 20
#>
param(
    [string]$Pattern = "analisis/capturas/orig_sel_%02d.png",
    [int]$From = 0,
    [int]$To = 20
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

function Stats([string]$path) {
    $bmp = [System.Drawing.Bitmap]::FromFile($path)
    $W = $bmp.Width
    $H = $bmp.Height
    $r = New-Object System.Drawing.Rectangle(0, 0, $W, $H)
    $d = $bmp.LockBits($r, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $stride = [Math]::Abs($d.Stride)
    $bytes = New-Object byte[] ($stride * $H)
    [System.Runtime.InteropServices.Marshal]::Copy($d.Scan0, $bytes, 0, $bytes.Length)
    $bmp.UnlockBits($d)
    $bmp.Dispose()

    $regions = @(
        @{ Name = "cielo"; x0 = 0.42; x1 = 0.58; y0 = 0.42; y1 = 0.58 },
        @{ Name = "pasto"; x0 = 0.05; x1 = 0.25; y0 = 0.05; y1 = 0.25 }
    )

    $out = [ordered]@{}
    foreach ($reg in $regions) {
        $sr = 0; $sg = 0; $sb = 0; $n = 0
        for ($y = [int]($H * $reg.y0); $y -lt [int]($H * $reg.y1); $y += 2) {
            $row = $y * $stride
            for ($x = [int]($W * $reg.x0); $x -lt [int]($W * $reg.x1); $x += 2) {
                $i = $row + $x * 3
                $sr += $bytes[$i + 2]; $sg += $bytes[$i + 1]; $sb += $bytes[$i]
                $n++
            }
        }
        $out[$reg.Name] = "{0,3},{1,3},{2,3}" -f [int]($sr / $n), [int]($sg / $n), [int]($sb / $n)
    }

    $white = 0
    for ($y = 0; $y -lt $H; $y += 2) {
        $row = $y * $stride
        for ($x = 0; $x -lt $W; $x += 2) {
            $i = $row + $x * 3
            if ($bytes[$i] -ge 250 -and $bytes[$i + 1] -ge 250 -and $bytes[$i + 2] -ge 250) { $white++ }
        }
    }
    $out["blancos"] = $white
    return $out
}

"{0,-6} {1,-14} {2,-14} {3}" -f "frame", "cielo(R,G,B)", "pasto(R,G,B)", "px_blanco"
for ($i = $From; $i -le $To; $i++) {
    $p = $Pattern.Replace("%02d", $i.ToString("00"))
    if (-not (Test-Path $p)) { continue }
    $s = Stats $p
    "{0,-6} {1,-14} {2,-14} {3}" -f $i, $s["cielo"], $s["pasto"], $s["blancos"]
}
