<#
Mide colores de una captura para comparar el original con la version Rust.
Uso: powershell -NoProfile -ExecutionPolicy Bypass -File analisis/muestreo.ps1 analisis/capturas/orig_01.png
#>
param([Parameter(Mandatory = $true)][string]$Path)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$bmp = [System.Drawing.Bitmap]::FromFile((Resolve-Path $Path))
$W = $bmp.Width
$H = $bmp.Height
"imagen: $Path  ${W}x${H}"

function Hex([System.Drawing.Color]$c) { "#{0:X2}{1:X2}{2:X2}" -f $c.R, $c.G, $c.B }

"`n[columna central  x=$([int]($W / 2))]  (de arriba a abajo)"
foreach ($f in 0.01, 0.15, 0.30, 0.42, 0.48, 0.52, 0.60, 0.75, 0.90, 0.99) {
    $y = [int]($H * $f)
    $c = $bmp.GetPixel([int]($W / 2), $y)
    "  y={0,4} ({1,4:P0})  {2}" -f $y, $f, (Hex $c)
}

"`n[columna izquierda x=$([int]($W * 0.12))]"
foreach ($f in 0.01, 0.15, 0.30, 0.42, 0.48, 0.52, 0.60, 0.75, 0.90, 0.99) {
    $y = [int]($H * $f)
    $c = $bmp.GetPixel([int]($W * 0.12), $y)
    "  y={0,4} ({1,4:P0})  {2}" -f $y, $f, (Hex $c)
}

"`n[fila horizontal y=$([int]($H / 2))]"
foreach ($f in 0.02, 0.20, 0.40, 0.60, 0.80, 0.98) {
    $x = [int]($W * $f)
    $c = $bmp.GetPixel($x, [int]($H / 2))
    "  x={0,4}  {1}" -f $x, (Hex $c)
}

# Color mas repetido (detecta el atlas magenta/negro de respaldo y el aplanado de texturas)
$counts = @{}
for ($y = 0; $y -lt $H; $y += 3) {
    for ($x = 0; $x -lt $W; $x += 3) {
        $k = Hex $bmp.GetPixel($x, $y)
        if ($counts.ContainsKey($k)) { $counts[$k]++ } else { $counts[$k] = 1 }
    }
}
"`n[colores mas frecuentes]"
$counts.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 6 |
    ForEach-Object { "  {0}  x{1}" -f $_.Key, $_.Value }

$bmp.Dispose()
