# Mide FPS de una version en una direccion de camara concreta (una sola ventana
# abierta a la vez, que es como se puede medir de verdad).
#   .\medir_dir.ps1 -Quien rs   -Dy 0      (horizonte; -590 cielo, +590 suelo)
#   .\medir_dir.ps1 -Quien orig -Dy -590
param(
    [Parameter(Mandatory = $true)][ValidateSet("rs", "orig")][string]$Quien,
    [int]$Dy = 0,
    [int]$Arranque = 6,
    [int]$Asentar = 5
)

$raiz = Split-Path -Parent $PSScriptRoot          # carpeta del proyecto
if (-not $raiz) { $raiz = (Get-Location).Path }
$mc = "$env:APPDATA\.minecraft"
$logOurs = Join-Path $raiz "input_log.txt"
$logOrig = Join-Path $raiz "analisis\mcmod\fps_orig.log"

# Cerrar lo que haya abierto (una version cada vez)
Get-Process -Name rubydung-rs -ErrorAction SilentlyContinue | Stop-Process -Force
Get-CimInstance Win32_Process -Filter "Name='java.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -like "*rubydung*" } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 800

if ($Quien -eq "rs") {
    # El port solo escribe la traza de entrada cuando se le pide.
    $env:RD_INPUT_LOG = "1"
    Remove-Item $logOurs -ErrorAction SilentlyContinue
    if ($env:RUBYDUNG_ESTRES -eq "1") {
        $env:RUBYDUNG_ESTRES = "1"
    }
    Start-Process -FilePath (Join-Path $raiz "target\x86_64-pc-windows-gnu\release\rubydung-rs.exe") `
        -WorkingDirectory $raiz | Out-Null
    $patron = "^RubyDung"
    $clic = $true
    $log = $logOurs
} else {
    Remove-Item $logOrig -ErrorAction SilentlyContinue
    # El classpath lleva espacios (ruta del proyecto), asi que se lanza desde un
    # .cmd con comillas y la consola redirigida a fps_orig.log.
    Start-Process -FilePath (Join-Path $PSScriptRoot "mcmod\medir_orig.cmd") -WindowStyle Minimized | Out-Null
    $patron = "^Game"
    $clic = $false
    $log = $logOrig
}

Start-Sleep -Seconds $Arranque

# Enfocar + girar
$args = @("-Title", $patron, "-dy", $Dy, "-veces", 1)
if ($clic) { $args += "-click" }
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "girar.ps1") @args | Out-Null

Start-Sleep -Seconds $Asentar

# Ultimas lecturas de FPS
if ($Quien -eq "rs") {
    $vals = @(Select-String -Path $log -Pattern "^\[\s*[\d.]+\] FPS (\d+)" -ErrorAction SilentlyContinue |
        ForEach-Object { [int]$_.Matches[0].Groups[1].Value })
} else {
    $vals = @(Select-String -Path $log -Pattern "(\d+) fps," -ErrorAction SilentlyContinue |
        ForEach-Object { [int]$_.Matches[0].Groups[1].Value })
}
$ult = @($vals | Select-Object -Last 4)

# Comprobacion de que la camara se movio (para no dar por bueno un caso mal medido)
$pos = ""
if ($Quien -eq "rs") {
    $pos = (Select-String -Path $log -Pattern "POS .*xRot=([-\d.]+)" -ErrorAction SilentlyContinue |
        Select-Object -Last 1).Matches[0].Groups[1].Value
} else {
    $pos = "(sin POS en el original)"
}

Write-Output ("RESULT {0} dy={1} pitch={2} fps={3}" -f $Quien, $Dy, $pos, ($ult -join ","))

Get-Process -Name rubydung-rs -ErrorAction SilentlyContinue | Stop-Process -Force
Get-CimInstance Win32_Process -Filter "Name='java.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -like "*rubydung*" } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
