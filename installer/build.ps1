# Compila el instalador de Rustium con Inno Setup.
#
#   powershell -ExecutionPolicy Bypass -File installer\build.ps1
#
# Antes de ejecutarlo, compila los binarios:
#   cargo +stable-x86_64-pc-windows-gnu build --profile pequeno --target x86_64-pc-windows-gnu
#
# El resultado queda en dist\Rustium-<version>-setup.exe

$ErrorActionPreference = "Stop"
$raiz = Split-Path -Parent $PSScriptRoot

$iscc = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $iscc) {
    Write-Host "Inno Setup 6 no esta instalado." -ForegroundColor Yellow
    Write-Host "Instalalo con:  winget install JRSoftware.InnoSetup"
    Write-Host "o desde https://jrsoftware.org/isdl.php"
    exit 1
}

foreach ($exe in @("rustium.exe", "rubydung-rs.exe")) {
    $p = Join-Path $raiz "target\x86_64-pc-windows-gnu\pequeno\$exe"
    if (-not (Test-Path $p)) {
        Write-Host "Falta $p. Compila antes con cargo." -ForegroundColor Red
        exit 1
    }
}

Write-Host "Compilando el instalador con $iscc"
& $iscc (Join-Path $PSScriptRoot "rustium.iss")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Get-ChildItem (Join-Path $raiz "dist") -Filter "Rustium-*-setup.exe" |
    Sort-Object LastWriteTime |
    Select-Object -Last 1 |
    ForEach-Object { Write-Host "Listo: $($_.FullName)" -ForegroundColor Green }
